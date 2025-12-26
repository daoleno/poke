//! Operations monitoring commands

pub mod alerts;
pub mod health;
pub mod logs;
pub mod mempool;
pub mod metrics;
pub mod peers;
pub mod rpc_stats;

pub use alerts::AlertChecker;
pub use metrics::MetricsCollector;

use crate::core::{Action, NotifyLevel};

/// Result of an ops check
pub struct OpsResult {
    pub title: String,
    pub items: Vec<OpsItem>,
}

pub struct OpsItem {
    pub label: String,
    pub value: String,
    pub status: OpsStatus,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum OpsStatus {
    Ok,
    Warning,
    Error,
    Unknown,
}

impl OpsResult {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            items: Vec::new(),
        }
    }

    pub fn add(mut self, label: impl Into<String>, value: impl Into<String>, status: OpsStatus) -> Self {
        self.items.push(OpsItem {
            label: label.into(),
            value: value.into(),
            status,
        });
        self
    }

    pub fn into_action(self) -> Action {
        let msg = self.items
            .iter()
            .map(|item| {
                let icon = match item.status {
                    OpsStatus::Ok => "●",
                    OpsStatus::Warning => "◐",
                    OpsStatus::Error => "○",
                    OpsStatus::Unknown => "?",
                };
                format!("{} {}: {}", icon, item.label, item.value)
            })
            .collect::<Vec<_>>()
            .join(" | ");
        Action::Notify(format!("{} - {}", self.title, msg), NotifyLevel::Info)
    }
}
