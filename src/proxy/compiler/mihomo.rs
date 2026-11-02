use super::{BackendCompiler, CompilationDiagnostic, CompiledArtifact, CompilerCapabilities};
use crate::kernel::KernelKind;
use crate::proxy::ir::RoutingPlan;
use anyhow::Result;
use vp_client_core::compilation::compiler::BackendCompiler as CoreCompiler;
#[derive(Default)]
pub struct MihomoCompiler;
impl MihomoCompiler {
    pub fn new() -> Self {
        Self
    }
    fn shared() -> &'static vp_client_core::compilation::compiler::MihomoCompiler {
        static SHARED: std::sync::OnceLock<vp_client_core::compilation::compiler::MihomoCompiler> =
            std::sync::OnceLock::new();
        SHARED.get_or_init(Default::default)
    }
}
impl BackendCompiler for MihomoCompiler {
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
pub use vp_client_core::compilation::compiler::mihomo::{
    FINAL_GROUP, SMART_ROUTING_PREPEND_RULES, URL_TEST_GROUP, URL_TEST_INTERVAL_SECS,
    URL_TEST_TIMEOUT_MS, URL_TEST_TOLERANCE_MS,
};
