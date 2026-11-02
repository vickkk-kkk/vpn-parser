use serde_json::{json, Value};
use vp_client_core::{
    compilation::{
        self,
        compiler::{get_backend_compiler, BackendCompiler, DiagnosticSeverity},
        options::{MihomoConfigOptions, ProxyMode},
        CompilationContext, GeoAssetProvider, KernelKind,
    },
    geo::{CategoryLookup, GeoKind, GeoProvider, GeoUnavailable},
    models::Node,
};
struct Inventory;
impl GeoProvider for Inventory {
    fn lookup(&self, _: GeoKind, _: &str) -> CategoryLookup {
        CategoryLookup::Present
    }
}
impl GeoAssetProvider for Inventory {
    fn asset_path(&self, kind: GeoKind) -> Option<&str> {
        Some(match kind {
            GeoKind::Site => "assets/geosite.dat",
            GeoKind::Ip => "assets/geoip.dat",
        })
    }
    fn lookup(&self, _: GeoKind, _: &str) -> CategoryLookup {
        CategoryLookup::Present
    }
}
fn context(inventory: &Inventory) -> CompilationContext<'_> {
    CompilationContext {
        geo: inventory,
        shoes_geo: inventory,
        connectivity_test_url: "https://probe.example.invalid/generate_204",
        mirror_direct_hosts: &["mirror.example.invalid"],
    }
}
fn node(protocol: &str, config: Value) -> Node {
    serde_json::from_value(json!({"id":"fixture", "source_id":null,"protocol":protocol,"name":"fixture-node","server":"proxy.example.invalid","port":443,"raw":"","config":config,"country":null,"latency_ms":null,"latency_ok":null,"download_kbps":null,"latency_kernel":null,"latency_test_kind":null,"latency_tested_at":null,"created_at":"2026-01-01T00:00:00Z","updated_at":"2026-01-01T00:00:00Z"})).unwrap()
}
#[test]
fn six_backends_share_the_pipeline_and_produce_deterministic_artifacts() {
    let inventory = Inventory;
    let context = context(&inventory);
    for kernel in [
        KernelKind::Mihomo,
        KernelKind::SingBox,
        KernelKind::Xray,
        KernelKind::Shoes,
        KernelKind::Meow,
        KernelKind::VpCore,
    ] {
        let n = if kernel == KernelKind::SingBox {
            node(
                "hysteria2",
                json!({"password":"test-only", "sni":"proxy.example.invalid"}),
            )
        } else {
            node(
                "trojan",
                json!({"password":"test-only","sni":"proxy.example.invalid","tls":true}),
            )
        };
        let options = MihomoConfigOptions {
            routing_mode: vp_client_core::config::RoutingMode::Direct,
            ..Default::default()
        };
        let result =
            compilation::compile_configuration(kernel, &options, Some(&n), &context).unwrap();
        let again =
            compilation::compile_configuration(kernel, &options, Some(&n), &context).unwrap();
        assert_eq!(
            result.artifact.to_string_pretty().unwrap(),
            again.artifact.to_string_pretty().unwrap()
        );
        let mut diagnostics = vec![];
        let plan =
            compilation::ir::lower_options_to_ir(&options, Some(&n), &context, &mut diagnostics);
        let legacy = get_backend_compiler(kernel)
            .compile(&plan, &context, &mut diagnostics)
            .unwrap();
        assert_eq!(
            result.artifact.as_json_value().unwrap(),
            legacy.as_json_value().unwrap()
        );
    }
}
#[test]
fn checked_api_rejects_both_mode_on_xray_without_leaking_input() {
    let inventory = Inventory;
    let context = context(&inventory);
    let n = node("trojan", json!({"password":"sensitive-test-sentinel"}));
    let options = MihomoConfigOptions {
        mode: ProxyMode::Both,
        ..Default::default()
    };
    let failure =
        compilation::compile_configuration(KernelKind::Xray, &options, Some(&n), &context)
            .unwrap_err();
    assert!(failure
        .diagnostics
        .iter()
        .any(|d| d.severity == DiagnosticSeverity::Error));
    assert!(!format!("{failure:?}").contains("sensitive-test-sentinel"));
}
#[test]
fn unsupported_protocol_is_an_explicit_error_not_a_fallback() {
    let inventory = Inventory;
    let context = context(&inventory);
    let n = node(
        "ssh-secret-sentinel",
        json!({"password":"sensitive-test-sentinel"}),
    );
    let failure = compilation::compile_configuration(
        KernelKind::SingBox,
        &Default::default(),
        Some(&n),
        &context,
    )
    .unwrap_err();
    let printed = format!("{failure:?}");
    assert!(!printed.contains("secret-sentinel"));
    assert!(!printed.contains("sensitive-test-sentinel"));
}
#[test]
fn clash_export_preserves_nested_unknown_fields_and_does_not_mutate_input() {
    let n = node(
        "socks5",
        json!({"unknown-extension":{"nested":[1,"two",true]}}),
    );
    let before = n.config.clone();
    let exported = vp_client_core::export::nodes_to_clash_proxies(std::slice::from_ref(&n));
    assert_eq!(
        exported[0]["unknown-extension"],
        before["unknown-extension"]
    );
    assert_eq!(n.config, before);
}
#[test]
fn shoes_asset_inventory_does_not_apply_two_letter_country_shortcut() {
    struct AbsentAssets;
    impl GeoAssetProvider for AbsentAssets {
        fn asset_path(&self, _: GeoKind) -> Option<&str> {
            Some("opaque/mobile/asset.dat")
        }
        fn lookup(&self, _: GeoKind, _: &str) -> CategoryLookup {
            CategoryLookup::Absent
        }
    }
    let geo = vp_client_core::geo::BytesGeoProvider {
        site: Err(GeoUnavailable::Missing),
        ip: Err(GeoUnavailable::Missing),
    };
    assert_eq!(geo.lookup(GeoKind::Ip, "CN"), CategoryLookup::Present);
    let assets = AbsentAssets;
    let context = CompilationContext {
        geo: &geo,
        shoes_geo: &assets,
        connectivity_test_url: "https://probe.example.invalid",
        mirror_direct_hosts: &[],
    };
    let n = node("trojan", json!({"password":"test-only"}));
    let options = MihomoConfigOptions::default();
    let mut diagnostics = vec![];
    let mut plan =
        compilation::ir::lower_options_to_ir(&options, Some(&n), &context, &mut diagnostics);
    plan.rules = vec![compilation::ir::IRRule::new(
        compilation::ir::IRMatcher::GeoIp("CN".into()),
        compilation::ir::IRAction::Direct,
        compilation::ir::RuleSource::User,
    )];
    let output = get_backend_compiler(KernelKind::Shoes)
        .compile(&plan, &context, &mut diagnostics)
        .unwrap()
        .to_string_pretty()
        .unwrap();
    assert!(!output.contains("opaque/mobile/asset.dat"));
    assert!(!diagnostics.is_empty());
}

#[test]
fn matches_pre_extraction_six_engine_baselines() {
    struct MissingAssets;
    impl GeoAssetProvider for MissingAssets {
        fn asset_path(&self, _: GeoKind) -> Option<&str> {
            None
        }
        fn lookup(&self, _: GeoKind, _: &str) -> CategoryLookup {
            CategoryLookup::Unavailable(GeoUnavailable::Missing)
        }
    }
    let geo = vp_client_core::geo::BytesGeoProvider {
        site: Err(GeoUnavailable::Missing),
        ip: Err(GeoUnavailable::Missing),
    };
    let assets = MissingAssets;
    let context = CompilationContext {
        geo: &geo,
        shoes_geo: &assets,
        connectivity_test_url: "https://www.google.com/generate_204",
        mirror_direct_hosts: &[
            "fastly.jsdelivr.net",
            "cdn.jsdelivr.net",
            "gh-proxy.com",
            "ghfast.top",
        ],
    };
    for kernel in [
        KernelKind::Mihomo,
        KernelKind::SingBox,
        KernelKind::Xray,
        KernelKind::Shoes,
        KernelKind::Meow,
        KernelKind::VpCore,
    ] {
        let protocol = if kernel == KernelKind::SingBox {
            "hysteria2"
        } else {
            "trojan"
        };
        let n = node(
            protocol,
            json!({"password":"synthetic-test-only","sni":"proxy.example.invalid","tls":true}),
        );
        let options = MihomoConfigOptions::default();
        let mut diagnostics = Vec::new();
        let plan =
            compilation::ir::lower_options_to_ir(&options, Some(&n), &context, &mut diagnostics);
        let actual = get_backend_compiler(kernel)
            .compile(&plan, &context, &mut diagnostics)
            .unwrap()
            .to_string_pretty()
            .unwrap();
        let expected = match kernel {
            KernelKind::Mihomo => include_str!("fixtures/compiler-baseline/mihomo.txt"),
            KernelKind::SingBox => include_str!("fixtures/compiler-baseline/sing_box.txt"),
            KernelKind::Xray => include_str!("fixtures/compiler-baseline/xray.txt"),
            KernelKind::Shoes => include_str!("fixtures/compiler-baseline/shoes.txt"),
            KernelKind::Meow => include_str!("fixtures/compiler-baseline/meow.txt"),
            KernelKind::VpCore => include_str!("fixtures/compiler-baseline/vpcore.txt"),
        };
        assert!(
            actual == expected,
            "serialized artifact changed for {}",
            kernel.as_str()
        );
    }
}

#[test]
fn checked_api_reports_omitted_candidates_without_input_names() {
    let inventory = Inventory;
    let context = context(&inventory);
    let n = node("trojan", json!({"password":"test-only"}));
    let options = MihomoConfigOptions {
        url_test: true,
        url_test_nodes: vec![
            json!("private-candidate-sentinel"),
            json!({"type":"trojan","password":"private-candidate-sentinel"}),
        ],
        ..Default::default()
    };
    let failure =
        compilation::compile_configuration(KernelKind::Mihomo, &options, Some(&n), &context)
            .unwrap_err();
    assert!(failure
        .diagnostics
        .iter()
        .any(|d| d.severity == DiagnosticSeverity::Error));
    assert!(!format!("{failure:?}").contains("private-candidate-sentinel"));
}

#[test]
fn checked_sing_box_rejects_undefined_geosite_rule_sets() {
    use vp_client_core::config::{RuleAction, RuleMatcher, TrafficRule};
    let inventory = Inventory;
    let context = context(&inventory);
    let n = node("hysteria2", json!({"password":"test-only"}));
    let mut options = MihomoConfigOptions::default();
    let rule = TrafficRule {
        matcher: RuleMatcher::GeoSite("private-category-sentinel".into()),
        action: RuleAction::Direct,
    };
    options.rules.push(rule);
    let failure =
        compilation::compile_configuration(KernelKind::SingBox, &options, Some(&n), &context)
            .unwrap_err();
    assert!(failure
        .diagnostics
        .iter()
        .any(|d| d.severity == DiagnosticSeverity::Error));
    assert!(!format!("{failure:?}").contains("private-category-sentinel"));
}

#[test]
fn checked_sing_box_rejects_undefined_direct_rule_set_references() {
    use vp_client_core::config::{RuleAction, RuleMatcher, TrafficRule};
    let inventory = Inventory;
    let context = context(&inventory);
    let n = node("hysteria2", json!({"password":"test-only"}));
    let mut options = MihomoConfigOptions::default();
    options.rules.push(TrafficRule {
        matcher: RuleMatcher::RuleSet("private-ruleset-sentinel".into()),
        action: RuleAction::Direct,
    });
    let failure =
        compilation::compile_configuration(KernelKind::SingBox, &options, Some(&n), &context)
            .unwrap_err();
    assert!(failure
        .diagnostics
        .iter()
        .any(|d| d.severity == DiagnosticSeverity::Error));
    assert!(!format!("{failure:?}").contains("private-ruleset-sentinel"));
}
