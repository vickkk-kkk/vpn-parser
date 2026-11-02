//! Legacy desktop signatures over the explicitly parameterized core lowerer.
use super::types::{IRRule, RoutingPlan, RuleSource};
use crate::models::Node;
use crate::proxy::MihomoConfigOptions;
pub use vp_client_core::compilation::ir::lower::DEFAULT_FAKE_IP_FILTERS;
pub fn lower_options_to_ir(
    options: &MihomoConfigOptions,
    primary_node: Option<&Node>,
) -> RoutingPlan {
    crate::proxy::compiler::with_desktop_context(|context| {
        vp_client_core::compilation::ir::lower::lower_options_to_ir(
            &options.into(),
            primary_node,
            context,
            &mut Vec::new(),
        )
    })
}
pub fn parse_clash_rules_yaml_to_ir(rules_yaml: Option<&str>, source: RuleSource) -> Vec<IRRule> {
    crate::proxy::compiler::with_desktop_context(|context| {
        vp_client_core::compilation::ir::lower::parse_clash_rules_yaml_to_ir(
            rules_yaml,
            source,
            context,
            &mut Vec::new(),
        )
    })
}
pub fn parse_override_rules_to_ir(override_yaml: Option<&str>) -> Vec<IRRule> {
    crate::proxy::compiler::with_desktop_context(|context| {
        vp_client_core::compilation::ir::lower::parse_override_rules_to_ir(
            override_yaml,
            context,
            &mut Vec::new(),
        )
    })
}
