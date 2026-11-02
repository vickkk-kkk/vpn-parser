use super::*;
use vp_client_core::{
    config::{RuleAction, RuleMatcher},
    parser::{parse_uri, NodeMetadata},
};
fn node(engine: KernelKind, id: &str) -> Node {
    let uri = if engine == KernelKind::SingBox {
        "hy2://private-preview-sentinel@192.0.2.1:443?sni=proxy.example.invalid#private-name-sentinel"
    } else {
        "trojan://private-preview-sentinel@192.0.2.1:443?sni=proxy.example.invalid#private-name-sentinel"
    };
    parse_uri(uri, None, &mut || NodeMetadata {
        id: id.into(),
        timestamp: "2026-01-01T00:00:00Z".parse().unwrap(),
    })
    .unwrap()
}
fn compile(model: &mut ConfigPreview, nodes: &[Node], mode: RoutingMode, rules: &[TrafficRule]) {
    let dns = DnsConfig::default();
    model.compile(nodes, &dns, &dns.nameservers.join("\n"), mode, rules, &[]);
}
#[test]
fn six_explicit_engines_compile_supported_nodes_without_probe_groups() {
    for engine in ENGINES {
        let nodes = vec![node(engine, "selected")];
        let mut model = ConfigPreview::default();
        model.select_engine(engine);
        model.select_node(Some("selected".into()));
        compile(&mut model, &nodes, RoutingMode::Global, &[]);
        assert!(
            model.has_artifact(),
            "supported engine fixture did not compile"
        );
        assert_eq!(model.engine(), Some(engine));
        assert!(model.revealed().is_none());
        let summary = model.summary();
        assert!(
            !summary.contains("private-preview-sentinel")
                && !summary.contains("private-name-sentinel")
        );
        model.reveal();
        let text = model.revealed().unwrap();
        assert!(
            !text.contains("url-test"),
            "disabled probe group was emitted"
        );
        assert!(!text.contains("generate_204"), "probe URL was emitted");
        assert!(
            !text.contains("\"url\": \"\"") && !text.contains("url: ''"),
            "empty probe URL emitted"
        );
        model.hide();
        assert!(model.revealed().is_none());
    }
}
#[test]
fn safe_options_are_explicit_and_preferences_never_auto() {
    for engine in ENGINES {
        let opts = preview_options(engine, DnsConfig::default(), RoutingMode::Rule, vec![]);
        assert_eq!(opts.mode, ProxyMode::None);
        assert!(!opts.mode.has_system() && !opts.mode.has_tun());
        assert!(!opts.allow_lan && !opts.terminal_proxy && !opts.url_test && !opts.smart_routing);
        assert!(!opts.tun.auto_route && !opts.tun.auto_detect_interface);
        assert!(opts.tun.dns_hijack.is_empty() && opts.port_mappings.is_empty());
        assert!(opts.override_yaml.is_none() && opts.subscription_groups_yaml.is_none());
        assert!(opts.controller_secret.is_empty());
        assert_eq!(opts.kernel_preference.kernel(), Some(engine));
    }
}
#[test]
fn required_selection_has_no_implicit_engine_or_node() {
    let nodes = vec![node(KernelKind::Mihomo, "selected")];
    let mut model = ConfigPreview::default();
    compile(&mut model, &nodes, RoutingMode::Global, &[]);
    assert!(!model.has_artifact());
    model.select_engine(KernelKind::Mihomo);
    compile(&mut model, &nodes, RoutingMode::Global, &[]);
    assert!(!model.has_artifact());
}
#[test]
fn current_invalid_dns_draft_does_not_fall_back_to_saved_dns() {
    let mut model = ConfigPreview::default();
    model.select_engine(KernelKind::Mihomo);
    model.select_node(Some("selected".into()));
    model.compile(
        &[node(KernelKind::Mihomo, "selected")],
        &DnsConfig::default(),
        "not a resolver",
        RoutingMode::Global,
        &[],
        &[],
    );
    assert!(!model.has_artifact());
    assert!(model.summary().contains("DNS draft is invalid"));
}
#[test]
fn node_identity_survives_reorder_and_missing_node_rejects() {
    let mut a = node(KernelKind::Mihomo, "a");
    a.server = "192.0.2.10".into();
    let mut b = node(KernelKind::Mihomo, "b");
    b.server = "192.0.2.20".into();
    let mut model = ConfigPreview::default();
    model.select_engine(KernelKind::Mihomo);
    model.select_node(Some("a".into()));
    compile(&mut model, &[b, a], RoutingMode::Global, &[]);
    assert!(model.has_artifact());
    assert_eq!(model.node_id(), Some("a"));
    compile(&mut model, &[], RoutingMode::Global, &[]);
    assert!(!model.has_artifact() && model.revealed().is_none());
}
#[test]
fn shoes_rule_mode_rejection_preserves_requested_engine_and_mode() {
    let mut model = ConfigPreview::default();
    model.select_engine(KernelKind::Shoes);
    model.select_node(Some("selected".into()));
    compile(
        &mut model,
        &[node(KernelKind::Shoes, "selected")],
        RoutingMode::Rule,
        &[],
    );
    assert!(!model.has_artifact());
    assert_eq!(model.engine(), Some(KernelKind::Shoes));
    assert!(model
        .diagnostics()
        .iter()
        .any(|d| d.message.contains("protocol-specific")));
}
#[test]
fn unsupported_protocol_does_not_fall_back() {
    let mut n = node(KernelKind::Xray, "selected");
    n.protocol = "unsupported-preview-protocol".into();
    n.config["type"] = "unsupported-preview-protocol".into();
    let mut model = ConfigPreview::default();
    model.select_engine(KernelKind::Xray);
    model.select_node(Some("selected".into()));
    compile(&mut model, &[n], RoutingMode::Global, &[]);
    assert!(!model.has_artifact());
    assert_eq!(model.engine(), Some(KernelKind::Xray));
    for d in model.diagnostics() {
        assert!(!d.message.contains("private-preview-sentinel"));
    }
}
#[test]
fn global_and_direct_explicitly_report_inactive_rules() {
    for mode in [RoutingMode::Global, RoutingMode::Direct] {
        let mut model = ConfigPreview::default();
        model.select_engine(KernelKind::Mihomo);
        model.select_node(Some("selected".into()));
        let rule = TrafficRule {
            matcher: RuleMatcher::Domain("example.invalid".into()),
            action: RuleAction::Direct,
        };
        compile(
            &mut model,
            &[node(KernelKind::Mihomo, "selected")],
            mode,
            &[rule],
        );
        assert!(model.inactive_rules());
    }
}
#[test]
fn input_and_selection_changes_drop_artifact_and_revealed_secret() {
    let mut model = ConfigPreview::default();
    model.select_engine(KernelKind::Mihomo);
    model.select_node(Some("selected".into()));
    let nodes = vec![node(KernelKind::Mihomo, "selected")];
    compile(&mut model, &nodes, RoutingMode::Global, &[]);
    model.reveal();
    assert!(model.revealed().is_some());
    model.invalidate();
    assert!(!model.has_artifact() && model.revealed().is_none());
    compile(&mut model, &nodes, RoutingMode::Global, &[]);
    model.reveal();
    model.select_engine(KernelKind::Xray);
    assert!(!model.has_artifact() && model.revealed().is_none());
    compile(&mut model, &nodes, RoutingMode::Global, &[]);
    model.reveal();
    model.select_node(None);
    assert!(!model.has_artifact() && model.revealed().is_none());
}
#[test]
fn missing_geo_is_not_empty_inventory_or_country_shortcut() {
    let geo = MissingGeo;
    assert_eq!(
        GeoProvider::lookup(&geo, GeoKind::Ip, "CN"),
        CategoryLookup::Unavailable(GeoUnavailable::Missing)
    );
    assert_eq!(
        GeoAssetProvider::lookup(&geo, GeoKind::Ip, "CN"),
        CategoryLookup::Unavailable(GeoUnavailable::Missing)
    );
    assert!(geo.asset_path(GeoKind::Ip).is_none());
    assert_eq!(
        vp_client_core::geo::dat_category_lookup(&[], "CN"),
        CategoryLookup::Absent
    );
}

#[test]
fn rule_merge_keeps_manual_precedence_and_removes_only_exact_duplicates() {
    let direct = TrafficRule {
        matcher: RuleMatcher::Domain("example.invalid".into()),
        action: RuleAction::Direct,
    };
    let proxy = TrafficRule {
        matcher: direct.matcher.clone(),
        action: RuleAction::Proxy,
    };
    let rules = combined_rules(
        &[direct.clone(), direct.clone()],
        &[proxy.clone(), direct.clone()],
    );
    assert_eq!(rules, vec![direct, proxy]);
}
#[test]
fn explicit_unsupported_process_rule_is_an_error_not_silently_omitted() {
    let mut model = ConfigPreview::default();
    model.select_engine(KernelKind::Xray);
    model.select_node(Some("selected".into()));
    let rule = TrafficRule {
        matcher: RuleMatcher::ProcessName("synthetic-app".into()),
        action: RuleAction::Direct,
    };
    compile(
        &mut model,
        &[node(KernelKind::Xray, "selected")],
        RoutingMode::Subscription,
        &[rule],
    );
    assert!(!model.has_artifact());
    assert!(model
        .diagnostics()
        .iter()
        .any(|d| d.severity == vp_client_core::compilation::compiler::DiagnosticSeverity::Error));
}
