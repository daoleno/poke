//! Transaction pool (mempool) status

use super::{OpsResult, OpsStatus};
use crate::core::{Action, NotifyLevel};

/// Mempool statistics
#[derive(Default, Clone)]
pub struct MempoolStats {
    pub pending: u64,
    pub queued: u64,
    pub local_pending: u64,
}

/// Display mempool status
pub fn mempool(stats: &MempoolStats) -> Action {
    let total = stats.pending + stats.queued;

    let pending_status = if stats.pending > 10000 {
        OpsStatus::Warning
    } else {
        OpsStatus::Ok
    };

    OpsResult::new("Mempool")
        .add("pending", stats.pending.to_string(), pending_status)
        .add("queued", stats.queued.to_string(), OpsStatus::Ok)
        .add("total", total.to_string(), OpsStatus::Ok)
        .into_action()
}

/// Placeholder when mempool not available
pub fn mempool_unavailable() -> Action {
    Action::Notify(
        "Mempool: requires txpool_status RPC (not available on all nodes)".into(),
        NotifyLevel::Warn,
    )
}
