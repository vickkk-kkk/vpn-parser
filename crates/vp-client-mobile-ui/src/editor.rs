use vp_client_core::config::{DnsConfig, RoutingMode, TrafficRule};

/// All edits remain local until a real adapter and persistence layer are bound.
pub(crate) struct Editor {
    pub dns: DnsConfig,
    pub mode: RoutingMode,
    pub nameservers: String,
    pub rules: Vec<TrafficRule>,
}
impl Default for Editor {
    fn default() -> Self {
        let dns = DnsConfig::default();
        Self {
            nameservers: dns.nameservers.join("\n"),
            dns,
            mode: RoutingMode::Rule,
            rules: Vec::new(),
        }
    }
}
impl Editor {
    pub fn validate_dns_draft(&mut self) -> bool {
        let mut candidate = self.dns.clone();
        candidate.nameservers = self
            .nameservers
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_owned)
            .collect();
        if candidate.validate().is_err() {
            return false;
        }
        self.dns = candidate;
        true
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_dns_draft_preserves_last_valid_configuration() {
        let mut editor = Editor::default();
        let original = editor.dns.clone();
        editor.nameservers = "not a resolver".into();
        assert!(!editor.validate_dns_draft());
        assert_eq!(editor.dns, original);
    }
    #[test]
    fn valid_dns_draft_uses_shared_validator() {
        let mut editor = Editor::default();
        editor.nameservers = "  192.0.2.1 \n\n https://resolver.example.invalid/dns-query ".into();
        assert!(editor.validate_dns_draft());
        assert_eq!(editor.dns.nameservers.len(), 2);
        assert_eq!(editor.mode, RoutingMode::Rule);
        assert!(editor.rules.is_empty());
    }
}
