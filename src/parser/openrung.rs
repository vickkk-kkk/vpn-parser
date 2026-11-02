//! Desktop metadata adapter for the shared broker parser.
use crate::models::Node;
pub fn parse_openrung_json(content: &str, source_id: Option<&str>) -> Option<Vec<Node>> {
    vp_client_core::parser::openrung::parse_openrung_json(
        content,
        source_id,
        &mut super::DesktopMetadata,
    )
}
