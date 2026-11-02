//! Checked compiler preview only. No engine discovery, execution or native VPN.
use vp_client_core::{
    compilation::{
        compile_configuration,
        compiler::{CompilationDiagnostic, CompiledArtifact},
        options::{AutoRoutingStrategy, MihomoConfigOptions, ProxyMode},
        CompilationContext, GeoAssetProvider, KernelKind, KernelPreference,
    },
    config::{DnsConfig, RoutingMode, TrafficRule, TunConfig, TunStack},
    geo::{CategoryLookup, GeoKind, GeoProvider, GeoUnavailable},
    models::Node,
};

pub const ENGINES: [KernelKind; 6] = [
    KernelKind::Mihomo,
    KernelKind::SingBox,
    KernelKind::Xray,
    KernelKind::Shoes,
    KernelKind::Meow,
    KernelKind::VpCore,
];
pub const TEMPLATE_NOTICE: &str = "Non-runnable compiler template only. No native engine, packet tunnel, geo asset layout or controller identity is provisioned. Nothing has been started.";

/// Both interfaces report missing evidence; no legacy GeoIP shortcut or fake path.
struct MissingGeo;
impl GeoProvider for MissingGeo {
    fn lookup(&self, _: GeoKind, _: &str) -> CategoryLookup {
        CategoryLookup::Unavailable(GeoUnavailable::Missing)
    }
}
impl GeoAssetProvider for MissingGeo {
    fn asset_path(&self, _: GeoKind) -> Option<&str> {
        None
    }
    fn lookup(&self, _: GeoKind, _: &str) -> CategoryLookup {
        CategoryLookup::Unavailable(GeoUnavailable::Missing)
    }
}

/// Deliberately not Debug: artifact and revealed text can contain credentials.
#[derive(Default)]
pub struct ConfigPreview {
    engine: Option<KernelKind>,
    node_id: Option<String>,
    artifact: Option<CompiledArtifact>,
    diagnostics: Vec<CompilationDiagnostic>,
    revealed: Option<String>,
    status: &'static str,
    inactive_rules: bool,
}
impl ConfigPreview {
    pub fn engine(&self) -> Option<KernelKind> {
        self.engine
    }
    pub fn node_id(&self) -> Option<&str> {
        self.node_id.as_deref()
    }
    pub fn select_engine(&mut self, engine: KernelKind) {
        if self.engine != Some(engine) {
            self.invalidate();
            self.engine = Some(engine);
        }
    }
    pub fn select_node(&mut self, id: Option<String>) {
        if self.node_id != id {
            self.invalidate();
            self.node_id = id;
        }
    }
    pub fn invalidate(&mut self) {
        self.artifact = None;
        self.diagnostics.clear();
        self.revealed = None;
        self.status = "Inputs changed; compile a new preview.";
        self.inactive_rules = false;
    }
    pub fn hide(&mut self) {
        self.revealed = None;
    }
    pub fn revealed(&self) -> Option<&str> {
        self.revealed.as_deref()
    }
    pub fn diagnostics(&self) -> &[CompilationDiagnostic] {
        &self.diagnostics
    }
    pub fn inactive_rules(&self) -> bool {
        self.inactive_rules
    }
    pub fn has_artifact(&self) -> bool {
        self.artifact.is_some()
    }
    /// Safe by construction: no node name, endpoint, ID, config or input text.
    pub fn summary(&self) -> String {
        let format = match self.artifact.as_ref() {
            Some(CompiledArtifact::Yaml(_)) => "YAML",
            Some(CompiledArtifact::Json(_)) => "JSON",
            None => "no artifact",
        };
        format!(
            "{} · {} · {}",
            self.engine
                .map(|e| e.display_name())
                .unwrap_or("Choose engine"),
            format,
            self.status
        )
    }
    /// Call only after the user's explicit reveal action. Never copy/export here.
    pub fn reveal(&mut self) {
        self.revealed = self
            .artifact
            .as_ref()
            .and_then(|a| a.to_string_pretty().ok());
    }
    pub fn compile(
        &mut self,
        nodes: &[Node],
        dns: &DnsConfig,
        dns_draft: &str,
        mode: RoutingMode,
        manual: &[TrafficRule],
        imported: &[TrafficRule],
    ) {
        self.invalidate();
        let Some(engine) = self.engine else {
            self.status = "Choose an engine; no automatic fallback.";
            return;
        };
        let Some(node) = nodes.iter().find(|n| Some(n.id.as_str()) == self.node_id()) else {
            self.status = "Choose an existing imported node.";
            return;
        };
        let mut dns = dns.clone();
        dns.nameservers = dns_draft
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect();
        if dns.validate().is_err() {
            self.status = "Current DNS draft is invalid; no configuration generated.";
            return;
        }
        // Manual exceptions first, then imported drafts; only exact duplicates removed.
        let rules = combined_rules(manual, imported);
        self.inactive_rules =
            !rules.is_empty() && matches!(mode, RoutingMode::Global | RoutingMode::Direct);
        let options = preview_options(engine, dns, mode, rules);
        let geo = MissingGeo;
        let context = CompilationContext {
            geo: &geo,
            shoes_geo: &geo,
            connectivity_test_url: "",
            mirror_direct_hosts: &[],
        };
        match compile_configuration(engine, &options, Some(node), &context) {
            Ok(output) => {
                self.artifact = Some(output.artifact);
                self.diagnostics = output.diagnostics;
                self.status =
                    "Compiler produced a draft, not a usable mobile engine configuration.";
            }
            Err(error) => {
                self.diagnostics = error.diagnostics;
                self.status =
                    "Selected engine rejected this draft. No fallback or routing-mode change.";
            }
        }
    }
}

fn combined_rules(manual: &[TrafficRule], imported: &[TrafficRule]) -> Vec<TrafficRule> {
    let mut rules = Vec::new();
    for rule in manual.iter().chain(imported) {
        if !rules.contains(rule) {
            rules.push(rule.clone());
        }
    }
    rules
}

fn preview_options(
    engine: KernelKind,
    dns: DnsConfig,
    routing_mode: RoutingMode,
    rules: Vec<TrafficRule>,
) -> MihomoConfigOptions {
    // Full explicit construction: desktop defaults must not silently enable features.
    MihomoConfigOptions {
        mode: ProxyMode::None,
        mixed_port: 9980,
        controller_port: 9090,
        allow_lan: false,
        tun: TunConfig {
            stack: TunStack::System,
            dns_hijack: Vec::new(),
            auto_route: false,
            auto_detect_interface: false,
        },
        dns,
        routing_mode,
        rules,
        kernel_preference: match engine {
            KernelKind::Mihomo => KernelPreference::Mihomo,
            KernelKind::SingBox => KernelPreference::SingBox,
            KernelKind::Xray => KernelPreference::Xray,
            KernelKind::Shoes => KernelPreference::Shoes,
            KernelKind::Meow => KernelPreference::Meow,
            KernelKind::VpCore => KernelPreference::VpCore,
        },
        controller_secret: String::new(),
        url_test: false,
        auto_strategy: AutoRoutingStrategy::UrlTest,
        url_test_size: 0,
        terminal_proxy: false,
        url_test_nodes: Vec::new(),
        smart_routing: false,
        override_yaml: None,
        subscription_rules_source_id: None,
        subscription_rules_yaml: None,
        subscription_groups_yaml: None,
        port_mappings: Vec::new(),
        port_mapping_nodes: Vec::new(),
        client_fingerprint: None,
        block_webrtc: false,
        block_trackers: false,
        udp_port_hopping: false,
        hop_ports: String::new(),
        selected_node_id: None,
        selected_node_name: None,
    }
}

#[cfg(test)]
mod tests;
