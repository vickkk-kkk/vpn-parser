//! Shared parsing helpers and DTOs; no host I/O, clocks, engines or subscriptions.
#[path = "../../../src/models/node.rs"]
mod desktop_models;
#[path = "../../../src/parser/common.rs"]
mod desktop_parser;

use base64::{engine::general_purpose, Engine as _};
use serde_json::json;
use vp_client_core::{models::Node, parser::*};

#[test]
fn node_json_roundtrip_preserves_all_fields_and_protocol_specific_config() {
    let expected = json!({
        "id": "fixture-node", "source_id": "fixture-source", "protocol": "vless",
        "name": "Fixture", "server": "proxy.example", "port": 443,
        "raw": "vless://fixture@proxy.example:443",
        "config": {"transport": {"type": "grpc", "serviceName": "sample"},
                   "future_extension": {"nested": [1, true, "retained"]}},
        "country": "ZZ", "latency_ms": 42, "latency_ok": true,
        "download_kbps": 123.5, "latency_kernel": "fixture",
        "latency_test_kind": "fixture", "latency_tested_at": "2026-01-01T00:00:00Z",
        "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-02T00:00:00Z"
    });
    let shared: Node = serde_json::from_value(expected.clone()).unwrap();
    let desktop: desktop_models::Node = shared;
    assert_eq!(serde_json::to_value(desktop).unwrap(), expected);
}

#[test]
fn summary_facade_types_preserve_identity_and_null_optionals() {
    let expected = json!({"id":"n", "source_id":null, "protocol":"future-protocol",
        "name":"n", "server":"proxy.example", "port":443, "country":null,
        "latency_ms":null, "latency_ok":null, "download_kbps":null,
        "latency_kernel":null, "latency_test_kind":null, "latency_tested_at":null,
        "updated_at":"2026-01-01T00:00:00Z"});
    let shared: vp_client_core::models::NodeSummary = serde_json::from_value(expected.clone()).unwrap();
    let desktop: desktop_models::NodeSummary = shared;
    assert_eq!(serde_json::to_value(desktop).unwrap(), expected);
    let group: vp_client_core::models::NodeGroupSummary = serde_json::from_value(
        json!({"source_id":null,"source_name":"Local","total":2,"alive":1})).unwrap();
    let desktop_group: desktop_models::NodeGroupSummary = group;
    assert_eq!(desktop_group.total, 2);
}

#[test]
fn base64_variants_preserve_subscription_text() {
    let input = "vless://fixture@proxy.example:443#Fixture";
    for engine in [&general_purpose::STANDARD, &general_purpose::STANDARD_NO_PAD,
                   &general_purpose::URL_SAFE, &general_purpose::URL_SAFE_NO_PAD] {
        let encoded = engine.encode(input);
        assert_eq!(decode_maybe_base64(&encoded).as_deref(), Some(input));
        assert_eq!(desktop_parser::decode_base64_str(&encoded).as_deref(), Some(input));
    }
    assert_eq!(decode_maybe_base64("too short"), None);
    assert_eq!(decode_base64_str("%%%"), None);
}

#[test]
fn normalizes_entities_wrappers_and_line_endings() {
    assert_eq!(normalize_subscription_text("a&amp;b\r\nc\rd"), "a&b\nc\nd");
    assert_eq!(desktop_parser::normalize_uri(" <vless://id@proxy.example:443?a=1&amp;b=2> "),
        "vless://id@proxy.example:443?a=1&b=2");
}

#[test]
fn extraction_keeps_protocol_vocabulary_order_and_unique_entries() {
    let schemes = ["ss", "ssr", "shadowsocksr", "vmess", "vless", "trojan", "tuic",
        "hysteria", "hysteria2", "hy2", "wireguard", "wg", "socks5", "socks", "masque"];
    let expected: Vec<String> = schemes.iter().map(|s| format!("{s}://fixture@proxy.example:443")).collect();
    let content = format!("{}\n{}", expected.join("\n"), expected[0]);
    assert_eq!(extract_uris(&content), expected);
    assert_eq!(desktop_parser::extract_uris(&content), expected);
}

#[test]
fn authority_helpers_keep_ipv6_percent_decoding_and_query_semantics() {
    let parsed = parse_authority("VLESS://user%40name@[2001:db8::1]:443?security=tls#Hello%20World").unwrap();
    assert_eq!(parsed, ("vless".into(), "user@name".into(), 443,
        "2001:db8::1".into(), "security=tls".into(), "Hello World".into()));
    let query = parse_query("key=first&key=last%20value&bare&=ignored");
    assert_eq!(query["key"], "last value");
    assert_eq!(query["bare"], "");
    assert!(!query.contains_key(""));
    assert!(parse_authority("vless://user@proxy.example:99999").is_none());
    assert!(split_host_port("[2001:db8::1]443").is_none());
}
