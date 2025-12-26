//! Node management (connect, list, switch)

use crate::core::{Action, NotifyLevel};
use super::WorkflowResult;

/// Node connection info
#[derive(Clone, Debug)]
pub struct NodeConnection {
    pub name: String,
    pub url: String,
    pub active: bool,
}

/// Connect to a node
pub fn connect(url: Option<String>) -> Action {
    let Some(url_str) = url else {
        return Action::Notify("Usage: :connect <url>".into(), NotifyLevel::Warn);
    };

    let url_trimmed = url_str.trim();
    if url_trimmed.is_empty() {
        return Action::Notify("Usage: :connect <url>".into(), NotifyLevel::Warn);
    }

    // Validate URL format
    if !url_trimmed.starts_with("http://") && !url_trimmed.starts_with("https://") && !url_trimmed.starts_with("ws://") && !url_trimmed.starts_with("wss://") {
        return Action::Notify("URL must start with http://, https://, ws://, or wss://".into(), NotifyLevel::Error);
    }

    WorkflowResult::new("Connect Node")
        .add("url", url_trimmed)
        .add("status", "Connection management in app state")
        .into_action()
}

/// List all nodes
pub fn list_nodes() -> Action {
    WorkflowResult::new("List Nodes")
        .add("status", "Node list in app state")
        .into_action()
}

/// Switch active node
pub fn switch_node(name: Option<String>) -> Action {
    let Some(node_name) = name else {
        return Action::Notify("Usage: :switch <name>".into(), NotifyLevel::Warn);
    };

    WorkflowResult::new("Switch Node")
        .add("node", node_name.trim())
        .add("status", "Node switching in app state")
        .into_action()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_connection() {
        let node = NodeConnection {
            name: "local".to_string(),
            url: "http://localhost:8545".to_string(),
            active: true,
        };
        assert_eq!(node.name, "local");
        assert!(node.active);
    }
}
