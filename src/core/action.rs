//! Actions that modules can return to communicate with the app
#![allow(dead_code)]

/// Actions returned by modules to communicate state changes
#[derive(Debug, Clone)]
pub enum Action {
    /// No action needed
    None,

    /// Navigate to a specific view
    Navigate(NavigateTarget),

    /// Copy text to clipboard context
    Copy(String),

    /// Show notification in status bar
    Notify(String, NotifyLevel),

    /// Open command palette with optional prefix
    OpenCommand(Option<String>),

    /// Close current overlay/popup
    CloseOverlay,

    /// Request quit
    Quit,
}

/// Navigation targets
#[derive(Debug, Clone)]
pub enum NavigateTarget {
    /// Go back to previous view
    Back,
    /// Go to dashboard
    Dashboard,
    /// Go to block explorer
    Blocks,
    /// Go to transaction explorer
    Transactions,
    /// Go to specific block
    Block(u64),
    /// Go to specific transaction
    Transaction(String),
    /// Go to specific address
    Address(String),
    /// Go to trace view for transaction
    Trace(String),
}

/// Notification levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotifyLevel {
    Info,
    Warn,
    Error,
}
