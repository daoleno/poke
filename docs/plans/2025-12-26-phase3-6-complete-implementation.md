# Phase 3-6: Complete Poke v2 Implementation

> **Date:** 2025-12-26
> **Status:** Planning → Execution

## Overview

Complete implementation of all remaining Poke v2 features across 4 major phases.

---

## Phase 3: Complete Toolkit (A)

**Goal:** Implement remaining developer tool commands

### Commands to Implement

| Priority | Command | Function | Complexity |
|----------|---------|----------|------------|
| P0 | `:encode` | ABI encode calldata | Medium |
| P0 | `:decode` | ABI decode calldata/logs | Medium |
| P0 | `:call` | Read-only contract calls | High |
| P1 | `:gas` | Gas estimation | Medium |
| P1 | `:slot` | Storage slot calculation | Low |
| P1 | `:create` | CREATE address calculation | Low |
| P1 | `:create2` | CREATE2 address calculation | Low |

### Tasks

1. **Encode/Decode** (ABI encoding/decoding)
   - `encode.rs`: Parse function signature, encode parameters
   - `decode.rs`: Decode calldata/event logs
   - Integrate with existing ABI resolver

2. **Contract Call** (read-only calls)
   - `call.rs`: Parse call syntax, execute via RPC
   - Support syntax: `:call 0xAddr.function(args)`
   - Return decoded results

3. **Gas Estimation**
   - `gas.rs`: Estimate gas for transactions
   - Support same syntax as `:call`

4. **Storage Slot Calculation**
   - `slot.rs`: Calculate storage slots
   - Support: mapping, array, struct offsets

5. **Address Calculation**
   - `create.rs`: CREATE address from deployer + nonce
   - `create2.rs`: CREATE2 address from salt + initcode

---

## Phase 4: Workflow Enhancement (B)

**Goal:** Implement workflow management for local development

### Features

| Priority | Feature | Description |
|----------|---------|-------------|
| P0 | Anvil Management | Start/stop/status local Anvil nodes |
| P0 | Anvil Control | impersonate, mine, snapshot, revert |
| P1 | Multi-node | Connect to multiple nodes, switch between them |
| P1 | Watch Enhancement | Watch addresses/contracts with alerts |

### Tasks

1. **Anvil Manager** (`workflow/anvil.rs`)
   - `:anvil` - Start Anvil with default config
   - `:anvil --fork mainnet` - Fork mode
   - `:anvil --port 8546` - Custom port
   - `:anvil kill` - Stop Anvil
   - Track Anvil process, auto-cleanup

2. **Anvil Control** (`workflow/anvil_control.rs`)
   - `:impersonate <addr>` - Impersonate account
   - `:mine [n]` - Mine n blocks
   - `:snapshot` - Create snapshot
   - `:revert <id>` - Revert to snapshot

3. **Node Manager** (`workflow/nodes.rs`)
   - `:connect <url>` - Add node connection
   - `:nodes` - List all nodes
   - `:switch <name>` - Switch active node
   - Store in context

4. **Watch Enhancement** (`workflow/watch.rs`)
   - Watch address balance changes
   - Watch contract events
   - Alert on threshold changes

---

## Phase 5: Ops Deep Dive (C)

**Goal:** Enhanced operations monitoring and alerting

### Features

| Priority | Feature | Description |
|----------|---------|-------------|
| P0 | Real-time Logs | Stream node logs to TUI |
| P0 | Metrics Panel | Visualize metrics with sparklines |
| P1 | Alert System | Configurable alerts for ops events |
| P1 | Performance | RPC call tracking and profiling |

### Tasks

1. **Real-time Logs** (`ops/logs.rs`)
   - `:logs` - Stream logs in overlay
   - `:logs --level error` - Filter by level
   - Parse geth/reth/anvil log formats
   - Color-coded output

2. **Metrics Panel** (`ops/metrics.rs`)
   - Collect time-series metrics
   - RPC latency history
   - Block time history
   - Peer count history
   - Gas price history
   - Render with sparklines

3. **Alert System** (`ops/alerts.rs`)
   - Define alert rules
   - Peer count low
   - RPC latency high
   - Sync stalled
   - Show in dashboard

4. **Performance Tracking** (`ops/performance.rs`)
   - Track all RPC calls
   - Aggregate statistics
   - Identify slow calls
   - Cache hit rates

---

## Phase 6: Dashboard/Explorer UI (D)

**Goal:** Implement panel-based dashboard and refactor explorer

### Architecture

```
L0: Dashboard (panel-based, default view)
├── NODES panel
├── ACTIVITY panel
├── INSPECTOR panel
└── WATCHING panel

L1: Explorer (full-screen drill-down)
├── Blocks view
├── Transactions view
└── Trace view
```

### Tasks

1. **Dashboard Module** (`modules/dashboard/`)
   - `mod.rs` - Dashboard state and layout
   - `nodes_panel.rs` - Node status panel
   - `activity_panel.rs` - Recent blocks/txs
   - `inspector_panel.rs` - Selected item details
   - `watching_panel.rs` - Watched items
   - Tab navigation between panels
   - Press `f` to enter explorer

2. **Explorer Refactor** (`modules/explorer/`)
   - Extract from current App
   - Modularize blocks/txs/trace views
   - Implement Module trait
   - Navigate back to dashboard with `Esc`

3. **Context Integration**
   - Wire up Selected enum
   - Clipboard operations
   - Labels system
   - Inspector auto-updates based on selection

4. **UI Improvements**
   - Panel borders and titles
   - Status icons
   - Color scheme
   - Help overlay (`?`)

---

## Implementation Order

Execute phases sequentially:

1. **Phase 3** - Complete Toolkit (foundational)
2. **Phase 4** - Workflow (uses toolkit)
3. **Phase 5** - Ops Deep Dive (independent)
4. **Phase 6** - Dashboard/Explorer (big refactor)

Each phase will have detailed sub-plans and be executed task-by-task with commits.

---

## Estimated Scope

| Phase | Tasks | Commits | Complexity |
|-------|-------|---------|------------|
| Phase 3 | ~7 | ~10 | Medium-High |
| Phase 4 | ~8 | ~12 | Medium-High |
| Phase 5 | ~6 | ~8 | Medium |
| Phase 6 | ~10 | ~15 | High |
| **Total** | **~31** | **~45** | **High** |

---

## Success Criteria

**Phase 3:** All toolkit commands work, can encode/decode/call contracts
**Phase 4:** Can start Anvil, manage nodes, watch addresses
**Phase 5:** Real-time logs, metrics visualization, alerts working
**Phase 6:** Dashboard with panels, explorer as drill-down, smooth navigation
