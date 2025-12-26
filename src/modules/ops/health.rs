//! Node health check

use super::{OpsResult, OpsStatus};
use crate::core::Action;

/// Health check result from RPC
pub struct HealthData {
    pub syncing: bool,
    pub peer_count: u32,
    pub rpc_latency_ms: Option<u64>,
    pub chain_id: Option<u64>,
    pub block_number: Option<u64>,
}

impl Default for HealthData {
    fn default() -> Self {
        Self {
            syncing: false,
            peer_count: 0,
            rpc_latency_ms: None,
            chain_id: None,
            block_number: None,
        }
    }
}

/// Generate health check action from data
pub fn health(data: &HealthData) -> Action {
    let sync_status = if data.syncing {
        OpsStatus::Warning
    } else {
        OpsStatus::Ok
    };
    let sync_text = if data.syncing { "syncing" } else { "synced" };

    let peer_status = if data.peer_count == 0 {
        OpsStatus::Error
    } else if data.peer_count < 3 {
        OpsStatus::Warning
    } else {
        OpsStatus::Ok
    };

    let rpc_status = match data.rpc_latency_ms {
        Some(ms) if ms < 100 => OpsStatus::Ok,
        Some(ms) if ms < 500 => OpsStatus::Warning,
        Some(_) => OpsStatus::Error,
        None => OpsStatus::Unknown,
    };
    let rpc_text = data.rpc_latency_ms
        .map(|ms| format!("{}ms", ms))
        .unwrap_or_else(|| "N/A".to_string());

    let block_text = data.block_number
        .map(|n| format!("#{}", n))
        .unwrap_or_else(|| "N/A".to_string());

    let result = OpsResult::new("Health")
        .add("sync", sync_text, sync_status)
        .add("peers", data.peer_count.to_string(), peer_status)
        .add("rpc", rpc_text, rpc_status)
        .add("block", block_text, OpsStatus::Ok);

    result.into_action()
}

/// Create a health action from current app state
pub fn health_from_state(
    syncing: bool,
    peer_count: u32,
    rpc_latency_ms: Option<u64>,
    block_number: Option<u64>,
) -> Action {
    let data = HealthData {
        syncing,
        peer_count,
        rpc_latency_ms,
        chain_id: None,
        block_number,
    };
    health(&data)
}
