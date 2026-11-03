//! Deterministic tests of the original full parser with explicit host inputs.
use base64::Engine;
use vp_client_core::{geo::*, parser::*};

fn metadata() -> impl NodeMetadataProvider {
    let mut n = 0;
    move || {
        n += 1;
        NodeMetadata {
            id: format!("node-{n}"),
            timestamp: chrono::DateTime::from_timestamp(1_700_000_000 + n, 0).unwrap(),
        }
    }
}

#[test]
fn anytls_uri_import_preserves_credentials_and_verified_tls() {
    let nodes = parse_subscription("anytls://p%40ss@proxy.example?sni=tls.example&insecure=0#My%20node", None, &mut metadata());
    assert_eq!(nodes.len(), 1);
    let n = &nodes[0];
    assert_eq!(n.protocol, "anytls");
    assert_eq!(n.port, 443);
    assert_eq!(n.name, "My node");
    assert_eq!(n.config["password"], "p@ss");
    assert_eq!(n.config["sni"], "tls.example");
    assert_eq!(n.config["insecure"], false);
}

#[test]
fn anytls_uri_rejects_unsafe_or_unrecognized_options() {
    for uri in ["anytls://p@proxy.example?insecure=1", "anytls://p@proxy.example?unknown=x",
        "anytls://p@proxy.example?sni=a&sni=b", "anytls://p%ZZ@proxy.example",
        "anytls://p%00@proxy.example", "anytls://p@proxy.example:0", "anytls://@proxy.example"] {
        assert!(parse_uri(uri, None, &mut metadata()).is_none(), "unexpectedly accepted: {uri}");
    }
}
fn dat(categories: &[&str]) -> Vec<u8> {
    categories
        .iter()
        .flat_map(|code| {
            let mut v = vec![0x0a, (code.len() + 2) as u8, 0x0a, code.len() as u8];
            v.extend(code.as_bytes());
            v
        })
        .collect()
}

#[test]
fn metadata_is_requested_per_constructed_node_before_dedup() {
    let mut calls = 0;
    let mut provider = || {
        calls += 1;
        NodeMetadata {
            id: format!("id-{calls}"),
            timestamp: chrono::DateTime::from_timestamp(1_700_000_000 + calls, 0).unwrap(),
        }
    };
    let nodes = parse_subscription(
        "vless://fixture@proxy.example:443#A\nvless://fixture@proxy.example:443#B",
        Some("source"),
        &mut provider,
    );
    assert_eq!(calls, 2);
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].id, "id-2");
    assert_eq!(nodes[0].source_id.as_deref(), Some("source"));
    assert_eq!(nodes[0].created_at, nodes[0].updated_at);
}

#[test]
fn unknown_clash_protocol_and_nested_config_survive_plain_and_base64() {
    let yaml = "proxies:\n  - {type: future-protocol, name: Fixture, server: proxy.example, port: 443, future: {nested: [true, 9]}}";
    let encoded = base64::engine::general_purpose::STANDARD.encode(yaml);
    for body in [yaml, encoded.as_str()] {
        let nodes = parse_subscription(body, None, &mut metadata());
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].protocol, "future-protocol");
        assert_eq!(
            nodes[0].config["future"]["nested"],
            serde_json::json!([true, 9])
        );
        assert!(nodes[0].raw.contains("future"));
    }
}

#[test]
fn geo_absence_filters_but_unavailable_retains_with_diagnostics() {
    let yaml = "rules:\n - GEOSITE,private,DIRECT\n - GEOSITE,missing,PROXY\n - GEOIP,LAN,DIRECT\n - GEOIP,CN,DIRECT\n";
    let bytes = dat(&["private"]);
    let available = BytesGeoProvider {
        site: Ok(&bytes),
        ip: Ok(&bytes),
    };
    let mut diagnostics = Vec::new();
    let rules = extract_clash_rules(yaml, &available, &mut diagnostics);
    assert_eq!(rules.len(), 3);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_index, 1);
    assert_eq!(diagnostics[0].lookup, CategoryLookup::Absent);
    let missing = BytesGeoProvider {
        site: Err(GeoUnavailable::Missing),
        ip: Err(GeoUnavailable::Unreadable),
    };
    diagnostics.clear();
    let rules = extract_clash_rules(yaml, &missing, &mut diagnostics);
    assert_eq!(rules.len(), 4);
    assert_eq!(diagnostics.len(), 3); // GEOIP,CN keeps its original bypass.
    assert!(diagnostics
        .iter()
        .all(|d| matches!(d.lookup, CategoryLookup::Unavailable(_))));
    diagnostics.clear();
    let raw = extract_clash_rules_yaml(yaml, &available, &mut diagnostics).unwrap();
    assert!(!raw.contains("GEOSITE,missing"));
    assert!(raw.contains("GEOIP,private,DIRECT"));
    let corrupt = BytesGeoProvider {
        site: Ok(&[0x08]),
        ip: Ok(&[0x08]),
    };
    diagnostics.clear();
    assert_eq!(
        extract_clash_rules(yaml, &corrupt, &mut diagnostics).len(),
        4
    );
    assert!(diagnostics
        .iter()
        .all(|d| d.lookup == CategoryLookup::Unavailable(GeoUnavailable::Malformed)));
}

#[test]
fn masque_json_requires_no_entropy_and_keeps_all_57_endpoints() {
    let input = r#"{"private_key":"fixture-private","endpoint_pub_key":"fixture-public","ipv4":"172.16.0.2"}"#;
    let nodes = parse_subscription(input, Some("fixture"), &mut metadata());
    assert_eq!(nodes.len(), 57);
    assert!(nodes
        .iter()
        .all(|n| n.protocol == "masque" && n.config["private-key"] == "fixture-private"));
    let ids: std::collections::HashSet<_> = nodes.iter().map(|n| &n.id).collect();
    assert_eq!(ids.len(), 57);
}

#[test]
fn plain_base64_and_embedded_lists_share_the_full_dispatcher() {
    let vmess = base64::engine::general_purpose::STANDARD.encode(
        r#"{"v":"2","ps":"VMess","add":"vmess.example","port":"443","id":"aaaaaaaa-bbbb-4ccc-8ddd-000000000001","aid":"0","net":"tcp","tls":"tls"}"#);
    let list = format!("ss://YWVzLTI1Ni1nY206cGFzc3dvcmQ@ss.example:8388#SS\nvmess://{vmess}\nvless://fixture@vless.example:443#VLESS\ntrojan://fixture@trojan.example:443#Trojan\nhy2://fixture@hy2.example:443#Hy2\nsocks5://user:pass@socks.example:1080#Socks\nhttp://user:pass@http.example:8080#HTTP");
    let encoded = base64::engine::general_purpose::STANDARD.encode(&list);
    let markdown = format!("Subscription fixture\n```text\n{list}\n```\nEnd");
    for input in [&list, &encoded, &markdown] {
        let nodes = parse_subscription(input, None, &mut metadata());
        let mut protocols: Vec<_> = nodes.iter().map(|n| n.protocol.as_str()).collect();
        protocols.sort();
        assert_eq!(
            protocols,
            vec![
                "http",
                "hysteria2",
                "shadowsocks",
                "socks5",
                "trojan",
                "vless",
                "vmess"
            ]
        );
    }
}
