//! Tab-based UI rendering

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Tabs as RataTabs};
use ratatui::Frame;

use crate::app::{App, OpsSection, Tab, ToolkitTool};

/// Draw the tab bar at the top
pub fn draw_tab_bar(f: &mut Frame, area: Rect, app: &App) {
    let titles: Vec<Line> = Tab::ALL
        .iter()
        .map(|tab| {
            let shortcut = tab.shortcut();
            let title = tab.title();
            Line::from(vec![
                Span::styled(
                    format!("{}:", shortcut),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw(title),
            ])
        })
        .collect();

    let selected = Tab::ALL.iter().position(|t| *t == app.current_tab).unwrap_or(0);

    let tabs = RataTabs::new(titles)
        .select(selected)
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .divider(" │ ");

    f.render_widget(tabs, area);
}

/// Draw the Home tab content
pub fn draw_home_tab(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Status bar
            Constraint::Min(10),    // Main content
        ])
        .split(area);

    // Status bar
    draw_status_bar(f, chunks[0], app);

    // Main content: Quick Access + Live Feed
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(chunks[1]);

    draw_quick_access(f, main_chunks[0], app);
    draw_live_feed(f, main_chunks[1], app);
}

fn draw_status_bar(f: &mut Frame, area: Rect, app: &App) {
    let status_color = if app.sync_progress >= 0.99 {
        Color::Green
    } else {
        Color::Yellow
    };

    let rtt_str = app
        .last_rtt_ms
        .map(|ms| format!("{}ms", ms))
        .unwrap_or_else(|| "--".to_string());

    let content = Line::from(vec![
        Span::styled("●", Style::default().fg(status_color)),
        Span::raw(" "),
        Span::styled(&app.node_kind, Style::default().fg(Color::White)),
        Span::raw(" @ "),
        Span::styled(&app.rpc_endpoint, Style::default().fg(Color::Cyan)),
        Span::raw("   "),
        Span::styled("Block ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("#{}", app.blocks.first().map(|b| b.number).unwrap_or(0)),
            Style::default().fg(Color::White),
        ),
        Span::raw("   "),
        Span::styled("Gas ", Style::default().fg(Color::DarkGray)),
        Span::raw("-- gwei"),
        Span::raw("   "),
        Span::styled("RTT ", Style::default().fg(Color::DarkGray)),
        Span::raw(rtt_str),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title("STATUS");

    let paragraph = Paragraph::new(content).block(block);
    f.render_widget(paragraph, area);
}

fn draw_quick_access(f: &mut Frame, area: Rect, _app: &App) {
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            " Navigation",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("  b", Style::default().fg(Color::Yellow)),
            Span::raw("  Blocks"),
        ]),
        Line::from(vec![
            Span::styled("  t", Style::default().fg(Color::Yellow)),
            Span::raw("  Transactions"),
        ]),
        Line::from(vec![
            Span::styled("  a", Style::default().fg(Color::Yellow)),
            Span::raw("  Address lookup"),
        ]),
        Line::from(vec![
            Span::styled("  r", Style::default().fg(Color::Yellow)),
            Span::raw("  Trace transaction"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            " Toolkit",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("  :encode", Style::default().fg(Color::Green)),
            Span::raw("   "),
            Span::styled(":decode", Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("  :hash", Style::default().fg(Color::Green)),
            Span::raw("     "),
            Span::styled(":convert", Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("  :selector", Style::default().fg(Color::Green)),
            Span::raw(" "),
            Span::styled(":4byte", Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("  :call", Style::default().fg(Color::Green)),
            Span::raw("     "),
            Span::styled(":gas", Style::default().fg(Color::Green)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw(" Press "),
            Span::styled(":", Style::default().fg(Color::Yellow)),
            Span::raw(" for command mode"),
        ]),
        Line::from(vec![
            Span::raw(" Press "),
            Span::styled("?", Style::default().fg(Color::Yellow)),
            Span::raw(" for help"),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title("QUICK ACCESS");

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}

fn draw_live_feed(f: &mut Frame, area: Rect, app: &App) {
    let mut lines = vec![Line::from("")];

    // Add recent blocks with their transactions
    for block in app.blocks.iter().rev().take(6) {
        lines.push(Line::from(vec![
            Span::styled(" ├─ ", Style::default().fg(Color::DarkGray)),
            Span::styled("Block ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("#{}", block.number),
                Style::default().fg(Color::White),
            ),
            Span::raw("  "),
            Span::styled(
                format!("{} txs", block.tx_count),
                Style::default().fg(Color::DarkGray),
            ),
        ]));

        // Show some transactions for this block
        for tx in app.txs.iter().filter(|t| t.block_number == block.number).take(2) {
            let short_hash = if tx.hash.len() > 10 {
                format!("{}...", &tx.hash[..10])
            } else {
                tx.hash.clone()
            };
            lines.push(Line::from(vec![
                Span::raw(" │   "),
                Span::styled(short_hash, Style::default().fg(Color::DarkGray)),
                Span::raw(" "),
                Span::styled(&tx.method, Style::default().fg(Color::White)),
            ]));
        }
    }

    // Watching section
    if !app.watched_addresses.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!(" Watching ({} addresses)", app.watched_addresses.len()),
            Style::default().fg(Color::Cyan),
        )));
        for addr in app.watched_addresses.iter().take(3) {
            let short = if addr.len() > 14 {
                format!("{}..{}", &addr[..6], &addr[addr.len() - 4..])
            } else {
                addr.clone()
            };
            lines.push(Line::from(vec![
                Span::styled(" ● ", Style::default().fg(Color::Yellow)),
                Span::styled(short, Style::default().fg(Color::White)),
            ]));
        }
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title("LIVE FEED");

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}

/// Draw the Toolkit tab content
pub fn draw_toolkit_tab(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(14),  // Sidebar
            Constraint::Min(40),     // Tool content
        ])
        .split(area);

    draw_toolkit_sidebar(f, chunks[0], app);
    draw_toolkit_content(f, chunks[1], app);
}

fn draw_toolkit_sidebar(f: &mut Frame, area: Rect, app: &App) {
    let mut lines = vec![Line::from("")];

    let mut current_category = "";
    for tool in ToolkitTool::ALL {
        let category = tool.category();
        if category != current_category {
            if !current_category.is_empty() {
                lines.push(Line::from(""));
            }
            lines.push(Line::from(Span::styled(
                format!(" {}", category),
                Style::default().fg(Color::Cyan),
            )));
            current_category = category;
        }

        let is_selected = app.toolkit_state.selected_tool == tool;
        let style = if is_selected {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let prefix = if is_selected { "▸" } else { " " };
        lines.push(Line::from(Span::styled(
            format!(" {}{}", prefix, tool.title()),
            style,
        )));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title("TOOLS");

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}

fn draw_toolkit_content(f: &mut Frame, area: Rect, app: &App) {
    let tool_name = app.toolkit_state.selected_tool.title().to_uppercase();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),   // Input
            Constraint::Length(8),   // Output
            Constraint::Min(3),      // History
        ])
        .split(area);

    // Input area
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(format!("{} - Input", tool_name));

    let input_text = if app.toolkit_state.input.is_empty() {
        Span::styled(
            " Type input and press Enter...",
            Style::default().fg(Color::DarkGray),
        )
    } else {
        Span::raw(format!(" {}", &app.toolkit_state.input))
    };

    let input_para = Paragraph::new(Line::from(input_text)).block(input_block);
    f.render_widget(input_para, chunks[0]);

    // Output area
    let output_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title("Result");

    let output_text = if app.toolkit_state.output.is_empty() {
        Span::styled(" (no output)", Style::default().fg(Color::DarkGray))
    } else {
        Span::styled(&app.toolkit_state.output, Style::default().fg(Color::Green))
    };

    let output_para = Paragraph::new(Line::from(output_text)).block(output_block);
    f.render_widget(output_para, chunks[1]);

    // History area
    let history_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title("History");

    let mut history_lines = vec![];
    for (i, cmd) in app.toolkit_state.history.iter().rev().take(5).enumerate() {
        let prefix = if Some(i) == app.toolkit_state.history_index {
            "▸"
        } else {
            " "
        };
        history_lines.push(Line::from(Span::styled(
            format!(" {}{}", prefix, cmd),
            Style::default().fg(Color::DarkGray),
        )));
    }

    let history_para = Paragraph::new(history_lines).block(history_block);
    f.render_widget(history_para, chunks[2]);
}

/// Draw the Ops tab content
pub fn draw_ops_tab(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(14),  // Sidebar
            Constraint::Min(40),     // Content
        ])
        .split(area);

    draw_ops_sidebar(f, chunks[0], app);
    draw_ops_content(f, chunks[1], app);
}

fn draw_ops_sidebar(f: &mut Frame, area: Rect, app: &App) {
    let mut lines = vec![Line::from("")];

    for section in OpsSection::ALL {
        let is_selected = app.ops_section == section;
        let style = if is_selected {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let prefix = if is_selected { "▸" } else { " " };
        lines.push(Line::from(Span::styled(
            format!(" {}{}", prefix, section.title()),
            style,
        )));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title("OPS");

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}

fn draw_ops_content(f: &mut Frame, area: Rect, app: &App) {
    let lines = match app.ops_section {
        OpsSection::Health => {
            let status_color = if app.sync_progress >= 0.99 {
                Color::Green
            } else {
                Color::Yellow
            };
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    " NODE HEALTH",
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled(" Status:   ", Style::default().fg(Color::DarkGray)),
                    Span::styled("●", Style::default().fg(status_color)),
                    Span::raw(if app.sync_progress >= 0.99 { " Connected" } else { " Syncing" }),
                ]),
                Line::from(vec![
                    Span::styled(" Endpoint: ", Style::default().fg(Color::DarkGray)),
                    Span::raw(&app.rpc_endpoint),
                ]),
                Line::from(vec![
                    Span::styled(" Node:     ", Style::default().fg(Color::DarkGray)),
                    Span::raw(&app.node_kind),
                ]),
                Line::from(vec![
                    Span::styled(" Sync:     ", Style::default().fg(Color::DarkGray)),
                    Span::raw(format!("{:.1}%", app.sync_progress * 100.0)),
                ]),
                Line::from(vec![
                    Span::styled(" RTT:      ", Style::default().fg(Color::DarkGray)),
                    Span::raw(
                        app.last_rtt_ms
                            .map(|ms| format!("{}ms", ms))
                            .unwrap_or_else(|| "--".to_string()),
                    ),
                ]),
                Line::from(vec![
                    Span::styled(" Peers:    ", Style::default().fg(Color::DarkGray)),
                    Span::raw(format!("{}", app.peer_count)),
                ]),
            ]
        }
        OpsSection::Peers => {
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    " PEERS",
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled(" Connected: ", Style::default().fg(Color::DarkGray)),
                    Span::raw(format!("{} peers", app.peer_count)),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    " (Detailed peer info requires node API)",
                    Style::default().fg(Color::DarkGray),
                )),
            ]
        }
        OpsSection::RpcStats => {
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    " RPC STATS",
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    " (RPC call statistics will appear here)",
                    Style::default().fg(Color::DarkGray),
                )),
            ]
        }
        _ => {
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!(" {}", app.ops_section.title().to_uppercase()),
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    " (Content coming soon)",
                    Style::default().fg(Color::DarkGray),
                )),
            ]
        }
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(app.ops_section.title().to_uppercase());

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}

/// Draw the Anvil tab content
pub fn draw_anvil_tab(f: &mut Frame, area: Rect, app: &App) {
    let is_anvil = app.node_kind.to_lowercase().contains("anvil");

    if !is_anvil {
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                " Not connected to Anvil",
                Style::default().fg(Color::Yellow),
            )),
            Line::from(""),
            Line::from(Span::styled(
                " This tab is only available when connected to an Anvil dev node.",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                " Use :connect <url> to connect to an Anvil instance.",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title("ANVIL");

        let paragraph = Paragraph::new(lines).block(block);
        f.render_widget(paragraph, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),   // Quick actions
            Constraint::Min(10),     // Snapshots and accounts
        ])
        .split(area);

    // Quick actions
    let actions_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title("QUICK ACTIONS");

    let actions_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  [m]", Style::default().fg(Color::Yellow)),
            Span::raw(" Mine 1 block    "),
            Span::styled("[M]", Style::default().fg(Color::Yellow)),
            Span::raw(" Mine N blocks..."),
        ]),
        Line::from(vec![
            Span::styled("  [s]", Style::default().fg(Color::Yellow)),
            Span::raw(" Snapshot        "),
            Span::styled("[r]", Style::default().fg(Color::Yellow)),
            Span::raw(" Revert to snapshot"),
        ]),
        Line::from(vec![
            Span::styled("  [i]", Style::default().fg(Color::Yellow)),
            Span::raw(" Impersonate     "),
            Span::styled("[I]", Style::default().fg(Color::Yellow)),
            Span::raw(" Stop impersonate"),
        ]),
        Line::from(vec![
            Span::styled("  [+]", Style::default().fg(Color::Yellow)),
            Span::raw(" Set balance     "),
            Span::styled("[T]", Style::default().fg(Color::Yellow)),
            Span::raw(" Set timestamp"),
        ]),
    ];

    let actions_para = Paragraph::new(actions_lines).block(actions_block);
    f.render_widget(actions_para, chunks[0]);

    // Lower section: Snapshots
    let lower_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    // Snapshots
    let snap_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title("SNAPSHOTS");

    let snap_lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            " No snapshots yet",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            " Press [s] to create a snapshot",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let snap_para = Paragraph::new(snap_lines).block(snap_block);
    f.render_widget(snap_para, lower_chunks[0]);

    // Impersonating
    let imp_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title("IMPERSONATING");

    let imp_lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            " No accounts impersonated",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            " Press [i] to impersonate an address",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let imp_para = Paragraph::new(imp_lines).block(imp_block);
    f.render_widget(imp_para, lower_chunks[1]);
}
