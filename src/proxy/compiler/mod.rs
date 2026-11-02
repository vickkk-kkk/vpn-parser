//! Desktop compatibility wrappers. Filesystem selection stays here, never in core.
pub mod meow;
pub mod mihomo;
pub mod shoes;
pub mod sing_box;
pub mod vp_core;
pub mod xray;
use crate::kernel::KernelKind;
use crate::proxy::ir::RoutingPlan;
use anyhow::Result;
pub use meow::MeowCompiler;
pub use mihomo::MihomoCompiler;
pub use shoes::ShoesCompiler;
pub use sing_box::SingBoxCompiler;
pub use vp_client_core::compilation::compiler::{
    CompilationDiagnostic, CompiledArtifact, CompilerCapabilities, DiagnosticSeverity,
};
use vp_client_core::compilation::{CompilationContext, GeoAssetProvider};
use vp_client_core::geo::{CategoryLookup, GeoKind, GeoUnavailable};
pub use vp_core::VpCoreCompiler;
pub use xray::XrayCompiler;

pub trait BackendCompiler: Send + Sync {
    fn kernel(&self) -> KernelKind;
    fn capabilities(&self) -> &CompilerCapabilities;
    fn validate(&self, plan: &RoutingPlan) -> Vec<CompilationDiagnostic>;
    /// Legacy desktop behavior: compilation does not newly enforce validation.
    fn compile(&self, plan: &RoutingPlan) -> Result<CompiledArtifact>;
}

pub fn get_backend_compiler(kernel: KernelKind) -> Box<dyn BackendCompiler> {
    match kernel {
        KernelKind::Mihomo => Box::new(MihomoCompiler),
        KernelKind::SingBox => Box::new(SingBoxCompiler),
        KernelKind::Xray => Box::new(XrayCompiler),
        KernelKind::Shoes => Box::new(ShoesCompiler),
        KernelKind::Meow => Box::new(MeowCompiler),
        KernelKind::VpCore => Box::new(VpCoreCompiler),
    }
}

struct DesktopGeoAssets {
    site: Option<String>,
    ip: Option<String>,
}
impl GeoAssetProvider for DesktopGeoAssets {
    fn asset_path(&self, kind: GeoKind) -> Option<&str> {
        match kind {
            GeoKind::Site => self.site.as_deref(),
            GeoKind::Ip => self.ip.as_deref(),
        }
    }
    fn lookup(&self, kind: GeoKind, code: &str) -> CategoryLookup {
        let Some(path) = self.asset_path(kind) else {
            return CategoryLookup::Unavailable(GeoUnavailable::Missing);
        };
        match std::fs::read(path) {
            Ok(bytes) => vp_client_core::geo::dat_category_lookup(
                &bytes,
                crate::geo::normalize_geo_code(code),
            ),
            Err(_) => CategoryLookup::Unavailable(GeoUnavailable::Unreadable),
        }
    }
}

pub(crate) fn with_desktop_context<T>(f: impl FnOnce(&CompilationContext<'_>) -> T) -> T {
    let assets = DesktopGeoAssets {
        site: crate::geo::find_geo_dat_file("geosite.dat")
            .map(|p| p.to_string_lossy().into_owned()),
        ip: crate::geo::find_geo_dat_file("geoip.dat").map(|p| p.to_string_lossy().into_owned()),
    };
    let context = CompilationContext {
        geo: &crate::geo::DesktopGeoProvider,
        shoes_geo: &assets,
        connectivity_test_url: crate::proxy_probe::PROXY_CONNECTIVITY_TEST_URL,
        mirror_direct_hosts: &crate::geo::MIRROR_DIRECT_HOSTS,
    };
    f(&context)
}
