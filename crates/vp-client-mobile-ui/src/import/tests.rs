use super::*;
use vp_client_core::{geo::BytesGeoProvider, parser::NodeMetadata};

const URI: &str = "trojan://synthetic-password@192.0.2.1:443#Example";
fn metadata() -> impl NodeMetadataProvider {
    let mut n = 0;
    move || {
        n += 1;
        NodeMetadata {
            id: format!("test-node-{n}"),
            timestamp: "2026-01-01T00:00:00Z".parse().unwrap(),
        }
    }
}
fn missing() -> BytesGeoProvider<'static> {
    BytesGeoProvider {
        site: Err(GeoUnavailable::Missing),
        ip: Err(GeoUnavailable::Missing),
    }
}
fn yaml(rule: &str) -> String {
    format!("proxies:\n  - name: Test\n    type: trojan\n    server: 192.0.2.1\n    port: 443\n    password: synthetic\nrules:\n  - '{rule}'\n")
}
#[test]
fn preview_confirm_uses_injected_metadata_and_never_tests_latency() {
    let mut state = ImportState::default();
    state.set_input(URI).unwrap();
    state.prepare(&mut metadata(), &missing()).unwrap();
    assert!(state.nodes().is_empty());
    assert_eq!(state.preview().unwrap().nodes()[0].id, "test-node-1");
    assert_eq!(state.confirm(), Ok(1));
    let node = &state.nodes()[0];
    assert_eq!(node.created_at.to_rfc3339(), "2026-01-01T00:00:00+00:00");
    assert!(node.source_id.is_none());
    assert!(node.latency_ms.is_none() && node.latency_ok.is_none());
    assert!(node.download_kbps.is_none() && node.latency_tested_at.is_none());
    assert_eq!(state.confirm(), Err(ImportError::NoPreview));
}
#[test]
fn edits_and_cancel_invalidate_preview_without_destroying_committed_nodes() {
    let mut state = ImportState::default();
    state.set_input(URI).unwrap();
    state.prepare(&mut metadata(), &missing()).unwrap();
    state.confirm().unwrap();
    state.set_input(URI).unwrap();
    state.prepare(&mut metadata(), &missing()).unwrap();
    state.set_input(URI).unwrap();
    assert_eq!(state.confirm(), Err(ImportError::NoPreview));
    state.prepare(&mut metadata(), &missing()).unwrap();
    state.cancel();
    assert_eq!(state.confirm(), Err(ImportError::NoPreview));
    assert_eq!(state.nodes().len(), 1);
}
#[test]
fn empty_unsupported_and_oversize_are_not_successes() {
    let mut state = ImportState::default();
    assert_eq!(
        state.prepare(&mut metadata(), &missing()),
        Err(ImportError::Empty)
    );
    state
        .set_input("https://subscription.example.invalid/private-token")
        .unwrap();
    assert_eq!(
        state.prepare(&mut metadata(), &missing()),
        Err(ImportError::NoNodes)
    );
    assert_eq!(
        state.set_input(&"x".repeat(MAX_INPUT_BYTES + 1)),
        Err(ImportError::TooLarge)
    );
    assert!(state.preview().is_none() && state.nodes().is_empty());
}
#[test]
fn base64_and_partial_content_use_upstream_parser_without_rejection_counts() {
    let mut state = ImportState::default();
    // Base64 of a synthetic URI; no extra test dependency needed.
    state
        .set_input("dHJvamFuOi8vc3ludGhldGljLXBhc3N3b3JkQDE5Mi4wLjIuMTo0NDMjRXhhbXBsZQ==")
        .unwrap();
    state.prepare(&mut metadata(), &missing()).unwrap();
    assert_eq!(state.confirm(), Ok(1));
    state
        .set_input(&format!("{URI}\nthis is not a node\ntrojan://broken"))
        .unwrap();
    state.prepare(&mut metadata(), &missing()).unwrap();
    assert_eq!(state.preview().unwrap().nodes().len(), 1);
    assert_eq!(state.confirm(), Ok(0));
    assert_eq!(state.nodes()[0].id, "test-node-1");
}
#[test]
fn transport_variants_remain_distinct() {
    let mut state = ImportState::default();
    state.set_input("trojan://synthetic@192.0.2.1:443?type=ws&path=%2Fa\ntrojan://synthetic@192.0.2.1:443?type=ws&path=%2Fb").unwrap();
    state.prepare(&mut metadata(), &missing()).unwrap();
    assert_eq!(state.confirm(), Ok(2));
}
#[test]
fn geo_failures_are_distinct_and_categories_are_not_copied_to_warnings() {
    for (site, expected, kept) in [
        (
            Err(GeoUnavailable::Missing),
            CategoryLookup::Unavailable(GeoUnavailable::Missing),
            1,
        ),
        (
            Err(GeoUnavailable::Unreadable),
            CategoryLookup::Unavailable(GeoUnavailable::Unreadable),
            1,
        ),
        (
            Ok(&[8u8][..]),
            CategoryLookup::Unavailable(GeoUnavailable::Malformed),
            1,
        ),
        (Ok(&[][..]), CategoryLookup::Absent, 0),
    ] {
        let mut state = ImportState::default();
        state
            .set_input(&yaml("GEOSITE,sensitive-category,DIRECT"))
            .unwrap();
        let geo = BytesGeoProvider {
            site,
            ip: Err(GeoUnavailable::Missing),
        };
        state.prepare(&mut metadata(), &geo).unwrap();
        let preview = state.preview().unwrap();
        assert_eq!(preview.warnings()[0].lookup, expected);
        assert_eq!(preview.rules().len(), kept);
        assert!(!format!("{:?}", preview.warnings()).contains("sensitive-category"));
    }
}
#[test]
fn too_many_nodes_rejected_before_preview() {
    let mut state = ImportState::default();
    let input = (1..=MAX_NODES + 1)
        .map(|p| format!("trojan://synthetic@192.0.2.1:{p}"))
        .collect::<Vec<_>>()
        .join("\n");
    state.set_input(&input).unwrap();
    assert_eq!(
        state.prepare(&mut metadata(), &missing()),
        Err(ImportError::TooManyNodes)
    );
    assert!(state.nodes().is_empty() && state.preview().is_none());
}
#[test]
fn cumulative_limit_is_atomic() {
    let mut state = ImportState::default();
    for (start, end) in [(1, MAX_NODES), (MAX_NODES + 1, MAX_NODES + 1)] {
        let input = (start..=end)
            .map(|p| format!("trojan://synthetic@192.0.2.1:{p}"))
            .collect::<Vec<_>>()
            .join("\n");
        state.set_input(&input).unwrap();
        state.prepare(&mut metadata(), &missing()).unwrap();
        if start == 1 {
            assert_eq!(state.confirm(), Ok(MAX_NODES));
        } else {
            assert_eq!(state.confirm(), Err(ImportError::TooManyNodes));
        }
    }
    assert_eq!(state.nodes().len(), MAX_NODES);
}
#[test]
fn display_text_is_bounded_and_removes_controls_and_bidi() {
    assert_eq!(display_text("a\n\u{202e}b"), "ab");
    assert_eq!(display_text(&"中".repeat(200)).chars().count(), 96);
}
#[test]
fn rule_limit_rejects_before_preview() {
    let mut state = ImportState::default();
    let mut content = yaml("DOMAIN,first.example.invalid,DIRECT");
    for i in 0..MAX_RULES {
        content.push_str(&format!("  - 'DOMAIN,n{i}.example.invalid,DIRECT'\n"));
    }
    state.set_input(&content).unwrap();
    assert_eq!(
        state.prepare(&mut metadata(), &missing()),
        Err(ImportError::TooManyRules)
    );
    assert!(state.rules().is_empty() && state.preview().is_none());
}
#[test]
fn cumulative_rule_limit_does_not_commit_nodes_or_rules() {
    let mut state = ImportState::default();
    let mut content = yaml("DOMAIN,first.example.invalid,DIRECT");
    for i in 1..MAX_RULES {
        content.push_str(&format!("  - 'DOMAIN,n{i}.example.invalid,DIRECT'\n"));
    }
    state.set_input(&content).unwrap();
    state.prepare(&mut metadata(), &missing()).unwrap();
    assert_eq!(state.confirm(), Ok(1));
    state
        .set_input(&yaml("DOMAIN,extra.example.invalid,DIRECT").replace("192.0.2.1", "192.0.2.2"))
        .unwrap();
    state.prepare(&mut metadata(), &missing()).unwrap();
    assert_eq!(state.confirm(), Err(ImportError::TooManyRules));
    assert_eq!(state.nodes().len(), 1);
    assert_eq!(state.rules().len(), MAX_RULES);
}
#[test]
fn input_byte_limit_handles_multibyte_text_and_invalidates_old_preview() {
    let mut state = ImportState::default();
    state.set_input(URI).unwrap();
    state.prepare(&mut metadata(), &missing()).unwrap();
    assert_eq!(
        state.set_input(&"中".repeat(MAX_INPUT_BYTES / 3 + 1)),
        Err(ImportError::TooLarge)
    );
    assert_eq!(state.confirm(), Err(ImportError::NoPreview));
    assert!(state.nodes().is_empty());
}
#[test]
fn unverified_geoip_compatibility_does_not_imply_asset_availability() {
    let mut state = ImportState::default();
    state.set_input(&yaml("GEOIP,ZZ,DIRECT")).unwrap();
    state.prepare(&mut metadata(), &missing()).unwrap();
    // Existing core behavior: two-letter GeoIP codes bypass inventory lookup.
    // The UI must retain its unconditional 'assets not supplied' notice.
    assert_eq!(state.preview().unwrap().rules().len(), 1);
}
