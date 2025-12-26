//! Dashboard module - panel-based default view

use crate::core::{Action, Context, Module};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph};

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
    fn id(&self) -> &'static str {
        "dashboard"
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent, _ctx: &mut Context) -> Action {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Tab => {
                self.next_panel();
                Action::None
            }
            KeyCode::Char('f') => {
                // Enter full-screen explorer
                Action::Navigate(crate::core::NavigateTarget::Blocks)
            }
            _ => Action::None,
        }
    }

    fn handle_command(&mut self, _cmd: &crate::core::Command, _ctx: &mut Context) -> Action {
        Action::None
    }

    fn render(&self, frame: &mut ratatui::Frame, area: Rect, _ctx: &Context) {
        // Split into 4 quadrants (2x2 grid)
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

        // Render each panel
        self.render_nodes_panel(frame, top_chunks[0]);
        self.render_activity_panel(frame, top_chunks[1]);
        self.render_inspector_panel(frame, bottom_chunks[0]);
        self.render_watching_panel(frame, bottom_chunks[1]);
    }
}

impl Dashboard {
    fn render_nodes_panel(&self, frame: &mut ratatui::Frame, area: Rect) {
        let is_active = self.active_panel == DashboardPanel::Nodes;
        let border_style = if is_active {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title("NODES")
            .border_style(border_style);

        let content = vec![
            "Node info will appear here",
            "",
            "- RPC endpoint",
            "- Node kind",
            "- Peer count",
            "- Sync status",
        ]
        .join("\n");

        let paragraph = Paragraph::new(content).block(block);
        frame.render_widget(paragraph, area);
    }

    fn render_activity_panel(&self, frame: &mut ratatui::Frame, area: Rect) {
        let is_active = self.active_panel == DashboardPanel::Activity;
        let border_style = if is_active {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title("ACTIVITY")
            .border_style(border_style);

        let content = vec![
            "Recent blocks/transactions",
            "",
            "Last 5 blocks:",
            "- Block #12345",
            "- Block #12344",
            "",
            "Last 5 transactions:",
            "- 0xabcd...",
        ]
        .join("\n");

        let paragraph = Paragraph::new(content).block(block);
        frame.render_widget(paragraph, area);
    }

    fn render_inspector_panel(&self, frame: &mut ratatui::Frame, area: Rect) {
        let is_active = self.active_panel == DashboardPanel::Inspector;
        let border_style = if is_active {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title("INSPECTOR")
            .border_style(border_style);

        let content = vec![
            "Selected item details",
            "",
            "Click on an item in ACTIVITY",
            "to view its details here",
            "",
            "Press 'y' to copy values",
        ]
        .join("\n");

        let paragraph = Paragraph::new(content).block(block);
        frame.render_widget(paragraph, area);
    }

    fn render_watching_panel(&self, frame: &mut ratatui::Frame, area: Rect) {
        let is_active = self.active_panel == DashboardPanel::Watching;
        let border_style = if is_active {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title("WATCHING")
            .border_style(border_style);

        let content = vec![
            "Watched addresses",
            "",
            "Use :watch <addr> to add",
            "Use :unwatch <addr> to remove",
            "",
            "No addresses watched yet",
        ]
        .join("\n");

        let paragraph = Paragraph::new(content).block(block);
        frame.render_widget(paragraph, area);
    }
}
