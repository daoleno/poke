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
    /// Selected item index in ACTIVITY panel (None if no selection)
    selected_activity: Option<usize>,
}

impl Dashboard {
    pub fn new() -> Self {
        Self {
            active_panel: DashboardPanel::Nodes,
            selected_activity: None,
        }
    }

    pub fn select_activity(&mut self, index: usize) {
        self.selected_activity = Some(index);
    }

    pub fn clear_selection(&mut self) {
        self.selected_activity = None;
    }

    pub fn selected_activity(&self) -> Option<usize> {
        self.selected_activity
    }

    pub fn move_selection_up(&mut self) {
        if let Some(idx) = self.selected_activity {
            if idx > 0 {
                self.selected_activity = Some(idx - 1);
            }
        } else {
            self.selected_activity = Some(0);
        }
    }

    pub fn move_selection_down(&mut self, max: usize) {
        if let Some(idx) = self.selected_activity {
            if idx + 1 < max {
                self.selected_activity = Some(idx + 1);
            }
        } else {
            self.selected_activity = Some(0);
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
            KeyCode::Up | KeyCode::Char('k') => {
                // Navigate up in Activity panel
                if self.active_panel == DashboardPanel::Activity {
                    self.move_selection_up();
                }
                Action::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                // Navigate down in Activity panel
                if self.active_panel == DashboardPanel::Activity {
                    // Max will be set by render based on actual item count
                    // For now, just move down without limit check
                    if let Some(idx) = self.selected_activity {
                        self.selected_activity = Some(idx + 1);
                    } else {
                        self.selected_activity = Some(0);
                    }
                }
                Action::None
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
        self.render_inspector_panel(frame, bottom_chunks[0], None);
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

        // Get selected activity item for inspector
        let selected_item = activity
            .and_then(|items| self.selected_activity.and_then(|idx| items.get(idx)));

        // Render each panel with data
        self.render_nodes_panel(frame, top_chunks[0], node_info);
        self.render_activity_panel(frame, top_chunks[1], activity);
        self.render_inspector_panel(frame, bottom_chunks[0], selected_item);
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
                lines.push(Line::from(Span::styled(
                    "Recent activity (↑↓ to select):",
                    Style::default().fg(Color::LightCyan),
                )));
                lines.push(Line::from(""));

                // Render all items in order with selection highlighting
                for (idx, item) in items.iter().enumerate() {
                    let is_selected = self.selected_activity == Some(idx);

                    match &item.kind {
                        ActivityKind::Block(num) => {
                            let mut spans = vec![
                                Span::raw("  "),
                                Span::styled("Block", Style::default().fg(Color::DarkGray)),
                                Span::raw(" "),
                            ];

                            let num_style = if is_selected {
                                Style::default().fg(Color::Black).bg(Color::Cyan)
                            } else {
                                Style::default().fg(Color::White)
                            };

                            spans.push(Span::styled(format!("#{}", num), num_style));
                            spans.push(Span::raw(" "));

                            let display_style = if is_selected {
                                Style::default().fg(Color::DarkGray).bg(Color::Cyan)
                            } else {
                                Style::default().fg(Color::DarkGray)
                            };

                            spans.push(Span::styled(&item.display, display_style));

                            lines.push(Line::from(spans));
                        }
                        ActivityKind::Transaction(hash) => {
                            let short_hash = if hash.len() > 12 {
                                &hash[..12]
                            } else {
                                hash
                            };

                            let mut spans = vec![
                                Span::raw("  "),
                                Span::styled("Tx", Style::default().fg(Color::DarkGray)),
                                Span::raw("    "),
                            ];

                            let hash_style = if is_selected {
                                Style::default().fg(Color::Black).bg(Color::Cyan)
                            } else {
                                Style::default().fg(Color::White)
                            };

                            spans.push(Span::styled(short_hash, hash_style));
                            spans.push(Span::raw(" "));

                            let display_style = if is_selected {
                                Style::default().fg(Color::DarkGray).bg(Color::Cyan)
                            } else {
                                Style::default().fg(Color::DarkGray)
                            };

                            spans.push(Span::styled(&item.display, display_style));

                            lines.push(Line::from(spans));
                        }
                    }
                }

                if lines.len() == 2 {
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

    fn render_inspector_panel(&self, frame: &mut ratatui::Frame, area: Rect, selected_item: Option<&ActivityItem>) {
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

        let lines = if let Some(item) = selected_item {
            // Show details of selected activity item
            match &item.kind {
                ActivityKind::Block(num) => {
                    vec![
                        Line::from(Span::styled(
                            "Block Details",
                            Style::default().fg(Color::LightCyan),
                        )),
                        Line::from(""),
                        Line::from(vec![
                            Span::styled("Number: ", Style::default().fg(Color::DarkGray)),
                            Span::styled(format!("#{}", num), Style::default().fg(Color::White)),
                        ]),
                        Line::from(vec![
                            Span::styled("Info: ", Style::default().fg(Color::DarkGray)),
                            Span::styled(&item.display, Style::default().fg(Color::White)),
                        ]),
                        Line::from(""),
                        Line::from(Span::styled(
                            "Actions:",
                            Style::default().fg(Color::LightCyan),
                        )),
                        Line::from(vec![
                            Span::styled("Enter", Style::default().fg(Color::Yellow)),
                            Span::raw("  - View block details"),
                        ]),
                        Line::from(vec![
                            Span::styled("f", Style::default().fg(Color::Yellow)),
                            Span::raw("      - Go to Explorer"),
                        ]),
                    ]
                }
                ActivityKind::Transaction(hash) => {
                    let short_hash = if hash.len() > 20 {
                        format!("{}..{}", &hash[..10], &hash[hash.len()-8..])
                    } else {
                        hash.clone()
                    };

                    vec![
                        Line::from(Span::styled(
                            "Transaction Details",
                            Style::default().fg(Color::LightCyan),
                        )),
                        Line::from(""),
                        Line::from(vec![
                            Span::styled("Hash: ", Style::default().fg(Color::DarkGray)),
                            Span::styled(short_hash, Style::default().fg(Color::White)),
                        ]),
                        Line::from(vec![
                            Span::styled("Method: ", Style::default().fg(Color::DarkGray)),
                            Span::styled(&item.display, Style::default().fg(Color::White)),
                        ]),
                        Line::from(""),
                        Line::from(Span::styled(
                            "Actions:",
                            Style::default().fg(Color::LightCyan),
                        )),
                        Line::from(vec![
                            Span::styled("Enter", Style::default().fg(Color::Yellow)),
                            Span::raw("  - View tx details"),
                        ]),
                        Line::from(vec![
                            Span::styled("f", Style::default().fg(Color::Yellow)),
                            Span::raw("      - Go to Explorer"),
                        ]),
                    ]
                }
            }
        } else {
            // Show quick actions when nothing is selected
            vec![
                Line::from(Span::styled(
                    "Quick Actions",
                    Style::default().fg(Color::LightCyan),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Tab", Style::default().fg(Color::Yellow)),
                    Span::raw("  - Navigate panels"),
                ]),
                Line::from(vec![
                    Span::styled("f", Style::default().fg(Color::Yellow)),
                    Span::raw("    - Enter Explorer"),
                ]),
                Line::from(vec![
                    Span::styled(":", Style::default().fg(Color::Yellow)),
                    Span::raw("    - Command mode"),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "Common Commands:",
                    Style::default().fg(Color::LightCyan),
                )),
                Line::from(vec![
                    Span::styled(":health", Style::default().fg(Color::Green)),
                    Span::styled("  - Node health", Style::default().fg(Color::DarkGray)),
                ]),
                Line::from(vec![
                    Span::styled(":peers", Style::default().fg(Color::Green)),
                    Span::styled("   - Peer info", Style::default().fg(Color::DarkGray)),
                ]),
                Line::from(vec![
                    Span::styled(":metrics", Style::default().fg(Color::Green)),
                    Span::styled("  - Show metrics", Style::default().fg(Color::DarkGray)),
                ]),
            ]
        };

        let paragraph = Paragraph::new(lines).block(block);
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
