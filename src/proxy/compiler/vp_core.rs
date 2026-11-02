use super::{BackendCompiler, CompilationDiagnostic, CompiledArtifact, CompilerCapabilities};
use crate::kernel::KernelKind;
use crate::proxy::ir::RoutingPlan;
use anyhow::Result;
use vp_client_core::compilation::compiler::BackendCompiler as CoreCompiler;
#[derive(Default)]
pub struct VpCoreCompiler;
impl VpCoreCompiler {
    pub fn new() -> Self {
        Self
    }
    fn shared() -> &'static vp_client_core::compilation::compiler::VpCoreCompiler {
        static SHARED: std::sync::OnceLock<vp_client_core::compilation::compiler::VpCoreCompiler> =
            std::sync::OnceLock::new();
        SHARED.get_or_init(Default::default)
    }
}
impl BackendCompiler for VpCoreCompiler {
    fn kernel(&self) -> KernelKind {
        Self::shared().kernel().into()
    }
    fn capabilities(&self) -> &CompilerCapabilities {
        Self::shared().capabilities()
    }
    fn validate(&self, plan: &RoutingPlan) -> Vec<CompilationDiagnostic> {
        Self::shared().validate(plan)
    }
    fn compile(&self, plan: &RoutingPlan) -> Result<CompiledArtifact> {
        super::with_desktop_context(|context| {
            Self::shared().compile(plan, context, &mut Vec::new())
        })
    }
}

pub use vp_client_core::compilation::compiler::vp_core::is_valid_vpcore_geoip_code;

#[cfg(test)]
mod tests {
    use crate::models::Node;
    use serde_json::json;
    #[test]
    fn test_vpcore_incompatibility_detection() {
        use crate::kernel::is_node_incompatible_with_vpcore;
        let mk = |proto: &str, config: serde_json::Value, raw: &str| Node {
            id: "t".into(),
            source_id: None,
            name: "t".into(),
            protocol: proto.into(),
            server: "1.2.3.4".into(),
            port: 443,
            raw: raw.into(),
            config,
            country: None,
            latency_ms: None,
            latency_ok: None,
            download_kbps: None,
            latency_kernel: None,
            latency_test_kind: None,
            latency_tested_at: None,
            created_at: chrono::DateTime::from_timestamp(0, 0).unwrap(),
            updated_at: chrono::DateTime::from_timestamp(0, 0).unwrap(),
        };

        assert!(
            is_node_incompatible_with_vpcore(&mk("vless", json!({"network": "xhttp"}), ""))
                .is_some()
        );
        assert!(is_node_incompatible_with_vpcore(&mk(
            "vmess",
            json!({"network": "httpupgrade"}),
            ""
        ))
        .is_some());
        assert!(is_node_incompatible_with_vpcore(&mk(
            "vless",
            json!({}),
            "vless://a@1.2.3.4:443?network=xhttp#x"
        ))
        .is_some());
        assert!(is_node_incompatible_with_vpcore(&mk("masque", json!({}), "")).is_some());
        // http / ws / tcp 传输自 HTTP 伪装传输落地后均受支持
        assert!(
            is_node_incompatible_with_vpcore(&mk("vless", json!({"network": "http"}), ""))
                .is_none()
        );
        assert!(
            is_node_incompatible_with_vpcore(&mk("vless", json!({"network": "ws"}), "")).is_none()
        );
        assert!(is_node_incompatible_with_vpcore(&mk("ss", json!({}), "")).is_none());
    }
}
