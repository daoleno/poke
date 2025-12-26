# Phase 6: Dashboard/Explorer UI Implementation

> **Date:** 2025-12-26
> **Status:** Execution

## Goal

Implement panel-based dashboard as the default view and refactor the existing explorer into a drill-down mode.

---

## Architecture

```
L0: Dashboard (panel-based, default view)
├── NODES panel     - Node status, RPC endpoint, peer count
├── ACTIVITY panel  - Recent blocks/transactions stream
├── INSPECTOR panel - Details of selected item
└── WATCHING panel  - Watched addresses/contracts

L1: Explorer (full-screen drill-down)
├── Blocks view     - Full block list with details
├── Transactions    - Full transaction list
└── Trace view      - Transaction trace visualization
```

**Navigation:**
- Dashboard is the default view on startup
- Press `f` to enter Explorer (full-screen)
- Press `Esc` in Explorer to return to Dashboard
- Tab navigation between dashboard panels

---

## Simplified Scope

Given the complexity, we'll implement Phase 6 in stages:

**Stage 1: Dashboard Module Structure**
- Create dashboard module with 4 panels
- Basic panel layout and rendering
- Tab navigation between panels

**Stage 2: Panel Implementations**
- NODES panel - Show current node info
- ACTIVITY panel - Recent blocks/txs preview (last 5-10 items)
- INSPECTOR panel - Selected item details
- WATCHING panel - Watched addresses list

**Stage 3: Explorer Refactor**
- Keep current explorer functionality
- Add navigation: Dashboard ↔ Explorer
- Press `f` in dashboard to enter explorer
- Press `Esc` in explorer to return to dashboard

**Stage 4: Context Integration**
- Wire up Selected enum for clipboard
- Inspector auto-updates based on selection
- Sync selection between dashboard and explorer

---

## Task Breakdown

### Task 1: Dashboard Module Structure

**Files:**
- Create: `src/modules/dashboard/mod.rs`
- Create: `src/modules/dashboard/panel.rs`
- Modify: `src/modules/mod.rs`
- Modify: `src/app.rs`

**Implementation:**

```rust
// src/modules/dashboard/mod.rs
pub mod panel;

use crate::core::{Action, Context, Module};
use ratatui::layout::{Constraint, Direction, Layout, Rect};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DashboardPanel {
    Nodes,
    Activity,
    Inspector,
    Watching,
}

pub struct Dashboard {
    active_panel: DashboardPanel,
}

impl Dashboard {
    pub fn new() -> Self {
        Self {
            active_panel: DashboardPanel::Nodes,
        }
    }

    pub fn next_panel(&mut self) {
        self.active_panel = match self.active_panel {
            DashboardPanel::Nodes => DashboardPanel::Activity,
            DashboardPanel::Activity => DashboardPanel::Inspector,
            DashboardPanel::Inspector => DashboardPanel::Watching,
            DashboardPanel::Watching => DashboardPanel::Nodes,
        };
    }

    pub fn prev_panel(&mut self) {
        self.active_panel = match self.active_panel {
            DashboardPanel::Nodes => DashboardPanel::Watching,
            DashboardPanel::Activity => DashboardPanel::Nodes,
            DashboardPanel::Inspector => DashboardPanel::Activity,
            DashboardPanel::Watching => DashboardPanel::Inspector,
        };
    }
}

impl Module for Dashboard {
    fn handle_key(&mut self, key: char, _ctx: &mut Context) -> Action {
        match key {
            '\t' => {
                self.next_panel();
                Action::None
            }
            'f' => Action::Navigate(crate::core::NavigateTarget::Explorer),
            _ => Action::None,
        }
    }

    fn handle_command(&mut self, _cmd: &str, _ctx: &mut Context) -> Action {
        Action::None
    }

    fn render(&mut self, frame: &mut ratatui::Frame, area: Rect, _ctx: &Context) {
        // Split into 4 quadrants
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        let top_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[0]);

        let bottom_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[1]);

        // Render panels (placeholders for now)
        self.render_nodes_panel(frame, top_chunks[0]);
        self.render_activity_panel(frame, top_chunks[1]);
        self.render_inspector_panel(frame, bottom_chunks[0]);
        self.render_watching_panel(frame, bottom_chunks[1]);
    }
}

impl Dashboard {
    fn render_nodes_panel(&self, frame: &mut ratatui::Frame, area: Rect) {
        use ratatui::widgets::{Block, Borders, Paragraph};
        use ratatui::style::{Color, Style};

        let is_active = self.active_panel == DashboardPanel::Nodes;
        let border_style = if is_active {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title("NODES")
            .border_style(border_style);

        let paragraph = Paragraph::new("Node info placeholder").block(block);
        frame.render_widget(paragraph, area);
    }

    fn render_activity_panel(&self, frame: &mut ratatui::Frame, area: Rect) {
        use ratatui::widgets::{Block, Borders, Paragraph};
        use ratatui::style::{Color, Style};

        let is_active = self.active_panel == DashboardPanel::Activity;
        let border_style = if is_active {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title("ACTIVITY")
            .border_style(border_style);

        let paragraph = Paragraph::new("Recent blocks/txs placeholder").block(block);
        frame.render_widget(paragraph, area);
    }

    fn render_inspector_panel(&self, frame: &mut ratatui::Frame, area: Rect) {
        use ratatui::widgets::{Block, Borders, Paragraph};
        use ratatui::style::{Color, Style};

        let is_active = self.active_panel == DashboardPanel::Inspector;
        let border_style = if is_active {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title("INSPECTOR")
            .border_style(border_style);

        let paragraph = Paragraph::new("Selected item details placeholder").block(block);
        frame.render_widget(paragraph, area);
    }

    fn render_watching_panel(&self, frame: &mut ratatui::Frame, area: Rect) {
        use ratatui::widgets::{Block, Borders, Paragraph};
        use ratatui::style::{Color, Style};

        let is_active = self.active_panel == DashboardPanel::Watching;
        let border_style = if is_active {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title("WATCHING")
            .border_style(border_style);

        let paragraph = Paragraph::new("Watched addresses placeholder").block(block);
        frame.render_widget(paragraph, area);
    }
}
```

---

### Task 2: Wire Dashboard into App

**Files:**
- Modify: `src/app.rs`

**Changes:**
1. Add `Dashboard` view to `View` enum
2. Add `dashboard` field to `App` struct
3. Initialize dashboard in `App::new()`
4. Add view switching between Dashboard and Explorer
5. Handle 'f' key to enter explorer from dashboard
6. Handle 'Esc' to return to dashboard from explorer

---

### Task 3: Implement NODES Panel Content

**Files:**
- Create: `src/modules/dashboard/nodes_panel.rs`
- Modify: `src/modules/dashboard/mod.rs`

**Content:**
- RPC endpoint display
- Node kind (geth/reth/anvil)
- Peer count
- Sync status
- Last RPC latency
- Health indicators

---

### Task 4: Implement ACTIVITY Panel Content

**Files:**
- Create: `src/modules/dashboard/activity_panel.rs`
- Modify: `src/modules/dashboard/mod.rs`

**Content:**
- Last 5 blocks (number, hash, txs count)
- Last 5 transactions (hash, from→to, method)
- Auto-scroll on new items
- Click to select item (updates inspector)

---

### Task 5: Implement INSPECTOR Panel Content

**Files:**
- Create: `src/modules/dashboard/inspector_panel.rs`
- Modify: `src/modules/dashboard/mod.rs`

**Content:**
- Show details of selected block/tx/address
- Display based on Context.selected
- Key-value pairs formatted nicely
- Copy to clipboard with 'y'

---

### Task 6: Implement WATCHING Panel Content

**Files:**
- Create: `src/modules/dashboard/watching_panel.rs`
- Modify: `src/modules/dashboard/mod.rs`

**Content:**
- List watched addresses
- Show labels
- Show last balance/activity
- Add/remove with commands

---

### Task 7: Context Integration

**Files:**
- Modify: `src/core/context.rs`
- Modify: `src/modules/dashboard/mod.rs`

**Changes:**
- Wire up Selected enum
- Update inspector when selection changes
- Clipboard integration for copy operations
- Sync selection between dashboard and explorer

---

## Implementation Order

1. Task 1: Dashboard module structure with placeholders
2. Task 2: Wire dashboard into App, add navigation
3. Task 3: NODES panel content
4. Task 4: ACTIVITY panel content
5. Task 5: INSPECTOR panel content
6. Task 6: WATCHING panel content
7. Task 7: Context integration

---

## Success Criteria

✓ Dashboard view with 4 panels renders on startup
✓ Tab key navigates between panels
✓ Press 'f' to enter Explorer mode
✓ Press 'Esc' in Explorer to return to Dashboard
✓ NODES panel shows node info
✓ ACTIVITY panel shows recent blocks/txs
✓ INSPECTOR panel shows selected item details
✓ WATCHING panel shows watched addresses
✓ Selection syncs between dashboard and explorer

---

## Commits

~8 commits:
1. `feat(dashboard): add dashboard module structure`
2. `feat(dashboard): wire dashboard into App with navigation`
3. `feat(dashboard): implement NODES panel`
4. `feat(dashboard): implement ACTIVITY panel`
5. `feat(dashboard): implement INSPECTOR panel`
6. `feat(dashboard): implement WATCHING panel`
7. `feat(dashboard): add context integration and selection sync`
8. `feat(dashboard): polish UI and add keyboard shortcuts`
