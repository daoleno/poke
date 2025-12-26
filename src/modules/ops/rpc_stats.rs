//! RPC call statistics

use super::{OpsResult, OpsStatus};
use crate::core::Action;

/// RPC statistics
#[derive(Default, Clone)]
pub struct RpcStats {
    pub total_calls: u64,
    pub failed_calls: u64,
    pub avg_latency_ms: u64,
    pub max_latency_ms: u64,
    pub calls_per_second: f64,
}

/// Display RPC statistics
pub fn rpc_stats(stats: &RpcStats) -> Action {
    let latency_status = if stats.avg_latency_ms < 100 {
        OpsStatus::Ok
    } else if stats.avg_latency_ms < 500 {
        OpsStatus::Warning
    } else {
        OpsStatus::Error
    };

    let error_rate = if stats.total_calls > 0 {
        (stats.failed_calls as f64 / stats.total_calls as f64) * 100.0
    } else {
        0.0
    };

    let error_status = if error_rate < 1.0 {
        OpsStatus::Ok
    } else if error_rate < 5.0 {
        OpsStatus::Warning
    } else {
        OpsStatus::Error
    };

    OpsResult::new("RPC Stats")
        .add("calls", stats.total_calls.to_string(), OpsStatus::Ok)
        .add("avg", format!("{}ms", stats.avg_latency_ms), latency_status)
        .add("max", format!("{}ms", stats.max_latency_ms), OpsStatus::Ok)
        .add("errors", format!("{:.1}%", error_rate), error_status)
        .into_action()
}

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
