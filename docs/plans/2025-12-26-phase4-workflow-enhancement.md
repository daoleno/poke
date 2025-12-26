# Phase 4: Workflow Enhancement Implementation

> **Date:** 2025-12-26
> **Dependencies:** Phase 3 (toolkit), existing infrastructure

## Goal

Implement workflow management for local development: Anvil management, node switching, and watch features.

---

## Architecture Overview

```
src/modules/workflow/
├── mod.rs           # Workflow module exports
├── anvil.rs         # Anvil lifecycle management
├── anvil_control.rs # Anvil-specific commands
└── nodes.rs         # Multi-node management
```

---

## Task Breakdown

### Task 1: Create Workflow Module Structure

**File:** `src/modules/workflow/mod.rs`

Basic structure for workflow commands.

---

### Task 2: Anvil Manager (Basic)

**File:** `src/modules/workflow/anvil.rs`

**Features:**
- `:anvil` - Start Anvil with default config
- `:anvil kill` - Stop Anvil
- `:anvil status` - Check if Anvil is running
- Track process ID and cleanup

**Implementation approach:**
- Use `std::process::Command` to spawn Anvil
- Store process handle in app state
- Check if anvil binary exists
- Default port: 8545

**Syntax:**
```
:anvil                    # Start with defaults
:anvil --port 8546        # Custom port
:anvil --fork mainnet     # Fork mode (future)
:anvil kill               # Stop
:anvil status             # Check status
```

---

### Task 3: Anvil Control Commands

**File:** `src/modules/workflow/anvil_control.rs`

**Features:**
- `:impersonate <addr>` - Impersonate account
- `:mine [n]` - Mine n blocks (default 1)
- `:snapshot` - Create snapshot
- `:revert <id>` - Revert to snapshot

**Implementation:**
- Use Anvil's RPC methods
- Requires async RPC calls
- For now: parse syntax, return preview

**Anvil RPC Methods:**
- `anvil_impersonateAccount`
- `anvil_mine`
- `anvil_snapshot`
- `anvil_revert`

---

### Task 4: Node Manager

**File:** `src/modules/workflow/nodes.rs`

**Features:**
- `:connect <url>` - Add node connection
- `:nodes` - List all nodes
- `:switch <name>` - Switch active node
- Store in app context

**Data structure:**
```rust
struct NodeConnection {
    name: String,
    url: String,
    kind: NodeKind, // Anvil, Geth, Reth, etc.
    active: bool,
}
```

---

### Task 5: Wire Up Commands

Update `src/app.rs` to:
- Add workflow command handlers
- Add node management state
- Add anvil process tracking

---

## Implementation Order

1. **Task 1** - Module structure
2. **Task 2** - Basic Anvil start/stop
3. **Task 3** - Anvil control commands (syntax only)
4. **Task 4** - Node management
5. **Task 5** - Integration

---

## Simplified Scope for Phase 4

Given the complexity of process management and async RPC:

**Implement Now:**
- Command parsing and syntax validation
- Anvil start/stop (basic process spawning)
- Node list management (data structures)
- Command structure for future async integration

**Defer to Later:**
- Full Anvil process lifecycle management
- Fork mode with automatic RPC proxying
- Async RPC calls for Anvil control
- Watch address changes (needs event polling)

---

## Success Criteria

✓ Can start Anvil with `:anvil`
✓ Can stop Anvil with `:anvil kill`
✓ Can check status with `:anvil status`
✓ Can parse Anvil control commands (impersonate, mine, snapshot, revert)
✓ Can manage node list (add, list, switch)
✓ All commands wired up in App

---

## Testing Strategy

- Unit tests for command parsing
- Manual testing for Anvil start/stop
- Verify process cleanup on exit

---

## Commits Structure

~8 commits:
1. `feat(workflow): add workflow module structure`
2. `feat(workflow): add basic Anvil manager`
3. `feat(workflow): add Anvil control commands (structure)`
4. `feat(workflow): add node management`
5. `feat(workflow): wire up workflow commands in App`
6. `test(workflow): add tests for command parsing`
7. `fix(workflow): improve Anvil process cleanup`
8. `docs(workflow): update command hints`

---

## Future Enhancements (Post-Phase 4)

- **Async RPC Integration** - Full Anvil control via RPC
- **Fork Mode** - Automatic mainnet forking with caching
- **Auto-cleanup** - Kill Anvil on app exit
- **Multiple Anvil Instances** - Manage multiple local nodes
- **Watch System** - Monitor address/contract changes
- **Snapshot Management** - Named snapshots with descriptions
