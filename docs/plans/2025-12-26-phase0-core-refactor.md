# Phase 0: Core Architecture Refactor

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Refactor Poke from monolithic app.rs to modular architecture with command system

**Architecture:**
- Extract core types to `src/core/` (context, command, action)
- Create `Module` trait for extensible modules
- Keep existing functionality working throughout refactor
- Introduce command system (`:command`) for toolkit

**Tech Stack:** Rust, Ratatui, Crossterm, Tokio

---

## Overview

Current state:
- `app.rs`: 2399 lines, ~50 fields in App struct, handles all state and logic
- `ui/mod.rs`: 1338 lines, handles all rendering
- Tightly coupled, hard to extend

Target state:
- `core/`: Context, Command parser, Action enum, Module trait
- `modules/`: Independent modules implementing Module trait
- Existing functionality preserved, new command system added

## Task 1: Create Core Directory Structure

**Files:**
- Create: `src/core/mod.rs`
- Create: `src/core/action.rs`
- Create: `src/core/context.rs`
- Modify: `src/main.rs:1-10` (add mod core)

**Step 1: Create core module files**

Create `src/core/mod.rs`:
```rust
pub mod action;
pub mod context;

pub use action::Action;
pub use context::Context;
```

**Step 2: Create action.rs**

Create `src/core/action.rs`:
```rust
//! Actions that modules can return to communicate with the app

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
```

**Step 3: Create context.rs**

Create `src/core/context.rs`:
```rust
//! Shared context passed to modules

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
```

**Step 4: Add core module to main.rs**

Modify `src/main.rs` - add after line 6:
```rust
mod core;
```

**Step 5: Verify compilation**

Run: `cargo check`
Expected: Compiles with no errors

**Step 6: Commit**

```bash
git add src/core/
git add src/main.rs
git commit -m "feat(core): add core module with Action and Context types"
```

---

## Task 2: Create Command Parser

**Files:**
- Create: `src/core/command.rs`
- Modify: `src/core/mod.rs`

**Step 1: Create command.rs**

Create `src/core/command.rs`:
```rust
//! Command parser for the : command system

/// Parsed command from user input
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    // Navigation commands
    Blocks,
    Transactions,
    Address(String),
    Trace(String),

    // Toolkit commands - data processing
    Encode(Option<String>),
    Decode(Option<String>),
    Hash(Option<String>),
    Hex(Option<String>),

    // Toolkit commands - query/convert
    Selector(Option<String>),
    FourByte(Option<String>),
    Convert(Option<String>),
    Timestamp(Option<String>),

    // Toolkit commands - contract interaction
    Call(Option<String>),
    Gas(Option<String>),
    Slot(Option<String>),

    // Toolkit commands - address calculation
    Create(Option<String>),
    Create2(Option<String>),
    Checksum(Option<String>),

    // Ops commands
    Health,
    Peers,
    Logs,
    Mempool,
    RpcStats,

    // Node management
    Connect(String),
    Anvil(Vec<String>),
    Impersonate(String),
    Mine(Option<u64>),
    Snapshot,
    Revert(Option<String>),

    // Unknown command
    Unknown(String),
}

/// Parse a command string (without the leading :)
pub fn parse_command(input: &str) -> Command {
    let input = input.trim();
    let mut parts = input.splitn(2, ' ');
    let cmd = parts.next().unwrap_or("");
    let args = parts.next().map(|s| s.trim().to_string());

    match cmd.to_lowercase().as_str() {
        // Navigation
        "blocks" | "blk" => Command::Blocks,
        "transactions" | "txs" | "tx" => Command::Transactions,
        "address" | "addr" => {
            if let Some(addr) = args {
                Command::Address(addr)
            } else {
                Command::Unknown(input.to_string())
            }
        }
        "trace" => {
            if let Some(hash) = args {
                Command::Trace(hash)
            } else {
                Command::Unknown(input.to_string())
            }
        }

        // Toolkit - data processing
        "encode" | "enc" => Command::Encode(args),
        "decode" | "dec" => Command::Decode(args),
        "hash" | "keccak" | "keccak256" => Command::Hash(args),
        "hex" => Command::Hex(args),

        // Toolkit - query/convert
        "selector" | "sig" => Command::Selector(args),
        "4byte" | "fourbyte" => Command::FourByte(args),
        "convert" | "conv" => Command::Convert(args),
        "timestamp" | "time" | "ts" => Command::Timestamp(args),

        // Toolkit - contract
        "call" => Command::Call(args),
        "gas" => Command::Gas(args),
        "slot" => Command::Slot(args),

        // Toolkit - address
        "create" => Command::Create(args),
        "create2" => Command::Create2(args),
        "checksum" | "check" => Command::Checksum(args),

        // Ops
        "health" => Command::Health,
        "peers" => Command::Peers,
        "logs" | "log" => Command::Logs,
        "mempool" | "pool" => Command::Mempool,
        "rpc-stats" | "rpcstats" | "stats" => Command::RpcStats,

        // Node management
        "connect" | "conn" => {
            if let Some(url) = args {
                Command::Connect(url)
            } else {
                Command::Unknown(input.to_string())
            }
        }
        "anvil" => {
            let anvil_args = args
                .map(|s| s.split_whitespace().map(|s| s.to_string()).collect())
                .unwrap_or_default();
            Command::Anvil(anvil_args)
        }
        "impersonate" | "imp" => {
            if let Some(addr) = args {
                Command::Impersonate(addr)
            } else {
                Command::Unknown(input.to_string())
            }
        }
        "mine" => {
            let n = args.and_then(|s| s.parse().ok());
            Command::Mine(n)
        }
        "snapshot" | "snap" => Command::Snapshot,
        "revert" => Command::Revert(args),

        _ => Command::Unknown(input.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_navigation_commands() {
        assert_eq!(parse_command("blocks"), Command::Blocks);
        assert_eq!(parse_command("blk"), Command::Blocks);
        assert_eq!(parse_command("txs"), Command::Transactions);
        assert_eq!(
            parse_command("address 0x1234"),
            Command::Address("0x1234".to_string())
        );
    }

    #[test]
    fn test_parse_toolkit_commands() {
        assert_eq!(parse_command("encode"), Command::Encode(None));
        assert_eq!(
            parse_command("encode transfer(address,uint256)"),
            Command::Encode(Some("transfer(address,uint256)".to_string()))
        );
        assert_eq!(
            parse_command("decode 0xabcd"),
            Command::Decode(Some("0xabcd".to_string()))
        );
        assert_eq!(
            parse_command("convert 1.5 ether"),
            Command::Convert(Some("1.5 ether".to_string()))
        );
    }

    #[test]
    fn test_parse_ops_commands() {
        assert_eq!(parse_command("health"), Command::Health);
        assert_eq!(parse_command("peers"), Command::Peers);
        assert_eq!(parse_command("logs"), Command::Logs);
    }

    #[test]
    fn test_parse_anvil_commands() {
        assert_eq!(parse_command("anvil"), Command::Anvil(vec![]));
        assert_eq!(
            parse_command("anvil --fork mainnet"),
            Command::Anvil(vec!["--fork".to_string(), "mainnet".to_string()])
        );
        assert_eq!(parse_command("mine"), Command::Mine(None));
        assert_eq!(parse_command("mine 10"), Command::Mine(Some(10)));
    }

    #[test]
    fn test_parse_unknown() {
        assert_eq!(
            parse_command("notacommand"),
            Command::Unknown("notacommand".to_string())
        );
    }
}
```

**Step 2: Update core/mod.rs**

Modify `src/core/mod.rs`:
```rust
pub mod action;
pub mod command;
pub mod context;

pub use action::{Action, NavigateTarget, NotifyLevel};
pub use command::{parse_command, Command};
pub use context::{Context, Selected};
```

**Step 3: Run tests**

Run: `cargo test core::command`
Expected: All tests pass

**Step 4: Verify compilation**

Run: `cargo check`
Expected: Compiles with no errors

**Step 5: Commit**

```bash
git add src/core/
git commit -m "feat(core): add command parser with full command set"
```

---

## Task 3: Create Module Trait

**Files:**
- Create: `src/core/module.rs`
- Modify: `src/core/mod.rs`

**Step 1: Create module.rs**

Create `src/core/module.rs`:
```rust
//! Module trait for extensible UI components

use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;

use super::{Action, Command, Context};

/// Trait for UI modules that can handle input and render themselves
pub trait Module {
    /// Module identifier
    fn id(&self) -> &'static str;

    /// Handle keyboard input
    /// Returns an Action describing what should happen
    fn handle_key(&mut self, key: KeyEvent, ctx: &mut Context) -> Action;

    /// Handle a parsed command
    /// Returns an Action describing what should happen
    fn handle_command(&mut self, cmd: &Command, ctx: &mut Context) -> Action;

    /// Render the module
    fn render(&self, frame: &mut Frame, area: Rect, ctx: &Context);

    /// Whether this module wants focus
    fn focusable(&self) -> bool {
        true
    }

    /// Called when module gains focus
    fn on_focus(&mut self, _ctx: &mut Context) {}

    /// Called when module loses focus
    fn on_blur(&mut self, _ctx: &mut Context) {}

    /// Called on each tick for async updates
    fn tick(&mut self, _ctx: &mut Context) {}
}

/// A boxed module for dynamic dispatch
pub type BoxedModule = Box<dyn Module + Send>;
```

**Step 2: Update core/mod.rs**

Modify `src/core/mod.rs`:
```rust
pub mod action;
pub mod command;
pub mod context;
pub mod module;

pub use action::{Action, NavigateTarget, NotifyLevel};
pub use command::{parse_command, Command};
pub use context::{Context, Selected};
pub use module::{BoxedModule, Module};
```

**Step 3: Verify compilation**

Run: `cargo check`
Expected: Compiles with no errors

**Step 4: Commit**

```bash
git add src/core/
git commit -m "feat(core): add Module trait for extensible components"
```

---

## Task 4: Create Modules Directory with Placeholder

**Files:**
- Create: `src/modules/mod.rs`
- Modify: `src/main.rs`

**Step 1: Create modules directory and mod.rs**

Create `src/modules/mod.rs`:
```rust
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

// Modules will be added incrementally:
// pub mod dashboard;
// pub mod explorer;
// pub mod toolkit;
// pub mod ops;
// pub mod workflow;
```

**Step 2: Add modules to main.rs**

Modify `src/main.rs` - add after `mod core;`:
```rust
mod modules;
```

**Step 3: Verify compilation**

Run: `cargo check`
Expected: Compiles with no errors (empty module is fine)

**Step 4: Commit**

```bash
git add src/modules/
git add src/main.rs
git commit -m "feat(modules): add modules directory structure"
```

---

## Task 5: Integrate Context with App

**Files:**
- Modify: `src/app.rs:429-487` (add context field to App)
- Modify: `src/main.rs:81-93` (initialize context)

**Step 1: Add Context import to app.rs**

Add to top of `src/app.rs` after other imports:
```rust
use crate::core::Context;
```

**Step 2: Add context field to App struct**

In `src/app.rs`, add to App struct (around line 430, after `view_stack`):
```rust
    /// Shared context for modules
    pub ctx: Context,
```

**Step 3: Initialize context in App::new()**

In `src/app.rs`, in the `App::new()` function (around line 491), add after `view_stack: vec![View::Overview],`:
```rust
            ctx: Context::new(),
```

**Step 4: Sync context with app state**

Add a method to App to sync context (add after `App::new()`):
```rust
    /// Sync context with app state
    pub fn sync_context(&mut self) {
        self.ctx.labels = self.labels.clone();
        self.ctx.rpc_endpoint = self.rpc_endpoint.clone();
        self.ctx.node_kind = self.node_kind.clone();
        self.ctx.paused = self.paused;

        // Update selected based on current view and selection
        self.ctx.selected = match self.current_view() {
            View::BlockDetail => {
                if let Some(block) = self.blocks.get(self.selected_block) {
                    crate::core::Selected::Block(block.number)
                } else {
                    crate::core::Selected::None
                }
            }
            View::TxDetail => {
                if let Some(tx) = self.visible_txs().get(self.selected_tx) {
                    crate::core::Selected::Transaction(tx.hash.clone())
                } else {
                    crate::core::Selected::None
                }
            }
            View::AddressDetail | View::ContractDetail => {
                if let Some(addr) = self.addresses.get(self.selected_address) {
                    crate::core::Selected::Address(addr.address.clone())
                } else if let Some(contract) = self.contracts.get(self.selected_contract) {
                    crate::core::Selected::Address(contract.address.clone())
                } else {
                    crate::core::Selected::None
                }
            }
            View::Trace => {
                if let Some(tx) = self.visible_txs().get(self.selected_tx) {
                    crate::core::Selected::TraceFrame {
                        tx: tx.hash.clone(),
                        index: self.selected_trace,
                    }
                } else {
                    crate::core::Selected::None
                }
            }
            _ => crate::core::Selected::None,
        };
    }
```

**Step 5: Call sync_context in main loop**

In `src/main.rs`, in the `run_app` function, add after `pump_background(&mut app, &runtime, &abi_evt_rx);`:
```rust
        app.sync_context();
```

**Step 6: Verify compilation**

Run: `cargo check`
Expected: Compiles with no errors

**Step 7: Commit**

```bash
git add src/app.rs src/main.rs
git commit -m "feat(core): integrate Context with App state"
```

---

## Task 6: Add Command Execution Hook

**Files:**
- Modify: `src/app.rs` (add execute_command method)
- Modify: `src/main.rs` (call execute_command)

**Step 1: Add execute_command method to App**

Add to `src/app.rs` in the second `impl App` block (around line 1556):
```rust
    /// Execute a parsed command
    pub fn execute_command(&mut self, cmd: &crate::core::Command) -> crate::core::Action {
        use crate::core::{Action, Command, NavigateTarget, NotifyLevel};

        match cmd {
            // Navigation commands
            Command::Blocks => Action::Navigate(NavigateTarget::Blocks),
            Command::Transactions => Action::Navigate(NavigateTarget::Transactions),
            Command::Address(addr) => Action::Navigate(NavigateTarget::Address(addr.clone())),
            Command::Trace(hash) => Action::Navigate(NavigateTarget::Trace(hash.clone())),

            // Toolkit commands - not yet implemented, show notification
            Command::Encode(_) => Action::Notify("Encode: coming soon".into(), NotifyLevel::Info),
            Command::Decode(_) => Action::Notify("Decode: coming soon".into(), NotifyLevel::Info),
            Command::Hash(_) => Action::Notify("Hash: coming soon".into(), NotifyLevel::Info),
            Command::Hex(_) => Action::Notify("Hex: coming soon".into(), NotifyLevel::Info),
            Command::Selector(_) => Action::Notify("Selector: coming soon".into(), NotifyLevel::Info),
            Command::FourByte(_) => Action::Notify("4byte: coming soon".into(), NotifyLevel::Info),
            Command::Convert(_) => Action::Notify("Convert: coming soon".into(), NotifyLevel::Info),
            Command::Timestamp(_) => Action::Notify("Timestamp: coming soon".into(), NotifyLevel::Info),
            Command::Call(_) => Action::Notify("Call: coming soon".into(), NotifyLevel::Info),
            Command::Gas(_) => Action::Notify("Gas: coming soon".into(), NotifyLevel::Info),
            Command::Slot(_) => Action::Notify("Slot: coming soon".into(), NotifyLevel::Info),
            Command::Create(_) => Action::Notify("Create: coming soon".into(), NotifyLevel::Info),
            Command::Create2(_) => Action::Notify("Create2: coming soon".into(), NotifyLevel::Info),
            Command::Checksum(_) => Action::Notify("Checksum: coming soon".into(), NotifyLevel::Info),

            // Ops commands - not yet implemented
            Command::Health => Action::Notify("Health: coming soon".into(), NotifyLevel::Info),
            Command::Peers => Action::Notify("Peers: coming soon".into(), NotifyLevel::Info),
            Command::Logs => Action::Notify("Logs: coming soon".into(), NotifyLevel::Info),
            Command::Mempool => Action::Notify("Mempool: coming soon".into(), NotifyLevel::Info),
            Command::RpcStats => Action::Notify("RPC Stats: coming soon".into(), NotifyLevel::Info),

            // Node management - not yet implemented
            Command::Connect(_) => Action::Notify("Connect: coming soon".into(), NotifyLevel::Info),
            Command::Anvil(_) => Action::Notify("Anvil: coming soon".into(), NotifyLevel::Info),
            Command::Impersonate(_) => Action::Notify("Impersonate: coming soon".into(), NotifyLevel::Info),
            Command::Mine(_) => Action::Notify("Mine: coming soon".into(), NotifyLevel::Info),
            Command::Snapshot => Action::Notify("Snapshot: coming soon".into(), NotifyLevel::Info),
            Command::Revert(_) => Action::Notify("Revert: coming soon".into(), NotifyLevel::Info),

            Command::Unknown(s) => Action::Notify(format!("Unknown command: {}", s), NotifyLevel::Warn),
        }
    }

    /// Apply an action returned by a command or module
    pub fn apply_action(&mut self, action: crate::core::Action) {
        use crate::core::{Action, NavigateTarget, NotifyLevel};

        match action {
            Action::None => {}
            Action::Navigate(target) => match target {
                NavigateTarget::Back => self.pop_view(),
                NavigateTarget::Dashboard => {
                    self.view_stack = vec![View::Overview];
                    self.active_section = Section::Overview;
                }
                NavigateTarget::Blocks => {
                    self.view_stack = vec![View::Overview];
                    self.active_section = Section::Blocks;
                    self.focus = Focus::List;
                }
                NavigateTarget::Transactions => {
                    self.view_stack = vec![View::Overview];
                    self.active_section = Section::Transactions;
                    self.focus = Focus::List;
                }
                NavigateTarget::Block(num) => {
                    // Find block and navigate
                    if let Some(idx) = self.blocks.iter().position(|b| b.number == num) {
                        self.selected_block = idx;
                        self.active_section = Section::Blocks;
                        self.push_view(View::BlockDetail);
                    }
                }
                NavigateTarget::Transaction(hash) => {
                    // Find tx and navigate
                    if let Some(idx) = self.txs.iter().position(|t| t.hash == hash) {
                        self.selected_tx = idx;
                        self.active_section = Section::Transactions;
                        self.push_view(View::TxDetail);
                    }
                }
                NavigateTarget::Address(addr) => {
                    self.set_status(format!("Navigate to address: {}", addr), StatusLevel::Info);
                }
                NavigateTarget::Trace(hash) => {
                    self.pending_trace_request = Some(hash);
                }
            },
            Action::Copy(text) => {
                self.ctx.set_clipboard(text.clone());
                self.set_status("Copied to clipboard".into(), StatusLevel::Info);
            }
            Action::Notify(msg, level) => {
                let level = match level {
                    NotifyLevel::Info => StatusLevel::Info,
                    NotifyLevel::Warn => StatusLevel::Warn,
                    NotifyLevel::Error => StatusLevel::Error,
                };
                self.set_status(msg, level);
            }
            Action::OpenCommand(prefix) => {
                self.input_mode = InputMode::Command;
                self.focus = Focus::Command;
                if let Some(p) = prefix {
                    self.command.input = p;
                }
            }
            Action::CloseOverlay => {
                self.help_open = false;
                self.settings_open = false;
            }
            Action::Quit => {
                self.should_quit = true;
            }
        }
    }
```

**Step 2: Update command handling in main.rs**

In `src/main.rs`, find where Enter is handled in Command mode (search for `InputMode::Command` and Enter key handling). Update the command execution to use the new system. Find the section that handles Enter in command mode and update it:

After the existing command handling (which does filtering/navigation), add:
```rust
                        // Try as a : command if it starts with known command names
                        let input = app.command.input.trim();
                        if !input.is_empty() {
                            let cmd = crate::core::parse_command(input);
                            let action = app.execute_command(&cmd);
                            app.apply_action(action);
                        }
```

**Step 3: Verify compilation**

Run: `cargo check`
Expected: Compiles with no errors

**Step 4: Test manually**

Run: `cargo run -- --rpc http://localhost:8545`
- Press `:` to enter command mode
- Type `blocks` and press Enter
- Should navigate to blocks section

**Step 5: Commit**

```bash
git add src/app.rs src/main.rs
git commit -m "feat(core): add command execution and action system"
```

---

## Task 7: Add Command Autocompletion Hints

**Files:**
- Modify: `src/ui/mod.rs` (update command line rendering)

**Step 1: Add command hint function**

Add to `src/ui/mod.rs` before `draw_command_line`:
```rust
fn command_hint(input: &str) -> Option<&'static str> {
    let input = input.trim().to_lowercase();
    if input.is_empty() {
        return None;
    }

    let commands = [
        ("blocks", "Navigate to blocks"),
        ("txs", "Navigate to transactions"),
        ("address", "Navigate to address"),
        ("trace", "Show transaction trace"),
        ("encode", "ABI encode calldata"),
        ("decode", "ABI decode data"),
        ("hash", "Compute keccak256"),
        ("hex", "Convert hex/dec/string"),
        ("selector", "Compute function selector"),
        ("4byte", "Lookup selector"),
        ("convert", "Convert units (wei/gwei/ether)"),
        ("timestamp", "Convert timestamp"),
        ("call", "Call contract (read-only)"),
        ("gas", "Estimate gas"),
        ("slot", "Calculate storage slot"),
        ("health", "Node health check"),
        ("peers", "Show peer details"),
        ("logs", "Show node logs"),
        ("anvil", "Manage Anvil"),
        ("mine", "Mine blocks (Anvil)"),
    ];

    for (cmd, desc) in commands {
        if cmd.starts_with(&input) {
            return Some(desc);
        }
    }
    None
}
```

**Step 2: Update draw_command_line to show hints**

In `src/ui/mod.rs`, find `draw_command_line` function and update it to show hints:

Find the line that creates the command paragraph and update to include hint:
```rust
fn draw_command_line(f: &mut Frame, area: Rect, app: &App) {
    let (prefix, input) = match app.input_mode {
        InputMode::Command => (":", &app.command.input),
        InputMode::Prompt(PromptKind::StorageSlot) => ("slot> ", &app.command.input),
        InputMode::Prompt(PromptKind::Label) => ("label> ", &app.command.input),
        _ => {
            // Show last command or hint
            let text = app.command.last.as_deref().unwrap_or("Press : for commands, / to search, ? for help");
            let p = Paragraph::new(text).style(Style::default().fg(Color::DarkGray));
            f.render_widget(p, area);
            return;
        }
    };

    // Build the line with optional hint
    let hint = command_hint(input);
    let mut spans = vec![
        Span::styled(prefix, Style::default().fg(Color::Yellow)),
        Span::raw(input),
    ];

    if let Some(h) = hint {
        spans.push(Span::styled(
            format!("  {}", h),
            Style::default().fg(Color::DarkGray),
        ));
    }

    let line = Line::from(spans);
    let p = Paragraph::new(line);
    f.render_widget(p, area);

    // Show cursor
    if matches!(app.input_mode, InputMode::Command | InputMode::Prompt(_)) {
        let cursor_x = area.x + prefix.len() as u16 + input.len() as u16;
        f.set_cursor(cursor_x, area.y);
    }
}
```

**Step 3: Verify compilation**

Run: `cargo check`
Expected: Compiles with no errors

**Step 4: Test manually**

Run: `cargo run -- --rpc http://localhost:8545`
- Press `:` to enter command mode
- Type `enc` - should see "ABI encode calldata" hint

**Step 5: Commit**

```bash
git add src/ui/mod.rs
git commit -m "feat(ui): add command autocompletion hints"
```

---

## Summary

After completing all tasks, you will have:

1. **Core module** (`src/core/`)
   - `Action` enum for module-to-app communication
   - `Context` for shared state
   - `Command` parser for `:command` system
   - `Module` trait for extensible components

2. **Modules directory** (`src/modules/`) - ready for expansion

3. **Integration**
   - Context synced with App state
   - Command execution via `:` commands
   - Action system for navigation and notifications
   - Autocompletion hints in command line

**Next phases will add:**
- Phase 1: Toolkit modules (encode, decode, convert, etc.)
- Phase 2: Ops modules (health, peers, logs)
- Phase 3: Workflow modules (anvil, watch)
