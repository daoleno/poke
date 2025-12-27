//! Metrics tracking (placeholder)

use crate::core::{Action, NotifyLevel};

/// Placeholder for metrics command
pub fn metrics_unavailable() -> Action {
    Action::Notify(
        "Metrics: collection not yet implemented".into(),
        NotifyLevel::Warn,
    )
}
