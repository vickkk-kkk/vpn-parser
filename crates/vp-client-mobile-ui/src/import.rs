//! Memory-only subscription import. No clock, randomness, filesystem or network.
use std::collections::HashSet;
use vp_client_core::{
    config::TrafficRule,
    geo::{CategoryLookup, GeoKind, GeoProvider, GeoUnavailable},
    models::Node,
    parser::{
        extract_subscription_rules, node_fingerprint, parse_subscription, NodeMetadataProvider,
    },
};

pub const MAX_INPUT_BYTES: usize = 128 * 1024;
pub const MAX_NODES: usize = 512;
pub const MAX_RULES: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportError {
    Empty,
    TooLarge,
    NoNodes,
    TooManyNodes,
    TooManyRules,
    NoPreview,
}
impl ImportError {
    pub fn message(self) -> &'static str {
        match self {
            Self::Empty => "Paste subscription content first.",
            Self::TooLarge => "Input exceeds 128 KiB; nothing was imported.",
            Self::NoNodes => {
                "No supported nodes recognized. Paste content, not a subscription URL."
            }
            Self::TooManyNodes => "Node limit exceeded (512); nothing was imported.",
            Self::TooManyRules => "Rule draft limit exceeded (512); nothing was imported.",
            Self::NoPreview => {
                "Preview is missing or out of date. Preview again before confirming."
            }
        }
    }
}

/// Intentionally excludes the untrusted category string and raw parser input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeoWarning {
    pub rule_index: usize,
    pub kind: GeoKind,
    pub lookup: CategoryLookup,
}
impl GeoWarning {
    pub fn message(self) -> &'static str {
        match self.lookup {
            CategoryLookup::Absent => "Category absent from supplied inventory; rule omitted.",
            CategoryLookup::Unavailable(GeoUnavailable::Missing) => {
                "Geo inventory missing; rule kept unverified."
            }
            CategoryLookup::Unavailable(GeoUnavailable::Unreadable) => {
                "Geo inventory unreadable; rule kept unverified."
            }
            CategoryLookup::Unavailable(GeoUnavailable::Malformed) => {
                "Geo inventory malformed; rule kept unverified."
            }
            CategoryLookup::Present => "Category found in supplied inventory.",
        }
    }
}

pub struct Preview {
    nodes: Vec<Node>,
    rules: Vec<TrafficRule>,
    warnings: Vec<GeoWarning>,
}
impl Preview {
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }
    pub fn rules(&self) -> &[TrafficRule] {
        &self.rules
    }
    pub fn warnings(&self) -> &[GeoWarning] {
        &self.warnings
    }
}

#[derive(Default)]
pub struct ImportState {
    input: String,
    preview: Option<Preview>,
    nodes: Vec<Node>,
    rules: Vec<TrafficRule>,
}
impl ImportState {
    /// Every edit invalidates preview, including edits that restore prior content.
    pub fn set_input(&mut self, input: &str) -> Result<(), ImportError> {
        self.preview = None;
        self.input.clear();
        if input.len() > MAX_INPUT_BYTES {
            return Err(ImportError::TooLarge);
        }
        self.input.push_str(input);
        Ok(())
    }
    pub fn preview(&self) -> Option<&Preview> {
        self.preview.as_ref()
    }
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }
    pub fn rules(&self) -> &[TrafficRule] {
        &self.rules
    }
    pub fn cancel(&mut self) {
        self.input.clear();
        self.preview = None;
    }

    pub fn prepare(
        &mut self,
        metadata: &mut impl NodeMetadataProvider,
        geo: &impl GeoProvider,
    ) -> Result<(), ImportError> {
        self.preview = None;
        if self.input.trim().is_empty() {
            return Err(ImportError::Empty);
        }
        let nodes = parse_subscription(&self.input, None, metadata);
        if nodes.is_empty() {
            return Err(ImportError::NoNodes);
        }
        if nodes.len() > MAX_NODES {
            return Err(ImportError::TooManyNodes);
        }
        let mut diagnostics = Vec::new();
        let rules = extract_subscription_rules(&self.input, geo, &mut diagnostics);
        if rules.len() > MAX_RULES {
            return Err(ImportError::TooManyRules);
        }
        let warnings = diagnostics
            .into_iter()
            .map(|d| GeoWarning {
                rule_index: d.rule_index,
                kind: d.kind,
                lookup: d.lookup,
            })
            .collect();
        self.preview = Some(Preview {
            nodes,
            rules,
            warnings,
        });
        Ok(())
    }

    /// Atomically append unique nodes/rules; consuming preview prevents repeat commits.
    /// Existing node identity and unknown latency are preserved, no connection is made.
    pub fn confirm(&mut self) -> Result<usize, ImportError> {
        let preview = self.preview.take().ok_or(ImportError::NoPreview)?;
        let mut fingerprints: HashSet<String> = self.nodes.iter().map(node_fingerprint).collect();
        let additions: Vec<Node> = preview
            .nodes
            .into_iter()
            .filter(|node| fingerprints.insert(node_fingerprint(node)))
            .collect();
        let mut rules = self.rules.clone();
        for rule in preview.rules {
            if !rules.contains(&rule) {
                rules.push(rule);
            }
        }
        if self.nodes.len() + additions.len() > MAX_NODES {
            return Err(ImportError::TooManyNodes);
        }
        if rules.len() > MAX_RULES {
            return Err(ImportError::TooManyRules);
        }
        let added = additions.len();
        self.nodes.extend(additions);
        self.rules = rules;
        self.input.clear();
        Ok(added)
    }
}

/// UI-only summaries must not copy raw URIs/config. Strip controls and bidi marks,
/// cap length, and render as plain text (never hyperlinks or markup).
pub fn display_text(value: &str) -> String {
    value
        .chars()
        .filter(|c| {
            !c.is_control()
                && !matches!(*c,
        '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        })
        .take(96)
        .collect()
}

#[cfg(test)]
mod tests;
