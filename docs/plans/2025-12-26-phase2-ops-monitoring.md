# Phase 2: Ops Monitoring Implementation

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement operations monitoring commands for node health, peers, and metrics

**Architecture:**
- Ops module at `src/modules/ops/` with health, peers, logs submodules
- Real-time data via existing RPC infrastructure
- Sparklines widget for metrics visualization

**Tech Stack:** Rust, ratatui (for sparklines), existing RPC provider

---

## Overview

Phase 2 focuses on operations monitoring commands:

| Priority | Command | Function |
|----------|---------|----------|
| P0 | `:health` | Node health check summary |
| P0 | `:peers` | Connected peers list |
| P0 | `:rpc-stats` | RPC call statistics |
| P1 | `:mempool` | Transaction pool status |
| P1 | Sparklines | Metrics visualization widget |

---

## Task 1: Create Ops Module Structure

**Files:**
- Create: `src/modules/ops/mod.rs`
- Create: `src/modules/ops/health.rs`
- Modify: `src/modules/mod.rs`

**Step 1: Create ops module**

Create `src/modules/ops/mod.rs`:
```rust
//! Operations monitoring commands

pub mod health;

use crate::core::{Action, NotifyLevel};

/// Result of an ops check
pub struct OpsResult {
    pub title: String,
    pub items: Vec<OpsItem>,
}

pub struct OpsItem {
    pub label: String,
    pub value: String,
    pub status: OpsStatus,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum OpsStatus {
    Ok,
    Warning,
    Error,
    Unknown,
}

impl OpsResult {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            items: Vec::new(),
        }
    }

    pub fn add(mut self, label: impl Into<String>, value: impl Into<String>, status: OpsStatus) -> Self {
        self.items.push(OpsItem {
            label: label.into(),
            value: value.into(),
            status,
        });
        self
    }

    pub fn into_action(self) -> Action {
        let msg = self.items
            .iter()
            .map(|item| {
                let icon = match item.status {
                    OpsStatus::Ok => "●",
                    OpsStatus::Warning => "◐",
                    OpsStatus::Error => "○",
                    OpsStatus::Unknown => "?",
                };
                format!("{} {}: {}", icon, item.label, item.value)
            })
            .collect::<Vec<_>>()
            .join(" | ");
        Action::Notify(format!("{} - {}", self.title, msg), NotifyLevel::Info)
    }
}
```

**Step 2: Create health.rs**

Create `src/modules/ops/health.rs`:
```rust
//! Node health check

use super::{OpsResult, OpsStatus};
use crate::core::Action;

/// Health check result from RPC
pub struct HealthData {
    pub syncing: bool,
    pub peer_count: u32,
    pub rpc_latency_ms: Option<u64>,
    pub chain_id: Option<u64>,
    pub block_number: Option<u64>,
}

impl Default for HealthData {
    fn default() -> Self {
        Self {
            syncing: false,
            peer_count: 0,
            rpc_latency_ms: None,
            chain_id: None,
            block_number: None,
        }
    }
}

/// Generate health check action from data
pub fn health(data: &HealthData) -> Action {
    let sync_status = if data.syncing {
        OpsStatus::Warning
    } else {
        OpsStatus::Ok
    };
    let sync_text = if data.syncing { "syncing" } else { "synced" };

    let peer_status = if data.peer_count == 0 {
        OpsStatus::Error
    } else if data.peer_count < 3 {
        OpsStatus::Warning
    } else {
        OpsStatus::Ok
    };

    let rpc_status = match data.rpc_latency_ms {
        Some(ms) if ms < 100 => OpsStatus::Ok,
        Some(ms) if ms < 500 => OpsStatus::Warning,
        Some(_) => OpsStatus::Error,
        None => OpsStatus::Unknown,
    };
    let rpc_text = data.rpc_latency_ms
        .map(|ms| format!("{}ms", ms))
        .unwrap_or_else(|| "N/A".to_string());

    let block_text = data.block_number
        .map(|n| format!("#{}", n))
        .unwrap_or_else(|| "N/A".to_string());

    let result = OpsResult::new("Health")
        .add("sync", sync_text, sync_status)
        .add("peers", data.peer_count.to_string(), peer_status)
        .add("rpc", rpc_text, rpc_status)
        .add("block", block_text, OpsStatus::Ok);

    result.into_action()
}

/// Create a health action from current app state
pub fn health_from_state(
    syncing: bool,
    peer_count: u32,
    rpc_latency_ms: Option<u64>,
    block_number: Option<u64>,
) -> Action {
    let data = HealthData {
        syncing,
        peer_count,
        rpc_latency_ms,
        chain_id: None,
        block_number,
    };
    health(&data)
}
```

**Step 3: Update modules/mod.rs**

Add `pub mod ops;` after `pub mod toolkit;`

**Step 4: Verify and commit**

---

## Task 2: Add peers command

**Files:**
- Create: `src/modules/ops/peers.rs`
- Modify: `src/modules/ops/mod.rs`

**Step 1: Create peers.rs**

```rust
//! Peer information display

use super::{OpsResult, OpsStatus};
use crate::core::Action;

/// Peer info from RPC
#[derive(Clone, Debug)]
pub struct PeerInfo {
    pub id: String,
    pub name: String,
    pub remote_addr: String,
    pub local_addr: String,
    pub caps: Vec<String>,
}

/// Display peer count and summary
pub fn peers(peer_count: u32, peers: &[PeerInfo]) -> Action {
    let status = if peer_count == 0 {
        OpsStatus::Error
    } else if peer_count < 3 {
        OpsStatus::Warning
    } else {
        OpsStatus::Ok
    };

    let mut result = OpsResult::new("Peers")
        .add("connected", peer_count.to_string(), status);

    // Add first few peers
    for (i, peer) in peers.iter().take(3).enumerate() {
        let name = if peer.name.len() > 20 {
            format!("{}...", &peer.name[..17])
        } else {
            peer.name.clone()
        };
        result = result.add(
            format!("peer{}", i + 1),
            format!("{} ({})", name, peer.remote_addr),
            OpsStatus::Ok,
        );
    }

    if peers.len() > 3 {
        result = result.add("more", format!("+{}", peers.len() - 3), OpsStatus::Ok);
    }

    result.into_action()
}

/// Simple peers display with just count
pub fn peers_count(count: u32) -> Action {
    let status = if count == 0 {
        OpsStatus::Error
    } else if count < 3 {
        OpsStatus::Warning
    } else {
        OpsStatus::Ok
    };

    OpsResult::new("Peers")
        .add("connected", count.to_string(), status)
        .into_action()
}
```

**Step 2: Update ops/mod.rs to add `pub mod peers;`**

**Step 3: Verify and commit**

---

## Task 3: Add rpc-stats command

**Files:**
- Create: `src/modules/ops/rpc_stats.rs`
- Modify: `src/modules/ops/mod.rs`

**Step 1: Create rpc_stats.rs**

```rust
//! RPC call statistics

use super::{OpsResult, OpsStatus};
use crate::core::Action;

/// RPC statistics
#[derive(Default, Clone)]
pub struct RpcStats {
    pub total_calls: u64,
    pub failed_calls: u64,
    pub avg_latency_ms: u64,
    pub max_latency_ms: u64,
    pub calls_per_second: f64,
}

/// Display RPC statistics
pub fn rpc_stats(stats: &RpcStats) -> Action {
    let latency_status = if stats.avg_latency_ms < 100 {
        OpsStatus::Ok
    } else if stats.avg_latency_ms < 500 {
        OpsStatus::Warning
    } else {
        OpsStatus::Error
    };

    let error_rate = if stats.total_calls > 0 {
        (stats.failed_calls as f64 / stats.total_calls as f64) * 100.0
    } else {
        0.0
    };

    let error_status = if error_rate < 1.0 {
        OpsStatus::Ok
    } else if error_rate < 5.0 {
        OpsStatus::Warning
    } else {
        OpsStatus::Error
    };

    OpsResult::new("RPC Stats")
        .add("calls", stats.total_calls.to_string(), OpsStatus::Ok)
        .add("avg", format!("{}ms", stats.avg_latency_ms), latency_status)
        .add("max", format!("{}ms", stats.max_latency_ms), OpsStatus::Ok)
        .add("errors", format!("{:.1}%", error_rate), error_status)
        .into_action()
}

/// Simple RPC stats from current state
pub fn rpc_stats_simple(latency_ms: Option<u64>, endpoint: &str) -> Action {
    let latency_status = match latency_ms {
        Some(ms) if ms < 100 => OpsStatus::Ok,
        Some(ms) if ms < 500 => OpsStatus::Warning,
        Some(_) => OpsStatus::Error,
        None => OpsStatus::Unknown,
    };

    let latency_text = latency_ms
        .map(|ms| format!("{}ms", ms))
        .unwrap_or_else(|| "N/A".to_string());

    let short_endpoint = if endpoint.len() > 30 {
        format!("{}...", &endpoint[..27])
    } else {
        endpoint.to_string()
    };

    OpsResult::new("RPC Stats")
        .add("endpoint", short_endpoint, OpsStatus::Ok)
        .add("latency", latency_text, latency_status)
        .into_action()
}
```

**Step 2: Update ops/mod.rs to add `pub mod rpc_stats;`**

**Step 3: Verify and commit**

---

## Task 4: Add mempool command

**Files:**
- Create: `src/modules/ops/mempool.rs`
- Modify: `src/modules/ops/mod.rs`

**Step 1: Create mempool.rs**

```rust
//! Transaction pool (mempool) status

use super::{OpsResult, OpsStatus};
use crate::core::{Action, NotifyLevel};

/// Mempool statistics
#[derive(Default, Clone)]
pub struct MempoolStats {
    pub pending: u64,
    pub queued: u64,
    pub local_pending: u64,
}

/// Display mempool status
pub fn mempool(stats: &MempoolStats) -> Action {
    let total = stats.pending + stats.queued;

    let pending_status = if stats.pending > 10000 {
        OpsStatus::Warning
    } else {
        OpsStatus::Ok
    };

    OpsResult::new("Mempool")
        .add("pending", stats.pending.to_string(), pending_status)
        .add("queued", stats.queued.to_string(), OpsStatus::Ok)
        .add("total", total.to_string(), OpsStatus::Ok)
        .into_action()
}

/// Placeholder when mempool not available
pub fn mempool_unavailable() -> Action {
    Action::Notify(
        "Mempool: requires txpool_status RPC (not available on all nodes)".into(),
        NotifyLevel::Warn,
    )
}
```

**Step 2: Update ops/mod.rs to add `pub mod mempool;`**

**Step 3: Verify and commit**

---

## Task 5: Create sparkline widget

**Files:**
- Create: `src/ui/widgets/mod.rs`
- Create: `src/ui/widgets/sparkline.rs`
- Modify: `src/ui/mod.rs`

**Step 1: Create widgets directory**

Create `src/ui/widgets/mod.rs`:
```rust
//! Reusable UI widgets

pub mod sparkline;

pub use sparkline::MiniSparkline;
```

**Step 2: Create sparkline.rs**

```rust
//! Mini sparkline widget for inline metrics

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

/// A compact inline sparkline (single line)
pub struct MiniSparkline<'a> {
    data: &'a [u64],
    max: Option<u64>,
    style: Style,
    bar_chars: [char; 8],
}

impl<'a> MiniSparkline<'a> {
    pub fn new(data: &'a [u64]) -> Self {
        Self {
            data,
            max: None,
            style: Style::default().fg(Color::Cyan),
            bar_chars: ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'],
        }
    }

    pub fn max(mut self, max: u64) -> Self {
        self.max = Some(max);
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl<'a> Widget for MiniSparkline<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 || self.data.is_empty() {
            return;
        }

        let max = self.max.unwrap_or_else(|| *self.data.iter().max().unwrap_or(&1));
        let max = max.max(1); // Avoid division by zero

        // Take the last N values that fit in the area
        let data_len = self.data.len().min(area.width as usize);
        let data_start = self.data.len().saturating_sub(data_len);

        for (i, &value) in self.data[data_start..].iter().enumerate() {
            let x = area.x + i as u16;
            if x >= area.x + area.width {
                break;
            }

            // Scale value to 0-7 range
            let scaled = if max > 0 {
                ((value as f64 / max as f64) * 7.0).round() as usize
            } else {
                0
            };
            let scaled = scaled.min(7);

            let ch = self.bar_chars[scaled];
            buf.get_mut(x, area.y).set_char(ch).set_style(self.style);
        }
    }
}

/// Format sparkline data as inline text (for status messages)
pub fn sparkline_text(data: &[u64], width: usize) -> String {
    if data.is_empty() {
        return String::new();
    }

    let bar_chars = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let max = *data.iter().max().unwrap_or(&1);
    let max = max.max(1);

    let data_len = data.len().min(width);
    let data_start = data.len().saturating_sub(data_len);

    data[data_start..]
        .iter()
        .map(|&value| {
            let scaled = if max > 0 {
                ((value as f64 / max as f64) * 7.0).round() as usize
            } else {
                0
            };
            bar_chars[scaled.min(7)]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sparkline_text() {
        let data = [1, 2, 3, 4, 5, 6, 7, 8];
        let text = sparkline_text(&data, 8);
        assert_eq!(text.chars().count(), 8);
    }

    #[test]
    fn test_sparkline_text_empty() {
        let data: [u64; 0] = [];
        let text = sparkline_text(&data, 8);
        assert!(text.is_empty());
    }
}
```

**Step 3: Update ui/mod.rs to add `pub mod widgets;`**

**Step 4: Verify and commit**

---

## Task 6: Wire up ops commands in App

**Files:**
- Modify: `src/app.rs`

**Step 1: Update execute_command**

Replace the placeholder ops commands with actual implementations:

```rust
// Ops commands - implemented
Command::Health => {
    let block_num = self.blocks.last().map(|b| b.number);
    crate::modules::ops::health::health_from_state(
        self.sync_progress < 1.0,
        self.peer_count,
        self.last_rtt_ms,
        block_num,
    )
}
Command::Peers => crate::modules::ops::peers::peers_count(self.peer_count),
Command::RpcStats => crate::modules::ops::rpc_stats::rpc_stats_simple(
    self.last_rtt_ms,
    &self.rpc_endpoint,
),
Command::Mempool => crate::modules::ops::mempool::mempool_unavailable(),

// Ops commands - not yet implemented
Command::Logs => Action::Notify("Logs: coming soon".into(), NotifyLevel::Info),
```

**Step 2: Verify and run tests**

**Step 3: Commit**

---

## Summary

After completing Phase 2, users can:

- `:health` → See node sync status, peer count, RPC latency, block number
- `:peers` → See connected peer count
- `:rpc-stats` → See RPC endpoint and latency
- `:mempool` → Get mempool status info (or unavailable message)
- Sparkline widget available for future metrics visualization
