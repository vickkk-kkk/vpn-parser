use super::{BackendCompiler, CompilationDiagnostic, CompiledArtifact, CompilerCapabilities};
use crate::kernel::KernelKind;
use crate::proxy::ir::RoutingPlan;
use anyhow::Result;
use vp_client_core::compilation::compiler::BackendCompiler as CoreCompiler;
#[derive(Default)]
pub struct SingBoxCompiler;
impl SingBoxCompiler {
    pub fn new() -> Self {
        Self
    }
    fn shared() -> &'static vp_client_core::compilation::compiler::SingBoxCompiler {
        static SHARED: std::sync::OnceLock<vp_client_core::compilation::compiler::SingBoxCompiler> =
            std::sync::OnceLock::new();
        SHARED.get_or_init(Default::default)
    }
}
impl BackendCompiler for SingBoxCompiler {
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
pub use vp_client_core::compilation::compiler::sing_box::{
    build_dns, GEOIP_CN_URL, GEOSITE_ADS_URL, GEOSITE_CN_URL, GEOSITE_GFW_URL,
};
