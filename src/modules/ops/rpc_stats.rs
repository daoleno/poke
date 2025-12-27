//! RPC call statistics

use super::{OpsResult, OpsStatus};
use crate::core::Action;

/// Simple RPC stats from current state
pub fn rpc_stats_simple(latency_ms: Option<u64>, endpoint: &str) -> Action {
    let latency_status = match latency_ms {
        Some(ms) if ms < 100 => OpsStatus::Ok,
        Some(ms) if ms < 500 => OpsStatus::Warning,
        Some(_) => OpsStatus::Error,
        None => OpsStatus::Unknown,
    };

    let latency_text = latency_ms
        .map(|ms| format!("{}ms", ms))
        .unwrap_or_else(|| "N/A".to_string());

    let short_endpoint = if endpoint.len() > 30 {
        format!("{}...", &endpoint[..27])
    } else {
        endpoint.to_string()
    };

    OpsResult::new("RPC Stats")
        .add("endpoint", short_endpoint, OpsStatus::Ok)
        .add("latency", latency_text, latency_status)
        .into_action()
}
