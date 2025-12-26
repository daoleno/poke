//! Toolkit commands for data processing and conversion

pub mod convert;
pub mod hash;
pub mod hex;

use crate::core::{Action, NotifyLevel};

/// Result of a toolkit operation
pub struct ToolResult {
    pub title: String,
    pub content: Vec<(String, String)>, // (label, value) pairs
}

impl ToolResult {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            content: Vec::new(),
        }
    }

    pub fn add(mut self, label: impl Into<String>, value: impl Into<String>) -> Self {
        self.content.push((label.into(), value.into()));
        self
    }

    pub fn into_action(self) -> Action {
        // For now, format as status message; later will be popup
        let msg = self
            .content
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<_>>()
            .join(" | ");
        Action::Notify(format!("{} - {}", self.title, msg), NotifyLevel::Info)
    }
}
