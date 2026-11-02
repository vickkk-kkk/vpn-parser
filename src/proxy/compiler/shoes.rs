use super::{BackendCompiler, CompilationDiagnostic, CompiledArtifact, CompilerCapabilities};
use crate::kernel::KernelKind;
use crate::proxy::ir::RoutingPlan;
use anyhow::Result;
use vp_client_core::compilation::compiler::BackendCompiler as CoreCompiler;
#[derive(Default)]
pub struct ShoesCompiler;
impl ShoesCompiler {
    pub fn new() -> Self {
        Self
    }
    fn shared() -> &'static vp_client_core::compilation::compiler::ShoesCompiler {
        static SHARED: std::sync::OnceLock<vp_client_core::compilation::compiler::ShoesCompiler> =
            std::sync::OnceLock::new();
        SHARED.get_or_init(Default::default)
    }
}
impl BackendCompiler for ShoesCompiler {
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
pub use vp_client_core::compilation::compiler::shoes::build_shoes_client_proxy;
pub(crate) fn find_geo_file(name: &str) -> Option<std::path::PathBuf> {
    crate::geo::find_geo_dat_file(name)
}
pub(crate) fn has_geosite_category(code: &str) -> bool {
    find_geo_file("geosite.dat").is_some_and(|path| {
        crate::geo::dat_has_category(&path, crate::geo::normalize_geo_code(code))
    })
}
