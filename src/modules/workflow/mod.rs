//! Workflow commands for local development

pub mod anvil;
pub mod anvil_control;
pub mod nodes;

pub use anvil::AnvilConfig;
pub use nodes::NodeConnection;

use crate::core::{Action, NotifyLevel};

/// Result of a workflow operation
pub struct WorkflowResult {
    pub title: String,
    pub items: Vec<(String, String)>,
}

impl WorkflowResult {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            items: Vec::new(),
        }
    }

    pub fn add(mut self, label: impl Into<String>, value: impl Into<String>) -> Self {
        self.items.push((label.into(), value.into()));
        self
    }

    pub fn into_action(self) -> Action {
        let msg = self.items
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<_>>()
            .join(" | ");
        Action::Notify(format!("{} - {}", self.title, msg), NotifyLevel::Info)
    }
}
