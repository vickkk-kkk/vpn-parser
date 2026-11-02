//! Pure upstream six-engine configuration compilation. No engine is started here.
pub mod compiler;
mod identity;
pub mod ir;
pub mod options;
use crate::{
    geo::{CategoryLookup, GeoKind, GeoProvider},
    models::Node,
};
use compiler::{CompilationDiagnostic, CompiledArtifact, DiagnosticSeverity};
pub use identity::{KernelKind, KernelPreference};
use options::MihomoConfigOptions;

/// Explicit target asset references. Implementations must provide the *actual*
/// category inventory, without the legacy two-letter country shortcut. The path
/// is opaque target-engine text; core never resolves, opens or creates it.
pub trait GeoAssetProvider {
    fn asset_path(&self, kind: GeoKind) -> Option<&str>;
    fn lookup(&self, kind: GeoKind, code: &str) -> CategoryLookup;
}

/// There is intentionally no default provider or inferred filesystem location.
pub struct CompilationContext<'a> {
    pub geo: &'a dyn GeoProvider,
    pub shoes_geo: &'a dyn GeoAssetProvider,
    pub connectivity_test_url: &'a str,
    pub mirror_direct_hosts: &'a [&'a str],
}

#[derive(Clone, Debug)]
pub struct CompilationOutput {
    pub artifact: CompiledArtifact,
    pub diagnostics: Vec<CompilationDiagnostic>,
}

/// Diagnostic messages intentionally exclude node names, credentials and input text.
#[derive(Clone, Debug)]
pub struct CompilationFailure {
    pub diagnostics: Vec<CompilationDiagnostic>,
}
impl std::fmt::Display for CompilationFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Configuration compilation rejected; inspect structured diagnostics")
    }
}
impl std::error::Error for CompilationFailure {}

fn error(message: &'static str) -> CompilationDiagnostic {
    CompilationDiagnostic {
        severity: DiagnosticSeverity::Error,
        message: message.into(),
        rule_index: None,
    }
}

/// Checked portable entrypoint. Success means configuration compilation only,
/// never engine availability, native VPN support, or a successful connection.
pub fn compile_configuration(
    kernel: KernelKind,
    options: &MihomoConfigOptions,
    primary_node: Option<&Node>,
    context: &CompilationContext<'_>,
) -> Result<CompilationOutput, CompilationFailure> {
    let mut diagnostics = Vec::new();
    if options.validate().is_err() {
        return Err(CompilationFailure {
            diagnostics: vec![error("Invalid configuration options")],
        });
    }
    let plan = ir::lower_options_to_ir(options, primary_node, context, &mut diagnostics);
    let compiler = compiler::get_backend_compiler(kernel);
    diagnostics.extend(compiler.validate(&plan));
    if diagnostics
        .iter()
        .any(|d| d.severity == DiagnosticSeverity::Error)
    {
        return Err(CompilationFailure { diagnostics });
    }
    match compiler.compile(&plan, context, &mut diagnostics) {
        Ok(artifact)
            if !diagnostics
                .iter()
                .any(|d| d.severity == DiagnosticSeverity::Error) =>
        {
            Ok(CompilationOutput {
                artifact,
                diagnostics,
            })
        }
        Ok(_) => Err(CompilationFailure { diagnostics }),
        Err(_) => {
            diagnostics.push(error(
                "Selected backend rejected the configuration or protocol",
            ));
            Err(CompilationFailure { diagnostics })
        }
    }
}
