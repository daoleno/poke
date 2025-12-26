//! UI Modules
//!
//! Each module implements the Module trait and handles its own:
//! - Key input processing
//! - Command handling
//! - Rendering
//!
//! Modules:
//! - dashboard: Main overview with nodes, activity, inspector panels
//! - explorer: Block, transaction, address browsing
//! - toolkit: Command-driven tools (encode, decode, convert, etc.)
//! - ops: Operations monitoring (health, peers, logs)
//! - workflow: Workflows (anvil, watch, call)

pub mod toolkit;

// Modules will be added incrementally:
// pub mod dashboard;
// pub mod explorer;
// pub mod ops;
// pub mod workflow;
