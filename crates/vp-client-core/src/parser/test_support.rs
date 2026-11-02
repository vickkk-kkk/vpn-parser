//! Compatibility-shaped helpers used only by retained upstream tests.
#![allow(dead_code)]
use crate::models::Node;
fn metadata() -> impl super::NodeMetadataProvider {
    let mut next = 0u64;
    move || {
        next += 1;
        super::NodeMetadata {
            id: format!("fixture-{next}"),
            timestamp: chrono::DateTime::from_timestamp(1_700_000_000 + next as i64, 0).unwrap(),
        }
    }
}
fn geo_bytes() -> Vec<u8> {
    let mut out = Vec::new();
    for category in [
        "cn",
        "private",
        "category-ads-all",
        "geolocation-!cn",
        "gfw",
        "google",
        "telegram",
    ] {
        out.extend([0x0a, (category.len() + 2) as u8, 0x0a, category.len() as u8]);
        out.extend(category.as_bytes());
    }
    out
}
pub fn parse_subscription(content: &str, source: Option<&str>) -> Vec<Node> {
    super::parse_subscription(content, source, &mut metadata())
}
pub fn parse_uri(content: &str, source: Option<&str>) -> Option<Node> {
    super::parse_uri(content, source, &mut metadata())
}
pub fn parse_clash_yaml(content: &str, source: Option<&str>) -> anyhow::Result<Vec<Node>> {
    super::parse_clash_yaml(content, source, &mut metadata())
}
pub fn parse_openrung_json(content: &str, source: Option<&str>) -> Option<Vec<Node>> {
    super::parse_openrung_json(content, source, &mut metadata())
}
pub fn parse_warp_masque_json(content: &str, source: Option<&str>) -> Option<Vec<Node>> {
    super::parse_warp_masque_json(content, source, &mut metadata())
}
pub fn parse_masque_uri(content: &str, source: Option<&str>) -> Option<Node> {
    super::warp_masque::parse_masque_uri(content, source, &mut metadata())
}
pub fn make_node(
    protocol: &str,
    name: &str,
    server: &str,
    port: u16,
    raw: &str,
    config: serde_json::Value,
    source: Option<&str>,
) -> Node {
    super::make_node(
        protocol,
        name,
        server,
        port,
        raw,
        config,
        source,
        &mut metadata(),
    )
}
pub fn extract_clash_rules(content: &str) -> Vec<crate::config::TrafficRule> {
    let bytes = geo_bytes();
    let geo = crate::geo::BytesGeoProvider {
        site: Ok(&bytes),
        ip: Ok(&bytes),
    };
    super::extract_clash_rules(content, &geo, &mut Vec::new())
}
pub fn extract_clash_rules_yaml(content: &str) -> Option<String> {
    let bytes = geo_bytes();
    let geo = crate::geo::BytesGeoProvider {
        site: Ok(&bytes),
        ip: Ok(&bytes),
    };
    super::extract_clash_rules_yaml(content, &geo, &mut Vec::new())
}
pub fn extract_subscription_rules_and_groups(content: &str) -> (Option<String>, Option<String>) {
    let bytes = geo_bytes();
    let geo = crate::geo::BytesGeoProvider {
        site: Ok(&bytes),
        ip: Ok(&bytes),
    };
    super::extract_subscription_rules_and_groups(content, &geo, &mut Vec::new())
}
