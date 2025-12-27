//! Transaction pool (mempool) status

use crate::core::{Action, NotifyLevel};

/// Placeholder when mempool not available
pub fn mempool_unavailable() -> Action {
    Action::Notify(
        "Mempool: requires txpool_status RPC (not available on all nodes)".into(),
        NotifyLevel::Warn,
    )
}
