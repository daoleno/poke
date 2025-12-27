//! Shared context passed to modules
#![allow(dead_code)]

use std::collections::BTreeMap;

/// Currently selected item in the UI
#[derive(Debug, Clone)]
pub enum Selected {
    None,
    Block(u64),
    Transaction(String),
    Address(String),
    TraceFrame { tx: String, index: usize },
}

/// Shared context available to all modules
#[derive(Debug)]
pub struct Context {
    /// Currently selected item
    pub selected: Selected,

    /// Clipboard content for copy/paste between tools
    pub clipboard: Option<String>,

    /// User-defined labels for addresses
    pub labels: BTreeMap<String, String>,

    /// Current RPC endpoint display string
    pub rpc_endpoint: String,

    /// Current node type (anvil, geth, reth, etc.)
    pub node_kind: String,

    /// Whether the UI is paused
    pub paused: bool,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            selected: Selected::None,
            clipboard: None,
            labels: BTreeMap::new(),
            rpc_endpoint: String::new(),
            node_kind: String::new(),
            paused: false,
        }
    }
}

impl Context {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get label for an address if it exists
    pub fn label_for(&self, address: &str) -> Option<&str> {
        self.labels.get(&address.to_lowercase()).map(|s| s.as_str())
    }

    /// Set clipboard content
    pub fn set_clipboard(&mut self, content: String) {
        self.clipboard = Some(content);
    }

    /// Get clipboard content
    pub fn get_clipboard(&self) -> Option<&str> {
        self.clipboard.as_deref()
    }
}
