pub mod meow;
pub mod mihomo;
pub mod shoes;
pub mod sing_box;
pub mod vp_core;
pub mod xray;

pub use meow::MeowCompiler;
pub use mihomo::MihomoCompiler;
pub use shoes::ShoesCompiler;
pub use sing_box::SingBoxCompiler;
pub use vp_core::VpCoreCompiler;
pub use xray::XrayCompiler;

use crate::compilation::ir::{IRMatcher, RoutingPlan};
use crate::compilation::KernelKind;
use anyhow::Result;
use std::collections::HashSet;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Warning,
    Error,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompilationDiagnostic {
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub rule_index: Option<usize>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct CompilerCapabilities {
    pub supports_tun: bool,
    pub supports_process_name: bool,
    pub supports_domain_keyword: bool,
    pub supports_proxy_groups: bool,
    pub supports_url_test: bool,
    pub supported_geosite_categories: Option<HashSet<String>>,
    pub supported_geoip_countries: Option<HashSet<String>>,
}

#[derive(Clone, Debug)]
pub enum CompiledArtifact {
    Yaml(String),
    Json(serde_json::Value),
}

impl CompiledArtifact {
    #[allow(dead_code)]
    pub fn to_string_pretty(&self) -> Result<String> {
        match self {
            CompiledArtifact::Yaml(yaml) => Ok(yaml.clone()),
            CompiledArtifact::Json(json) => Ok(serde_json::to_string_pretty(json)?),
        }
    }

    pub fn as_json_value(&self) -> Result<serde_json::Value> {
        match self {
            CompiledArtifact::Json(val) => Ok(val.clone()),
            CompiledArtifact::Yaml(yaml) => Ok(serde_yaml::from_str(yaml)?),
        }
    }
}

#[allow(dead_code)]
pub trait BackendCompiler: Send + Sync {
    fn kernel(&self) -> KernelKind;
    fn capabilities(&self) -> &CompilerCapabilities;

    /// 校验 RoutingPlan 是否有当前内核无法支持的特性，返回诊断信息列表
    fn validate(&self, plan: &RoutingPlan) -> Vec<CompilationDiagnostic> {
        let mut diagnostics = Vec::new();
        let caps = self.capabilities();

        // 校验 TUN 支持
        if plan.inbounds.mode.has_tun() && !caps.supports_tun {
            diagnostics.push(CompilationDiagnostic {
                severity: DiagnosticSeverity::Error,
                message: format!(
                    "内核 {} 不支持 TUN 虚拟网卡模式，请切换至系统代理模式或选择 Mihomo / Sing-box",
                    self.kernel().display_name()
                ),
                rule_index: None,
            });
        }

        // 校验 URL-Test 自动测速组
        if plan.outbounds.url_test && !caps.supports_url_test {
            diagnostics.push(CompilationDiagnostic {
                severity: DiagnosticSeverity::Warning,
                message: format!(
                    "内核 {} 不支持内核内自动选路 (url-test)，将回退至主节点",
                    self.kernel().display_name()
                ),
                rule_index: None,
            });
        }

        // 遍历规则检查匹配器能力
        for (idx, rule) in plan.rules.iter().enumerate() {
            for matcher in &rule.matchers {
                match matcher {
                    IRMatcher::ProcessName(_) if !caps.supports_process_name => {
                        diagnostics.push(CompilationDiagnostic {
                            severity: DiagnosticSeverity::Error,
                            message: format!(
                                "内核 {} 不支持基于进程名的分流规则",
                                self.kernel().display_name()
                            ),
                            rule_index: Some(idx),
                        });
                    }
                    IRMatcher::DomainKeyword(_) if !caps.supports_domain_keyword => {
                        diagnostics.push(CompilationDiagnostic {
                            severity: DiagnosticSeverity::Warning,
                            message: format!(
                                "内核 {} 不原生支持域名关键字分流，将尝试降级",
                                self.kernel().display_name()
                            ),
                            rule_index: Some(idx),
                        });
                    }
                    IRMatcher::GeoSite(category) => {
                        if let Some(ref allowed) = caps.supported_geosite_categories {
                            if !allowed.contains(category) {
                                diagnostics.push(CompilationDiagnostic {
                                    severity: DiagnosticSeverity::Warning,
                                    message: format!(
                                        "内核 {} 缺少预置规则集",
                                        self.kernel().display_name()
                                    ),
                                    rule_index: Some(idx),
                                });
                            }
                        }
                    }
                    IRMatcher::GeoIp(category) => {
                        if let Some(ref allowed) = caps.supported_geoip_countries {
                            if !allowed.contains(category) {
                                diagnostics.push(CompilationDiagnostic {
                                    severity: DiagnosticSeverity::Warning,
                                    message:
                                        "Backend does not provide the requested GeoIP rule set"
                                            .into(),
                                    rule_index: Some(idx),
                                });
                            }
                        }
                    }
                    IRMatcher::Quic | IRMatcher::Network(_)
                        if self.kernel() == KernelKind::Shoes =>
                    {
                        diagnostics.push(CompilationDiagnostic {
                            severity: DiagnosticSeverity::Error,
                            message: "Shoes cannot preserve this protocol-specific rule".into(),
                            rule_index: Some(idx),
                        });
                    }
                    _ => {}
                }
            }
        }

        diagnostics
    }

    /// 将统一 RoutingPlan 编译为对应内核的目标配置工件
    fn compile(
        &self,
        plan: &RoutingPlan,
        context: &crate::compilation::CompilationContext<'_>,
        diagnostics: &mut Vec<CompilationDiagnostic>,
    ) -> Result<CompiledArtifact>;
}

/// 根据内核种类获取对应的 BackendCompiler 实例
#[allow(dead_code)]
pub fn get_backend_compiler(kernel: KernelKind) -> Box<dyn BackendCompiler> {
    match kernel {
        KernelKind::Mihomo => Box::new(MihomoCompiler::new()),
        KernelKind::SingBox => Box::new(SingBoxCompiler::new()),
        KernelKind::Xray => Box::new(XrayCompiler::new()),
        KernelKind::Shoes => Box::new(ShoesCompiler::new()),
        KernelKind::Meow => Box::new(MeowCompiler),
        KernelKind::VpCore => Box::new(VpCoreCompiler::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compilation::ir::{DnsPlan, InboundPlan, OutboundPlan};

    // Avoid lowering and providers here: validation operates on explicit data only.
    fn empty_plan() -> RoutingPlan {
        let options = crate::compilation::options::MihomoConfigOptions::default();
        RoutingPlan {
            inbounds: InboundPlan {
                mixed_port: 9980,
                controller_port: 9090,
                controller_secret: String::new(),
                allow_lan: false,
                mode: options.mode,
                tun: options.tun,
                port_mappings: vec![],
            },
            outbounds: OutboundPlan {
                primary_node: None,
                url_test: false,
                auto_strategy: options.auto_strategy,
                url_test_size: 2,
                url_test_nodes: vec![],
                smart_routing: false,
                override_yaml: None,
                subscription_rules_yaml: None,
                subscription_groups_yaml: None,
                port_mapping_nodes: vec![],
                client_fingerprint: None,
                udp_port_hopping: false,
                hop_ports: String::new(),
            },
            dns: DnsPlan {
                enhanced_mode: options.dns.enhanced_mode,
                nameservers: vec![],
                default_nameservers: vec![],
                china_nameservers: vec![],
                overseas_nameservers: vec![],
                proxy_server_nameservers: vec![],
                fake_ip_range: String::new(),
                fake_ip_filter: vec![],
                respect_rules: false,
            },
            rules: vec![],
            default_action: crate::compilation::ir::IRAction::Direct,
            routing_mode: options.routing_mode,
        }
    }

    #[test]
    fn validator_rejects_both_mode_for_non_tun_backends() {
        let mut plan = empty_plan();
        plan.inbounds.mode = crate::compilation::options::ProxyMode::Both;
        for backend in [KernelKind::Xray, KernelKind::Shoes] {
            assert!(get_backend_compiler(backend)
                .validate(&plan)
                .iter()
                .any(|d| d.severity == DiagnosticSeverity::Error));
        }
    }

    #[test]
    fn diagnostic_text_does_not_echo_matcher_payload() {
        let mut plan = empty_plan();
        plan.rules.push(crate::compilation::ir::IRRule::new(
            IRMatcher::ProcessName("sensitive-process-value".into()),
            crate::compilation::ir::IRAction::Reject,
            crate::compilation::ir::RuleSource::User,
        ));
        let diagnostics = XrayCompiler::new().validate(&plan);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule_index, Some(0));
        assert!(!diagnostics[0].message.contains("sensitive-process-value"));
    }
}
