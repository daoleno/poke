//! Dashboard module - panel-based default view

use crate::core::{Action, Context, Module};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

/// Node information for NODES panel
#[derive(Clone, Debug)]
pub struct NodeInfo {
    pub rpc_endpoint: String,
    pub node_kind: String,
    pub peer_count: u32,
    pub sync_progress: f64,
    pub last_rtt_ms: Option<u64>,
}

/// Activity item for ACTIVITY panel
#[derive(Clone, Debug)]
pub struct ActivityItem {
    pub kind: ActivityKind,
    pub display: String,
}

#[derive(Clone, Debug)]
pub enum ActivityKind {
    Block(u64),
    Transaction(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DashboardPanel {
    Nodes,
    Activity,
    Inspector,
    Watching,
}

#[derive(Clone, Debug)]
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

        // Render each panel (placeholders for now - will be updated by draw_dashboard)
        self.render_nodes_panel(frame, top_chunks[0], None);
        self.render_activity_panel(frame, top_chunks[1], None);
        self.render_inspector_panel(frame, bottom_chunks[0]);
        self.render_watching_panel(frame, bottom_chunks[1], None);
    }
}

impl Dashboard {
    /// Render panels with app state data
    pub fn render_with_data(
        &self,
        frame: &mut ratatui::Frame,
        area: Rect,
        node_info: Option<&NodeInfo>,
        activity: Option<&[ActivityItem]>,
        watched: Option<&[String]>,
    ) {
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

        // Render each panel with data
        self.render_nodes_panel(frame, top_chunks[0], node_info);
        self.render_activity_panel(frame, top_chunks[1], activity);
        self.render_inspector_panel(frame, bottom_chunks[0]);
        self.render_watching_panel(frame, bottom_chunks[1], watched);
    }

    fn render_nodes_panel(&self, frame: &mut ratatui::Frame, area: Rect, node_info: Option<&NodeInfo>) {
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

        let lines = if let Some(info) = node_info {
            // Shorten endpoint if too long
            let endpoint = if info.rpc_endpoint.len() > 35 {
                format!("{}...", &info.rpc_endpoint[..32])
            } else {
                info.rpc_endpoint.clone()
            };

            // Format sync progress
            let sync_text = format!("{:.1}%", info.sync_progress * 100.0);
            let sync_color = if info.sync_progress >= 0.99 {
                Color::Green
            } else if info.sync_progress >= 0.5 {
                Color::Yellow
            } else {
                Color::Red
            };

            // Format RTT
            let rtt_text = info.last_rtt_ms
                .map(|ms| format!("{}ms", ms))
                .unwrap_or_else(|| "--".to_string());
            let rtt_color = match info.last_rtt_ms {
                Some(ms) if ms < 100 => Color::Green,
                Some(ms) if ms < 500 => Color::Yellow,
                Some(_) => Color::Red,
                None => Color::DarkGray,
            };

            // Format peer count
            let peer_color = if info.peer_count == 0 {
                Color::Red
            } else if info.peer_count < 3 {
                Color::Yellow
            } else {
                Color::Green
            };

            vec![
                Line::from(vec![
                    Span::styled("RPC: ", Style::default().fg(Color::DarkGray)),
                    Span::raw(endpoint),
                ]),
                Line::from(vec![
                    Span::styled("Node: ", Style::default().fg(Color::DarkGray)),
                    Span::raw(&info.node_kind),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Peers: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(info.peer_count.to_string(), Style::default().fg(peer_color)),
                ]),
                Line::from(vec![
                    Span::styled("Sync: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(sync_text, Style::default().fg(sync_color)),
                ]),
                Line::from(vec![
                    Span::styled("RTT: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(rtt_text, Style::default().fg(rtt_color)),
                ]),
            ]
        } else {
            vec![
                Line::from("No node info available"),
                Line::from(""),
                Line::from(Span::styled(
                    "Connect to a node to see status",
                    Style::default().fg(Color::DarkGray),
                )),
            ]
        };

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, area);
    }

    fn render_activity_panel(&self, frame: &mut ratatui::Frame, area: Rect, activity: Option<&[ActivityItem]>) {
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

        let lines = if let Some(items) = activity {
            if items.is_empty() {
                vec![
                    Line::from("No activity yet"),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Waiting for blocks and transactions...",
                        Style::default().fg(Color::DarkGray),
                    )),
                ]
            } else {
                let mut lines = vec![];

                // Show recent blocks
                let blocks: Vec<_> = items
                    .iter()
                    .filter(|item| matches!(item.kind, ActivityKind::Block(_)))
                    .take(5)
                    .collect();

                if !blocks.is_empty() {
                    lines.push(Line::from(Span::styled(
                        "Recent blocks:",
                        Style::default().fg(Color::LightCyan),
                    )));
                    for item in blocks {
                        if let ActivityKind::Block(num) = item.kind {
                            lines.push(Line::from(vec![
                                Span::raw("  "),
                                Span::styled(format!("#{}", num), Style::default().fg(Color::White)),
                                Span::styled("  ", Style::default().fg(Color::DarkGray)),
                                Span::styled(&item.display, Style::default().fg(Color::DarkGray)),
                            ]));
                        }
                    }
                    lines.push(Line::from(""));
                }

                // Show recent transactions
                let txs: Vec<_> = items
                    .iter()
                    .filter(|item| matches!(item.kind, ActivityKind::Transaction(_)))
                    .take(5)
                    .collect();

                if !txs.is_empty() {
                    lines.push(Line::from(Span::styled(
                        "Recent txs:",
                        Style::default().fg(Color::LightCyan),
                    )));
                    for item in txs {
                        if let ActivityKind::Transaction(ref hash) = item.kind {
                            let short_hash = if hash.len() > 12 {
                                &hash[..12]
                            } else {
                                hash
                            };
                            lines.push(Line::from(vec![
                                Span::raw("  "),
                                Span::styled(short_hash, Style::default().fg(Color::White)),
                                Span::styled("  ", Style::default().fg(Color::DarkGray)),
                                Span::styled(&item.display, Style::default().fg(Color::DarkGray)),
                            ]));
                        }
                    }
                }

                if lines.is_empty() {
                    vec![Line::from("No activity to display")]
                } else {
                    lines
                }
            }
        } else {
            vec![
                Line::from("Activity feed"),
                Line::from(""),
                Line::from(Span::styled(
                    "Recent blocks and transactions will appear here",
                    Style::default().fg(Color::DarkGray),
                )),
            ]
        };

        let paragraph = Paragraph::new(lines).block(block);
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

    fn render_watching_panel(&self, frame: &mut ratatui::Frame, area: Rect, watched: Option<&[String]>) {
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

        let lines = if let Some(addresses) = watched {
            if addresses.is_empty() {
                vec![
                    Line::from("No watched addresses"),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Use :watch <addr> to add",
                        Style::default().fg(Color::DarkGray),
                    )),
                ]
            } else {
                let mut lines = vec![
                    Line::from(Span::styled(
                        format!("Watching {} addresses:", addresses.len()),
                        Style::default().fg(Color::LightCyan),
                    )),
                    Line::from(""),
                ];

                for addr in addresses.iter().take(8) {
                    let short = if addr.len() > 12 {
                        format!("{}..{}", &addr[..6], &addr[addr.len()-4..])
                    } else {
                        addr.clone()
                    };
                    lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled("●", Style::default().fg(Color::Yellow)),
                        Span::raw("  "),
                        Span::styled(short, Style::default().fg(Color::White)),
                    ]));
                }

                if addresses.len() > 8 {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        format!("  +{} more...", addresses.len() - 8),
                        Style::default().fg(Color::DarkGray),
                    )));
                }

                lines
            }
        } else {
            vec![
                Line::from("Watched addresses"),
                Line::from(""),
                Line::from(Span::styled(
                    "Track specific addresses here",
                    Style::default().fg(Color::DarkGray),
                )),
            ]
        };

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, area);
    }
}
