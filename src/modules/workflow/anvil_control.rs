//! Anvil control commands (impersonate, mine, snapshot, revert)

use crate::core::{Action, NotifyLevel};
use super::WorkflowResult;

/// Impersonate an account
pub fn impersonate(address: Option<String>) -> Action {
    let Some(addr) = address else {
        return Action::Notify("Usage: :impersonate <address>".into(), NotifyLevel::Warn);
    };

    WorkflowResult::new("Anvil Impersonate")
        .add("address", addr.trim())
        .add("status", "RPC call pending - async integration needed")
        .into_action()
}

/// Mine blocks
pub fn mine(count: Option<String>) -> Action {
    let blocks = count
        .and_then(|s| s.trim().parse::<u64>().ok())
        .unwrap_or(1);

    WorkflowResult::new("Anvil Mine")
        .add("blocks", blocks.to_string())
        .add("status", "RPC call pending - async integration needed")
        .into_action()
}

/// Create snapshot
pub fn snapshot() -> Action {
    WorkflowResult::new("Anvil Snapshot")
        .add("status", "RPC call pending - async integration needed")
        .into_action()
}

/// Revert to snapshot
pub fn revert(snapshot_id: Option<String>) -> Action {
    let Some(id) = snapshot_id else {
        return Action::Notify("Usage: :revert <snapshot_id>".into(), NotifyLevel::Warn);
    };

    WorkflowResult::new("Anvil Revert")
        .add("snapshot_id", id.trim())
        .add("status", "RPC call pending - async integration needed")
        .into_action()
}
