//! Exercise the actual desktop facade without linking or starting the desktop binary.
#[path = "../../../src/proxy/config.rs"]
mod desktop_config;

use serde_json::json;
use vp_client_core::config::*;

#[test]
fn desktop_facade_preserves_type_identity_and_helpers() {
    let shared = TrafficRule {
        matcher: RuleMatcher::DomainSuffix("example.com".into()),
        action: RuleAction::Proxy,
    };
    // This assignment fails to compile if the facade introduces duplicate model types.
    let desktop: desktop_config::TrafficRule = shared.clone();
    let roundtrip: TrafficRule = desktop;
    assert_eq!(roundtrip, shared);
    assert_eq!(desktop_config::recommended_traffic_rules(), recommended_traffic_rules());
    assert_eq!(desktop_config::DnsConfig::default(), DnsConfig::default());
}

#[test]
fn serialized_rule_and_enum_names_remain_stable() {
    let expected = json!({
        "matcher": {"type": "domain_suffix", "value": "example.com"},
        "action": "proxy"
    });
    let rule: TrafficRule = serde_json::from_value(expected.clone()).unwrap();
    assert_eq!(serde_json::to_value(&rule).unwrap(), expected);
    assert_eq!(rule.to_mihomo_rule().unwrap(), "DOMAIN-SUFFIX,example.com,PROXY");
    assert_eq!(serde_json::to_value(RoutingMode::ChinaDirect).unwrap(), "china_direct");
    assert_eq!(serde_json::to_value(DnsEnhancedMode::FakeIp).unwrap(), "fake_ip");
    assert_eq!(serde_json::to_value(TunStack::Mixed).unwrap(), "mixed");
}

#[test]
fn old_dns_json_restores_missing_field_without_changing_custom_resolver() {
    let mut config: DnsConfig = serde_json::from_value(json!({
        "enabled": true,
        "ipv6": false,
        "enhanced_mode": "fake_ip",
        "nameservers": ["https://resolver.example/dns-query"]
    })).unwrap();
    assert_eq!(config.proxy_server_nameserver, DnsConfig::default().proxy_server_nameserver);
    assert!(!config.migrate_legacy_defaults());
    assert_eq!(config.nameservers, ["https://resolver.example/dns-query"]);
    assert!(config.validate().is_ok());
    let restored: DnsConfig = serde_json::from_value(serde_json::to_value(&config).unwrap()).unwrap();
    assert_eq!(restored, config);
}

#[test]
fn legacy_dns_migration_is_idempotent() {
    let mut config = DnsConfig {
        nameservers: vec!["8.8.8.8".into(), "114.114.114.114".into()],
        proxy_server_nameserver: vec![],
        ..DnsConfig::default()
    };
    assert!(config.migrate_legacy_defaults());
    let migrated = config.clone();
    assert!(!config.migrate_legacy_defaults());
    assert_eq!(config, migrated);
}

#[test]
fn rule_values_cannot_inject_extra_rules_or_targets() {
    for value in ["", "   ", "example.com,DIRECT", "example.com\nMATCH,DIRECT", "example.com\rMATCH,DIRECT"] {
        let rule = TrafficRule {
            matcher: RuleMatcher::Domain(value.into()),
            action: RuleAction::Proxy,
        };
        assert!(rule.to_mihomo_rule().is_err(), "accepted {value:?}");
    }
}

#[test]
fn large_rule_dedup_preserves_order_and_distinct_actions() {
    let first = TrafficRule {
        matcher: RuleMatcher::Domain("example.com".into()),
        action: RuleAction::Direct,
    };
    let second = TrafficRule { action: RuleAction::Proxy, ..first.clone() };
    let mut rules = vec![first.clone(); 601];
    rules.push(second.clone());
    assert_eq!(optimize_traffic_rules(&rules), vec![first, second]);
}
