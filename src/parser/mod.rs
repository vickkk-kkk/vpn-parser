//! Desktop compatibility API over the shared upstream parser.
pub mod openrung;
pub mod warp_masque;
use crate::models::Node;
use chrono::Local;
pub use openrung::parse_openrung_json;
pub use vp_client_core::parser::{
    clean_node_name, decode_maybe_base64, empty_config, extract_clash_groups_yaml, extract_uris,
    fingerprint_from_parts, guess_country, guess_country_with_server, looks_like_clash,
    node_fingerprint, normalize_protocol, normalize_subscription_text,
};
pub use warp_masque::parse_warp_masque_json;

pub(crate) struct DesktopMetadata;
impl vp_client_core::parser::NodeMetadataProvider for DesktopMetadata {
    fn next_metadata(&mut self) -> vp_client_core::parser::NodeMetadata {
        let timestamp = chrono::Utc::now();
        let id = uuid::Uuid::new_v4().to_string();
        vp_client_core::parser::NodeMetadata { id, timestamp }
    }
}
pub fn parse_subscription(content: &str, source_id: Option<&str>) -> Vec<Node> {
    vp_client_core::parser::parse_subscription(content, source_id, &mut DesktopMetadata)
}
pub fn parse_uri(content: &str, source_id: Option<&str>) -> Option<Node> {
    vp_client_core::parser::parse_uri(content, source_id, &mut DesktopMetadata)
}
pub fn parse_clash_yaml(content: &str, source_id: Option<&str>) -> anyhow::Result<Vec<Node>> {
    vp_client_core::parser::parse_clash_yaml(content, source_id, &mut DesktopMetadata)
}
pub fn make_node(
    protocol: &str,
    name: &str,
    server: &str,
    port: u16,
    raw: &str,
    config: serde_json::Value,
    source_id: Option<&str>,
) -> Node {
    vp_client_core::parser::make_node(
        protocol,
        name,
        server,
        port,
        raw,
        config,
        source_id,
        &mut DesktopMetadata,
    )
}
/// Expand date placeholders in subscription URL templates.
/// Supported: {Ymd} {ymd} {Y} {m} {mm} {d} {dd}
pub fn expand_url_template(template: &str) -> String {
    let now = Local::now();
    let y = now.format("%Y").to_string();
    let m = now.format("%m").to_string();
    let d = now.format("%d").to_string();
    let ymd = format!("{y}{m}{d}");
    let ymd_short = format!("{}{}{}", &y[2..], m, d);
    let m_no0 = m.trim_start_matches('0');
    let d_no0 = d.trim_start_matches('0');
    let m_short = if m_no0.is_empty() { "0" } else { m_no0 };
    let d_short = if d_no0.is_empty() { "0" } else { d_no0 };

    template
        .replace("{Ymd}", &ymd)
        .replace("{ymd}", &ymd_short)
        .replace("{Y}", &y)
        .replace("{mm}", &m)
        .replace("{m}", m_short)
        .replace("{dd}", &d)
        .replace("{d}", d_short)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_solispirit_sample() {
        let path = "data/sample-solispirit.txt";
        let Ok(content) = std::fs::read_to_string(path) else {
            eprintln!("sample missing, skip");
            return;
        };
        let uris = extract_uris(&content);
        let mut parsed = 0usize;
        let mut fail_ss = 0usize;
        let mut fail_vmess = 0usize;
        let mut fail_vless = 0usize;
        let mut fail_trojan = 0usize;
        let mut fail_other = 0usize;
        let mut fail_samples = Vec::new();
        for uri in &uris {
            if parse_uri(uri, None).is_some() {
                parsed += 1;
                continue;
            }
            let low = uri.to_ascii_lowercase();
            if low.starts_with("ss://") {
                fail_ss += 1;
            } else if low.starts_with("vmess://") {
                fail_vmess += 1;
            } else if low.starts_with("vless://") {
                fail_vless += 1;
            } else if low.starts_with("trojan://") {
                fail_trojan += 1;
            } else {
                fail_other += 1;
            }
            if fail_samples.len() < 8 {
                fail_samples.push(uri.chars().take(140).collect::<String>());
            }
        }
        let nodes = parse_subscription(&content, Some("test"));
        eprintln!(
            "uris={} parsed_raw={} deduped={} fail_ss={} fail_vmess={} fail_vless={} fail_trojan={} fail_other={}",
            uris.len(),
            parsed,
            nodes.len(),
            fail_ss,
            fail_vmess,
            fail_vless,
            fail_trojan,
            fail_other
        );
        for s in fail_samples {
            eprintln!("FAIL {s}");
        }
        assert!(parsed > 7000, "raw parse too low: {parsed}");
        assert!(nodes.len() > 3500, "deduped too low: {}", nodes.len());
    }

    #[test]
    fn expand_date_template() {
        let result = expand_url_template("https://example.com/{Ymd}-clash.yaml");
        assert!(!result.contains("{Ymd}"));
    }
}

fn warn_filtered(diagnostics: &[vp_client_core::parser::RuleDiagnostic]) {
    for diagnostic in diagnostics {
        if diagnostic.lookup == vp_client_core::geo::CategoryLookup::Absent {
            tracing::warn!(category = %diagnostic.category, kind = ?diagnostic.kind,
                "订阅规则中含有本地不存在的 GEO 分类，入口已自动清洗跳过");
        }
    }
}
pub fn extract_clash_rules(content: &str) -> Vec<crate::proxy::TrafficRule> {
    let mut diagnostics = Vec::new();
    let result = vp_client_core::parser::extract_clash_rules(
        content,
        &crate::geo::DesktopGeoProvider,
        &mut diagnostics,
    );
    warn_filtered(&diagnostics);
    result
}
pub fn extract_clash_rules_yaml(content: &str) -> Option<String> {
    let mut diagnostics = Vec::new();
    let result = vp_client_core::parser::extract_clash_rules_yaml(
        content,
        &crate::geo::DesktopGeoProvider,
        &mut diagnostics,
    );
    warn_filtered(&diagnostics);
    result
}
pub fn extract_subscription_rules(content: &str) -> Vec<crate::proxy::TrafficRule> {
    let mut diagnostics = Vec::new();
    let result = vp_client_core::parser::extract_subscription_rules(
        content,
        &crate::geo::DesktopGeoProvider,
        &mut diagnostics,
    );
    warn_filtered(&diagnostics);
    result
}
pub fn extract_subscription_rules_and_groups(content: &str) -> (Option<String>, Option<String>) {
    let mut diagnostics = Vec::new();
    let result = vp_client_core::parser::extract_subscription_rules_and_groups(
        content,
        &crate::geo::DesktopGeoProvider,
        &mut diagnostics,
    );
    warn_filtered(&diagnostics);
    result
}

#[cfg(test)]
mod facade_tests {
    use super::*;
    #[test]
    fn desktop_facade_keeps_node_shape_and_real_metadata() {
        let yaml = "proxies: [{type: future, name: fixture, server: proxy.example, port: 443, opaque: {value: true}}]";
        let first = parse_subscription(yaml, Some("fixture"));
        let second = parse_subscription(yaml, Some("fixture"));
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].protocol, "future");
        assert_eq!(first[0].config["opaque"]["value"], true);
        assert_eq!(first[0].source_id.as_deref(), Some("fixture"));
        assert_eq!(first[0].created_at, first[0].updated_at);
        assert_eq!(
            uuid::Uuid::parse_str(&first[0].id)
                .unwrap()
                .get_version_num(),
            4
        );
        assert_ne!(first[0].id, second[0].id);
    }
}
