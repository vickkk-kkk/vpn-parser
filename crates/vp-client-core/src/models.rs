//! Persisted node DTOs; no storage, clock or engine operations.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A parsed proxy node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub source_id: Option<String>,
    pub protocol: String,
    pub name: String,
    pub server: String,
    pub port: u16,
    /// Original share URI or raw config fragment.
    pub raw: String,
    /// Normalized JSON config for API consumers.
    pub config: serde_json::Value,
    pub country: Option<String>,
    /// TCP connect latency in milliseconds. None = not tested.
    pub latency_ms: Option<i64>,
    /// true=ok, false=timeout/fail, None=not tested
    pub latency_ok: Option<bool>,
    /// Download throughput in KB/s when measurable.
    pub download_kbps: Option<f64>,
    pub latency_kernel: Option<String>,
    pub latency_test_kind: Option<String>,
    pub latency_tested_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSummary {
    pub id: String,
    pub source_id: Option<String>,
    pub protocol: String,
    pub name: String,
    pub server: String,
    pub port: u16,
    pub country: Option<String>,
    pub latency_ms: Option<i64>,
    pub latency_ok: Option<bool>,
    pub download_kbps: Option<f64>,
    pub latency_kernel: Option<String>,
    pub latency_test_kind: Option<String>,
    pub latency_tested_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeGroupSummary {
    pub source_id: Option<String>,
    pub source_name: String,
    pub total: i64,
    pub alive: i64,
}

