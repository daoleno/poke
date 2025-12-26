# Phase 5: Ops Deep Dive Implementation

> **Date:** 2025-12-26
> **Status:** Execution

## Goal

Enhanced operations monitoring: real-time logs placeholder, metrics visualization, and alert framework.

---

## Simplified Scope

Given time constraints, focus on:
1. **Logs command structure** - Parse and prepare for streaming
2. **Metrics tracking** - Data structures for time-series metrics
3. **Alert framework** - Rule definitions and checking logic

**Defer:**
- Actual log streaming (requires async/tokio runtime)
- Live metrics updates (requires background polling)
- Alert notifications UI

---

## Task Breakdown

### Task 1: Logs Command Structure

**File:** `src/modules/ops/logs.rs`

**Features:**
- `:logs` - Show logs placeholder
- `:logs --level error` - Filter by level
- Parse log level and format options

**Implementation:**
```rust
pub fn logs(input: Option<String>) -> Action
```

---

### Task 2: Metrics Data Structures

**File:** `src/modules/ops/metrics.rs`

**Features:**
- `MetricsCollector` - Store time-series data
- `Metric` - Single metric with history
- Support for RPC latency, block time, peer count, gas price

**Implementation:**
```rust
pub struct MetricsCollector {
    pub rpc_latency: Vec<u64>,
    pub block_times: Vec<u64>,
    pub peer_counts: Vec<u32>,
    pub gas_prices: Vec<u64>,
}
```

---

### Task 3: Display Metrics

**File:** `src/modules/ops/metrics.rs`

**Features:**
- `:metrics` - Show current metrics with sparklines
- Format using sparkline_text from ui::widgets

---

### Task 4: Alert Framework

**File:** `src/modules/ops/alerts.rs`

**Features:**
- `AlertRule` - Define alert conditions
- `AlertChecker` - Evaluate rules
- Built-in rules: peer_count_low, rpc_latency_high, sync_stalled

**Implementation:**
```rust
pub struct AlertRule {
    pub name: String,
    pub condition: AlertCondition,
    pub message: String,
}

pub enum AlertCondition {
    PeerCountBelow(u32),
    RpcLatencyAbove(u64),
    SyncStalled,
}
```

---

### Task 5: Wire Up Commands

Update `src/app.rs` to:
- Add logs command handler
- Add metrics command handler
- Initialize MetricsCollector
- Check alerts periodically

---

## Implementation Order

1. Task 1: Logs structure
2. Task 2: Metrics data structures
3. Task 3: Display metrics
4. Task 4: Alert framework
5. Task 5: Wire up

---

## Success Criteria

✓ `:logs` command shows structure
✓ MetricsCollector defined with time-series storage
✓ `:metrics` displays formatted metrics
✓ AlertRule framework defined
✓ Commands wired up in App

---

## Commits

~5 commits:
1. `feat(ops): add logs command structure`
2. `feat(ops): add metrics collector`
3. `feat(ops): add metrics display with sparklines`
4. `feat(ops): add alert framework`
5. `feat(ops): wire up new ops commands`
