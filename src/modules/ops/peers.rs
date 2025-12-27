//! Peer information display

use super::{OpsResult, OpsStatus};
use crate::core::Action;

/// Simple peers display with just count
pub fn peers_count(count: u32) -> Action {
    let status = if count == 0 {
        OpsStatus::Error
    } else if count < 3 {
        OpsStatus::Warning
    } else {
        OpsStatus::Ok
    };

    OpsResult::new("Peers")
        .add("connected", count.to_string(), status)
        .into_action()
}
