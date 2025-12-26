use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

pub mod layout;
pub mod widgets;

use crate::app::{
    AddressKind, App, CallStatus, Focus, InputMode, ListKind, PromptKind, Section, StatusLevel,
    TxStatus, View,
};
use crate::config;

pub fn draw(f: &mut Frame, app: &mut App) {
    let size = f.size();

    // Check if we're in dashboard view
    if app.current_view() == View::Dashboard {
        // Render dashboard (full screen, no header/sidebar)
        draw_dashboard(f, size, app);
        return;
    }

    // Original explorer layout
    let areas = layout::areas(size);

    draw_header(f, areas.header, app);
    draw_sidebar(f, areas.sidebar_sections, areas.sidebar_watch, app);
    draw_list_panel(f, areas.list, app);
    draw_detail_panel(f, areas.details, app);
    draw_status_line(f, areas.status_line, app);
    draw_command_line(f, areas.command_line, app);

    if app.help_open {
        draw_help_popup(f, areas.size, app);
    }
    if app.settings_open {
        draw_settings_popup(f, areas.size, app);
    }
}

fn draw_dashboard(f: &mut Frame, area: Rect, app: &mut App) {
    use crate::core::Module;
    use crate::modules::dashboard::{ActivityItem, ActivityKind, NodeInfo};

    // Split area for dashboard and status/command line
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(10),
            Constraint::Length(1),  // status line
            Constraint::Length(1),  // command line
        ])
        .split(area);

    // Prepare node info
    let node_info = NodeInfo {
        rpc_endpoint: app.rpc_endpoint.clone(),
        node_kind: app.node_kind.clone(),
        peer_count: app.peer_count,
        sync_progress: app.sync_progress,
        last_rtt_ms: app.last_rtt_ms,
    };

    // Prepare activity items (last 5 blocks + last 5 txs)
    let mut activity_items = Vec::new();

    // Add recent blocks
    for block in app.blocks.iter().rev().take(5) {
        activity_items.push(ActivityItem {
            kind: ActivityKind::Block(block.number),
            display: format!("{} txs", block.tx_count),
        });
    }

    // Add recent transactions
    for tx in app.txs.iter().rev().take(5) {
        activity_items.push(ActivityItem {
            kind: ActivityKind::Transaction(tx.hash.clone()),
            display: tx.method.clone(),
        });
    }

    // Prepare watched addresses
    let watched: Vec<String> = app.watched_addresses.iter().cloned().collect();

    // Update context with selected activity item (for Enter key navigation)
    if let Some(idx) = app.dashboard.selected_activity() {
        if let Some(item) = activity_items.get(idx) {
            use crate::core::Selected;
            app.ctx.selected = match &item.kind {
                ActivityKind::Block(num) => Selected::Block(*num),
                ActivityKind::Transaction(hash) => Selected::Transaction(hash.clone()),
            };
        } else {
            app.ctx.selected = crate::core::Selected::None;
        }
    } else {
        app.ctx.selected = crate::core::Selected::None;
    }

    // Render dashboard with data
    let dashboard = app.dashboard.clone();
    dashboard.render_with_data(
        f,
        chunks[0],
        Some(&node_info),
        Some(&activity_items),
        Some(&watched),
    );

    // Render status and command line
    draw_status_line(f, chunks[1], app);
    draw_command_line(f, chunks[2], app);

    if app.help_open {
        draw_help_popup(f, area, app);
    }
    if app.settings_open {
        draw_settings_popup(f, area, app);
    }
}

fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(area);

    let title = Line::from(vec![
        Span::styled(
            "Poke",
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled("RPC", Style::default().fg(Color::DarkGray)),
        Span::raw(format!(" {} ", app.rpc_endpoint)),
        Span::styled("Node", Style::default().fg(Color::DarkGray)),
        Span::raw(format!(" {} ", app.node_kind)),
        Span::styled("Focus", Style::default().fg(Color::DarkGray)),
        Span::raw(format!(" {}", app.focus_label())),
    ]);

    let left = Paragraph::new(title)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Left);

    let rtt = app
        .last_rtt_ms
        .map(|ms| format!("{ms}ms"))
        .unwrap_or_else(|| "--".to_string());
    let sync = format!("{:.0}%", app.sync_progress * 100.0);
    let right_line = Line::from(vec![
        Span::styled("RTT ", Style::default().fg(Color::DarkGray)),
        Span::raw(format!("{}  ", rtt)),
        Span::styled("Peers ", Style::default().fg(Color::DarkGray)),
        Span::raw(format!("{}  ", app.peer_count)),
        Span::styled("Sync ", Style::default().fg(Color::DarkGray)),
        Span::raw(sync),
    ]);
    let right = Paragraph::new(right_line)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Left);

    f.render_widget(left, chunks[0]);
    f.render_widget(right, chunks[1]);
}

fn draw_sidebar(f: &mut Frame, sections_area: Rect, watch_area: Rect, app: &App) {
    let border_style = if app.focus == Focus::Sidebar {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let items: Vec<ListItem> = Section::ALL
        .iter()
        .map(|section| {
            let is_active = *section == app.active_section;
            let mut spans = vec![Span::raw(section.title())];
            if is_active {
                spans.push(Span::raw(" *"));
            }
            let style = if is_active {
                Style::default()
                    .fg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Line::from(spans)).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Sections")
                .border_style(border_style),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol("-> ");

    let mut state = ListState::default();
    state.select(Some(
        Section::ALL
            .iter()
            .position(|section| *section == app.active_section)
            .unwrap_or(0),
    ));
    f.render_stateful_widget(list, sections_area, &mut state);

    let watched = if app.watched_addresses.is_empty() {
        vec![Line::from("No watch targets")]
    } else {
        app.watched_addresses
            .iter()
            .take(5)
            .map(|addr| Line::from(short_addr(addr)))
            .collect()
    };
    let watch_block = Paragraph::new(Text::from(watched))
        .block(Block::default().borders(Borders::ALL).title("Watch"))
        .wrap(Wrap { trim: true });

    f.render_widget(watch_block, watch_area);
}

fn draw_list_panel(f: &mut Frame, area: Rect, app: &App) {
    let title = list_title(app);
    let border_style = if app.focus == Focus::List {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let (items, selected) = match app.list_kind() {
        ListKind::Blocks => (block_items(app), app.selected_block),
        ListKind::Transactions => (tx_items(app), app.selected_tx),
        ListKind::Addresses => (address_items(app), app.selected_address),
        ListKind::Contracts => (contract_items(app), app.selected_contract),
        ListKind::Trace => (trace_items(app), app.selected_trace),
    };

    let highlight_style = if app.focus == Focus::List {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().add_modifier(Modifier::BOLD)
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(border_style),
        )
        .highlight_style(highlight_style)
        .highlight_symbol(">> ");

    let mut state = ListState::default();
    if !app.list_is_empty() {
        state.select(Some(selected));
    }
    f.render_stateful_widget(list, area, &mut state);
}

fn draw_detail_panel(f: &mut Frame, area: Rect, app: &App) {
    let border_style = if app.focus == Focus::Details {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let title = match app.current_view() {
        View::Dashboard => "Dashboard",  // Not used in dashboard view
        View::Overview => match app.active_section {
            Section::Overview => "Overview",
            Section::Blocks => "Block Inspector",
            Section::Transactions => "Tx Inspector",
            Section::Addresses => "Address Inspector",
            Section::Contracts => "Contract Inspector",
        },
        View::BlockDetail => "Block Detail",
        View::TxDetail => "Transaction Detail",
        View::AddressDetail => "Address Detail",
        View::ContractDetail => "Contract Detail",
        View::Trace => "Trace Detail",
    };

    let mut lines = match app.current_view() {
        View::Dashboard => Vec::new(),  // Not used in dashboard view
        View::Overview => match app.active_section {
            Section::Overview => overview_lines(app),
            Section::Blocks => block_inspector_lines(app),
            Section::Transactions => tx_inspector_lines(app),
            Section::Addresses => address_inspector_lines(app),
            Section::Contracts => contract_inspector_lines(app),
        },
        View::BlockDetail => block_browser_lines(app),
        View::TxDetail => tx_detail_lines(app),
        View::AddressDetail => address_browser_lines(app),
        View::ContractDetail => contract_browser_lines(app),
        View::Trace => trace_detail_lines(app),
    };

    if lines.is_empty() {
        lines.push(Line::from("No data"));
    }

    let paragraph = Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(border_style),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn draw_status_line(f: &mut Frame, area: Rect, app: &App) {
    let latest = app
        .blocks
        .last()
        .map(|b| b.number.to_string())
        .unwrap_or_else(|| "--".to_string());
    let mut spans = vec![
        Span::styled("Latest ", Style::default().fg(Color::DarkGray)),
        Span::raw(format!("{}  ", latest)),
        Span::styled("Section ", Style::default().fg(Color::DarkGray)),
        Span::raw(format!("{}  ", app.active_section.title())),
        Span::styled("View ", Style::default().fg(Color::DarkGray)),
        Span::raw(app.view_breadcrumb()),
    ];
    if let Some(registry) = app.abi_registry.as_ref() {
        spans.push(Span::raw("  "));
        spans.push(Span::styled("ABI ", Style::default().fg(Color::DarkGray)));
        spans.push(Span::raw(registry.len().to_string()));
    }
    if !app.tokens.is_empty() {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            "Tokens ",
            Style::default().fg(Color::DarkGray),
        ));
        spans.push(Span::raw(app.tokens.len().to_string()));
    }
    if let Some(filter) = app.active_filter.as_ref() {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            "Filter ",
            Style::default().fg(Color::DarkGray),
        ));
        spans.push(Span::raw(filter.raw.clone()));
    }

    let line = Line::from(spans);

    let paragraph = Paragraph::new(line)
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);
}

/// Get command hint for autocompletion
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

fn draw_command_line(f: &mut Frame, area: Rect, app: &App) {
    let content = match app.input_mode {
        InputMode::Command => {
            let hint = command_hint(&app.command.input);
            let hint_text = hint.unwrap_or("search: block/addr/tx | filter: key:value");
            Line::from(vec![
                Span::styled(": ", Style::default().fg(Color::Yellow)),
                Span::raw(&app.command.input),
                Span::styled(
                    format!("  {}", hint_text),
                    Style::default().fg(Color::DarkGray),
                ),
            ])
        }
        InputMode::Prompt(PromptKind::StorageSlot) => {
            let target = app
                .prompt_context
                .as_deref()
                .map(short_addr)
                .unwrap_or_else(|| "--".to_string());
            Line::from(vec![
                Span::styled("> slot ", Style::default().fg(Color::LightCyan)),
                Span::raw(&app.command.input),
                Span::styled(
                    format!("  (contract: {target}, Enter=ok Esc=cancel)"),
                    Style::default().fg(Color::DarkGray),
                ),
            ])
        }
        InputMode::Prompt(PromptKind::Label) => {
            let target = app
                .prompt_context
                .as_deref()
                .map(short_addr)
                .unwrap_or_else(|| "--".to_string());
            Line::from(vec![
                Span::styled("> label ", Style::default().fg(Color::LightCyan)),
                Span::raw(&app.command.input),
                Span::styled(
                    format!("  (addr: {target}, empty=clear, Enter=ok Esc=cancel)"),
                    Style::default().fg(Color::DarkGray),
                ),
            ])
        }
        InputMode::Normal => {
            if let Some((text, level)) = app.status_text() {
                let color = match level {
                    StatusLevel::Info => Color::LightGreen,
                    StatusLevel::Warn => Color::LightYellow,
                    StatusLevel::Error => Color::LightRed,
                };
                Line::from(vec![
                    Span::styled("msg: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(text, Style::default().fg(color)),
                ])
            } else {
                action_hints(app)
            }
        }
    };

    let paragraph = Paragraph::new(content).style(Style::default().fg(Color::White));
    f.render_widget(paragraph, area);
}

fn draw_help_popup(f: &mut Frame, area: Rect, app: &App) {
    let popup_area = centered_rect(72, 64, area);
    f.render_widget(Clear, popup_area);

    let lines = vec![
        Line::from("Navigation"),
        Line::from("  Tab / h / l Cycle focus (vim)"),
        Line::from("  j / k      Move selection (vim)"),
        Line::from("  gg / G     Top / bottom (vim)"),
        Line::from("  Ctrl-u/d   Half page up/down (vim)"),
        Line::from("  Ctrl-b/f   Page up/down (vim)"),
        Line::from("  [ / ]      Prev/Next section"),
        Line::from("  1-5        Jump to section"),
        Line::from("  Enter      Drill / open"),
        Line::from("  Esc        Back / close"),
        Line::from("  Mouse      Scroll + click select"),
        Line::from(""),
        Line::from("Actions"),
        Line::from("  /          Search / filter"),
        Line::from("  Space      Pause/Resume"),
        Line::from("  f          Pin block (Blocks)"),
        Line::from("  s          Settings"),
        Line::from("  a          Reload ABI (Settings)"),
        Line::from("  [ / ]      Switch RPC (Settings)"),
        Line::from("  w          Watch address"),
        Line::from("  n          Label address"),
        Line::from("  p          Poke balance"),
        Line::from("  o          Storage slot (Contract)"),
        Line::from("  t          Trace view (Tx)"),
        Line::from("  e          Expand/collapse trace"),
        Line::from("  r          Refresh"),
        Line::from("  ?          Toggle help"),
        Line::from("  q          Quit"),
        Line::from(""),
        Line::from("Search examples:"),
        Line::from("  / 12345678"),
        Line::from("  / 0x<40-hex-address>"),
        Line::from("  / 0x<64-hex-txhash>"),
        Line::from(""),
        Line::from("Filter examples:"),
        Line::from("  / addr:0x.. method:swap status:revert block:123"),
        Line::from(""),
        Line::from(format!("Active section: {}", app.active_section.title())),
    ];

    let paragraph = Paragraph::new(Text::from(lines))
        .block(Block::default().title("Help").borders(Borders::ALL))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, popup_area);
}

fn draw_settings_popup(f: &mut Frame, area: Rect, app: &App) {
    let popup_area = centered_rect(76, 70, area);
    f.render_widget(Clear, popup_area);

    let config_path = config::config_path()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "(unknown)".to_string());
    let label_db_path = config::labels_db_path()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "(unknown)".to_string());
    let abi_summary = app
        .abi_registry
        .as_ref()
        .map(|registry| {
            format!(
                "{} selectors ({} files, {} errs, {}ms)",
                registry.len(),
                registry.scanned_files,
                registry.errors.len(),
                registry.scan_ms
            )
        })
        .unwrap_or_else(|| "(not loaded)".to_string());

    let endpoint_position = if app.rpc_endpoints.is_empty() {
        "--".to_string()
    } else {
        format!("{}/{}", app.rpc_endpoint_index + 1, app.rpc_endpoints.len())
    };

    let mut lines = vec![
        Line::from(Span::styled(
            "Settings",
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(format!("Config:  {}", config_path)),
        Line::from(format!(
            "Labels:  {}  ({})",
            app.labels.len(),
            label_db_path
        )),
        Line::from(format!("RPC:     {}  ({})", app.rpc_endpoint, endpoint_position)),
        Line::from(format!("Node:    {}", app.node_kind)),
        Line::from(format!("ABI:     {}", abi_summary)),
        Line::from(format!("Tokens:  {}", app.tokens.len())),
        Line::from(""),
        Line::from("Keys: r reload config, a reload ABI, [/] switch RPC, Esc close"),
        Line::from(""),
    ];

    if !app.rpc_endpoints.is_empty() {
        lines.push(Line::from(Span::styled(
            "RPC endpoints",
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        )));
        for (idx, endpoint) in app.rpc_endpoints.iter().take(6).enumerate() {
            let marker = if idx == app.rpc_endpoint_index { ">" } else { " " };
            lines.push(Line::from(format!("{} {}", marker, endpoint.label)));
        }
        if app.rpc_endpoints.len() > 6 {
            lines.push(Line::from(format!(
                "… ({} endpoints)",
                app.rpc_endpoints.len()
            )));
        }
        lines.push(Line::from(""));
    }

    if !app.abi_scan_roots.is_empty() {
        lines.push(Line::from(Span::styled(
            "ABI scan roots",
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        )));
        for root in app.abi_scan_roots.iter().take(3) {
            lines.push(Line::from(format!("- {}", root.display())));
        }
        if app.abi_scan_roots.len() > 3 {
            lines.push(Line::from(format!(
                "… (+{} more)",
                app.abi_scan_roots.len() - 3
            )));
        }
        lines.push(Line::from(""));
    }

    if !app.tokens.is_empty() {
        lines.push(Line::from(Span::styled(
            "Configured tokens",
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        )));
        for token in app.tokens.iter().take(10) {
            let decimals = token
                .decimals
                .map(|d| d.to_string())
                .unwrap_or_else(|| "?".to_string());
            lines.push(Line::from(format!(
                "- {} (decimals {})  {}",
                token.display_symbol(),
                decimals,
                short_addr(&token.address)
            )));
        }
        if app.tokens.len() > 10 {
            lines.push(Line::from(format!("… ({} tokens)", app.tokens.len())));
        }
    } else {
        lines.push(Line::from("No tokens configured."));
        lines.push(Line::from("Example config.toml:"));
        lines.push(Line::from("  [[tokens]]"));
        lines.push(Line::from("  address = \"0x...\""));
        lines.push(Line::from("  symbol = \"USDC\""));
        lines.push(Line::from("  decimals = 6"));
    }

    let paragraph = Paragraph::new(Text::from(lines))
        .block(Block::default().title("Settings").borders(Borders::ALL))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, popup_area);
}

fn list_title(app: &App) -> String {
    let base = match app.list_kind() {
        ListKind::Blocks => "Blocks".to_string(),
        ListKind::Transactions => match app.current_view() {
            View::BlockDetail => app
                .selected_block()
                .map(|block| format!("Block #{} · Transactions", block.number))
                .unwrap_or_else(|| "Block · Transactions".to_string()),
            View::AddressDetail => app
                .selected_address()
                .map(|addr| format!("Address {} · Transactions", short_addr(&addr.address)))
                .unwrap_or_else(|| "Address · Transactions".to_string()),
            View::ContractDetail => app
                .selected_contract()
                .map(|contract| {
                    format!("Contract {} · Transactions", short_addr(&contract.address))
                })
                .unwrap_or_else(|| "Contract · Transactions".to_string()),
            _ => "Transactions".to_string(),
        },
        ListKind::Addresses => "Addresses".to_string(),
        ListKind::Contracts => "Contracts".to_string(),
        ListKind::Trace => "Trace Stack".to_string(),
    };
    if let Some(filter) = app.active_filter.as_ref() {
        format!("{base}  [filter: {}]", filter.raw)
    } else {
        base
    }
}

fn action_hints(app: &App) -> Line<'static> {
    let mut spans = vec![
        Span::styled("[ ]", Style::default().fg(Color::LightCyan)),
        Span::raw(" Section  "),
        Span::styled("Tab", Style::default().fg(Color::LightCyan)),
        Span::raw(" Focus  "),
        Span::styled("Enter", Style::default().fg(Color::LightCyan)),
        Span::raw(" Open  "),
        Span::styled("Space", Style::default().fg(Color::LightCyan)),
        Span::raw(" Pause  "),
        Span::styled("/", Style::default().fg(Color::LightCyan)),
        Span::raw(" Search/Filter  "),
        Span::styled("s", Style::default().fg(Color::LightCyan)),
        Span::raw(" Settings  "),
        Span::styled("?", Style::default().fg(Color::LightCyan)),
        Span::raw(" Help  "),
    ];

    if app.list_kind() == ListKind::Blocks {
        spans.extend([
            Span::styled("f", Style::default().fg(Color::LightCyan)),
            Span::raw(" Pin  "),
        ]);
    }
    if matches!(app.list_kind(), ListKind::Addresses | ListKind::Contracts)
        || matches!(
            app.current_view(),
            View::AddressDetail | View::ContractDetail
        )
    {
        spans.extend([
            Span::styled("w", Style::default().fg(Color::LightCyan)),
            Span::raw(" Watch  "),
            Span::styled("n", Style::default().fg(Color::LightCyan)),
            Span::raw(" Label  "),
            Span::styled("p", Style::default().fg(Color::LightCyan)),
            Span::raw(" Poke  "),
        ]);
    }
    if matches!(app.list_kind(), ListKind::Contracts)
        || matches!(app.current_view(), View::ContractDetail)
    {
        spans.extend([
            Span::styled("o", Style::default().fg(Color::LightCyan)),
            Span::raw(" Slot  "),
        ]);
    }
    if app.can_enter_trace() {
        spans.extend([
            Span::styled("t", Style::default().fg(Color::LightCyan)),
            Span::raw(" Trace  "),
        ]);
    }
    if app.current_view() == View::Trace {
        spans.extend([
            Span::styled("e", Style::default().fg(Color::LightCyan)),
            Span::raw(" Expand  "),
        ]);
    }

    spans.extend([
        Span::styled("q", Style::default().fg(Color::LightCyan)),
        Span::raw(" Quit"),
    ]);

    Line::from(spans)
}

fn block_items(app: &App) -> Vec<ListItem> {
    app.filtered_block_indices()
        .iter()
        .filter_map(|idx| app.blocks.get(*idx))
        .map(|block| {
            let pinned = if app.pinned_blocks.contains(&block.number) {
                Span::styled("* ", Style::default().fg(Color::LightYellow))
            } else {
                Span::raw("  ")
            };
            // Format gas_used with K/M suffix for readability
            let gas_str = if block.gas_used >= 1_000_000 {
                format!("{:.1}M", block.gas_used as f64 / 1_000_000.0)
            } else if block.gas_used >= 1_000 {
                format!("{:.0}K", block.gas_used as f64 / 1_000.0)
            } else {
                format!("{}", block.gas_used)
            };
            let line = Line::from(vec![
                pinned,
                Span::styled(format!("{:>7}", block.number), Style::default().fg(Color::White)),
                Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{:>3}", block.tx_count), Style::default().fg(Color::LightCyan)),
                Span::styled(" txs │ ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{:>6}", gas_str), Style::default().fg(Color::LightGreen)),
                Span::styled(" gas", Style::default().fg(Color::DarkGray)),
            ]);
            ListItem::new(line)
        })
        .collect()
}

fn tx_items(app: &App) -> Vec<ListItem> {
    app.filtered_tx_indices()
        .iter()
        .filter_map(|idx| app.txs.get(*idx))
        .map(|tx| {
            let watched =
                app.watched_addresses.contains(&tx.from) || app.watched_addresses.contains(&tx.to);
            let watched_marker = if watched {
                Span::styled("*", Style::default().fg(Color::LightYellow))
            } else {
                Span::raw(" ")
            };
            let status = match tx.status {
                TxStatus::Success => Span::styled("ok", Style::default().fg(Color::LightGreen)),
                TxStatus::Revert => Span::styled("rv", Style::default().fg(Color::LightRed)),
                TxStatus::Unknown => Span::styled("??", Style::default().fg(Color::DarkGray)),
            };
            let hash = short_hash(&tx.hash, 12);
            let from = short_addr(&tx.from);
            let to = short_addr(&tx.to);
            let line = Line::from(vec![
                watched_marker,
                Span::raw(" "),
                Span::raw(format!("{}  ", hash)),
                Span::styled(
                    format!("{}", tx.method),
                    Style::default().fg(Color::LightCyan),
                ),
                Span::raw(format!("  {} -> {}  ", from, to)),
                Span::raw(format!("{:.3} eth  ", tx.value)),
                status,
            ]);
            ListItem::new(line)
        })
        .collect()
}

fn address_items(app: &App) -> Vec<ListItem> {
    app.filtered_address_indices()
        .iter()
        .filter_map(|idx| app.addresses.get(*idx))
        .map(|addr| {
            let watched = if app.watched_addresses.contains(&addr.address) {
                "*"
            } else {
                " "
            };
            let label = addr
                .label
                .as_ref()
                .map(|l| l.as_str())
                .unwrap_or("(no-tag)");
            let kind = match addr.kind {
                AddressKind::Eoa => "EOA",
                AddressKind::Contract => "CON",
            };
            // Show "--" for unfetched balance
            let (balance_str, delta_str) = if addr.balance == 0.0 && addr.delta == 0.0 {
                ("--".to_string(), "--".to_string())
            } else {
                let delta = if addr.delta >= 0.0 { "+" } else { "" };
                (format!("{:.2} eth", addr.balance), format!("{delta}{:.2}", addr.delta))
            };
            let line = format!(
                "{watched} {}  {}  {}  {}",
                short_addr(&addr.address),
                kind,
                balance_str,
                delta_str
            );
            ListItem::new(Line::from(vec![
                Span::raw(line),
                Span::styled(format!("  {}", label), Style::default().fg(Color::DarkGray)),
            ]))
        })
        .collect()
}

fn contract_items(app: &App) -> Vec<ListItem> {
    app.filtered_contract_indices()
        .iter()
        .filter_map(|idx| app.contracts.get(*idx))
        .map(|contract| {
            let watched = if app.watched_addresses.contains(&contract.address) {
                "*"
            } else {
                " "
            };
            let label = contract
                .label
                .as_ref()
                .map(|l| l.as_str())
                .unwrap_or("(contract)");
            // Show "--" for unfetched balance
            let (balance_str, delta_str) = if contract.balance == 0.0 && contract.delta == 0.0 {
                ("--".to_string(), "--".to_string())
            } else {
                (format!("{:.2} eth", contract.balance), format!("{:+.2}", contract.delta))
            };
            let line = format!(
                "{watched} {}  {}  {}  tx {:>3}",
                short_addr(&contract.address),
                balance_str,
                delta_str,
                contract.tx_count
            );
            ListItem::new(Line::from(vec![
                Span::raw(line),
                Span::styled(format!("  {}", label), Style::default().fg(Color::DarkGray)),
            ]))
        })
        .collect()
}

fn trace_items(app: &App) -> Vec<ListItem> {
    app.trace_visible_indices()
        .iter()
        .filter_map(|idx| app.traces.get(*idx).map(|frame| (*idx, frame)))
        .map(|(idx, frame)| {
            let indent = "  ".repeat(frame.depth);
            let has_children = app
                .traces
                .get(idx + 1)
                .map(|next| next.depth > frame.depth)
                .unwrap_or(false);
            let folded = if !has_children {
                "  "
            } else if frame.collapsed {
                "+ "
            } else {
                "- "
            };
            let status = match frame.status {
                CallStatus::Ok => Span::styled("ok", Style::default().fg(Color::LightGreen)),
                CallStatus::Revert => Span::styled("rv", Style::default().fg(Color::LightRed)),
            };
            let label = frame
                .method
                .as_deref()
                .or(frame.selector.as_deref())
                .unwrap_or("");
            let line = Line::from(vec![
                Span::raw(format!("{}{}{} ", indent, folded, frame.call)),
                Span::styled(label.to_string(), Style::default().fg(Color::LightCyan)),
                Span::raw("  "),
                Span::raw(format!(
                    "{} -> {}  ",
                    short_addr(&frame.from),
                    short_addr(&frame.to)
                )),
                Span::raw(format!("{:.3} eth  ", frame.value)),
                status,
            ]);
            ListItem::new(line)
        })
        .collect()
}

fn overview_lines(app: &App) -> Vec<Line<'static>> {
    let latest_block = app
        .blocks
        .last()
        .map(|b| b.number.to_string())
        .unwrap_or_else(|| "--".to_string());
    let latest_tx = app
        .txs
        .last()
        .map(|tx| short_hash(&tx.hash, 12))
        .unwrap_or_else(|| "--".to_string());
    vec![
        Line::from(vec![
            Span::styled("Latest block: ", Style::default().fg(Color::LightCyan)),
            Span::raw(latest_block),
        ]),
        Line::from(vec![
            Span::styled("Latest tx: ", Style::default().fg(Color::LightCyan)),
            Span::raw(latest_tx),
        ]),
        Line::from(vec![
            Span::styled("Peers: ", Style::default().fg(Color::LightCyan)),
            Span::raw(app.peer_count.to_string()),
        ]),
        Line::from(vec![
            Span::styled("Sync: ", Style::default().fg(Color::LightCyan)),
            Span::raw(format!("{:.0}%", app.sync_progress * 100.0)),
        ]),
        Line::from(""),
        Line::from("Use [ ] or 1-5 to switch sections."),
        Line::from("Enter drills / opens trace."),
        Line::from("Type / to search/filter (clear/reset to reset)."),
    ]
}

fn block_summary_lines(app: &App) -> Vec<Line<'static>> {
    if let Some(block) = app.selected_block() {
        let pinned = if app.pinned_blocks.contains(&block.number) {
            "yes"
        } else {
            "no"
        };
        return vec![
            Line::from(vec![
                Span::styled("Block #", Style::default().fg(Color::LightCyan)),
                Span::raw(block.number.to_string()),
            ]),
            Line::from(format!("Tx count: {}", block.tx_count)),
            Line::from(format!("Gas used: {}", block.gas_used)),
            Line::from(format!("Base fee: {} gwei", block.base_fee)),
            Line::from(format!("Miner: {}", &block.miner)),
            Line::from(format!("Pinned: {}", pinned)),
        ];
    }
    Vec::new()
}

fn block_inspector_lines(app: &App) -> Vec<Line<'static>> {
    let mut lines = block_summary_lines(app);
    if !lines.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from("Enter: drill into block tx list"));
    }
    lines
}

fn tx_inspector_lines(app: &App) -> Vec<Line<'static>> {
    if let Some(tx) = app.selected_tx() {
        let status = match tx.status {
            TxStatus::Success => "success",
            TxStatus::Revert => "revert",
            TxStatus::Unknown => "unknown",
        };
        let signature = tx
            .signature
            .as_ref()
            .map(|sig| sig.as_str())
            .unwrap_or("(no local ABI match)");
        let input_preview = if tx.input.len() > 18 {
            format!("{}… ({} chars)", &tx.input[..18], tx.input.len())
        } else {
            tx.input.clone()
        };
        let mut lines = vec![
            Line::from(vec![
                Span::styled("Tx ", Style::default().fg(Color::LightCyan)),
                Span::raw(tx.hash.clone()),
            ]),
            Line::from(format!("From: {}", tx.from)),
            Line::from(format!("To:   {}", tx.to)),
            Line::from(format!("Method: {}", tx.method)),
            Line::from(format!("Selector: {}", tx.selector)),
            Line::from(format!("Signature: {}", signature)),
            Line::from(format!("Value: {:.3} eth", tx.value)),
            Line::from(format!("Gas used: {}", tx.gas_used)),
            Line::from(format!("Status: {}", status)),
            Line::from(format!("Block: #{}", tx.block_number)),
            Line::from(format!("Input: {}", input_preview)),
        ];

        if let Some(args) = tx.decoded_args.as_ref() {
            if !args.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "Args",
                    Style::default()
                        .fg(Color::LightCyan)
                        .add_modifier(Modifier::BOLD),
                )));
                for arg in args.iter().take(10) {
                    let name = &arg.name;
                    let kind = &arg.kind;
                    let value = truncate_str(&arg.value, 72);
                    lines.push(Line::from(format!("{name} ({kind}) = {value}")));
                }
                if args.len() > 10 {
                    lines.push(Line::from(format!("… ({} args)", args.len())));
                }
            }
        } else if let Some(err) = tx.decode_error.as_ref() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "ABI decode failed",
                Style::default().fg(Color::LightYellow),
            )));
            lines.push(Line::from(truncate_str(err, 86)));
        }

        lines.push(Line::from(""));
        lines.push(Line::from("Enter/t: open trace"));
        lines
    } else {
        Vec::new()
    }
}

fn address_summary_lines(app: &App) -> Vec<Line<'static>> {
    if let Some(addr) = app.selected_address() {
        let kind = match addr.kind {
            AddressKind::Eoa => "EOA",
            AddressKind::Contract => "Contract",
        };
        let label = addr.label.clone().unwrap_or_else(|| "(no tag)".to_string());
        let watched = if app.watched_addresses.contains(&addr.address) {
            "yes"
        } else {
            "no"
        };
        // Show "--" for unfetched balance (both balance and delta are 0)
        let balance_str = if addr.balance == 0.0 && addr.delta == 0.0 {
            "-- (press p to fetch)".to_string()
        } else {
            format!("{:.6} eth", addr.balance)
        };
        let delta_str = if addr.balance == 0.0 && addr.delta == 0.0 {
            "--".to_string()
        } else {
            format!("{:+.6}", addr.delta)
        };
        let mut lines = vec![
            Line::from(vec![
                Span::styled("Address ", Style::default().fg(Color::LightCyan)),
                Span::raw(addr.address.clone()),
            ]),
            Line::from(format!("Type: {}", kind)),
            Line::from(format!("Label: {}", label)),
            Line::from(format!("Balance: {}", balance_str)),
            Line::from(format!("Delta: {}", delta_str)),
            Line::from(format!("Watched: {}", watched)),
        ];
        lines.extend(token_balance_lines(app, &addr.address));
        return lines;
    }
    Vec::new()
}

fn address_inspector_lines(app: &App) -> Vec<Line<'static>> {
    let mut lines = address_summary_lines(app);
    if !lines.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from("Enter: drill into address tx list"));
        lines.push(Line::from("p: poke  w: watch  n: label"));
    }
    lines
}

fn contract_summary_lines(app: &App) -> Vec<Line<'static>> {
    if let Some(contract) = app.selected_contract() {
        let label = contract
            .label
            .clone()
            .unwrap_or_else(|| "(contract)".to_string());
        let watched = if app.watched_addresses.contains(&contract.address) {
            "yes"
        } else {
            "no"
        };
        // Show "--" for unfetched balance
        let balance_str = if contract.balance == 0.0 && contract.delta == 0.0 {
            "-- (press p to fetch)".to_string()
        } else {
            format!("{:.6} eth", contract.balance)
        };
        let delta_str = if contract.balance == 0.0 && contract.delta == 0.0 {
            "--".to_string()
        } else {
            format!("{:+.6}", contract.delta)
        };
        let mut lines = vec![
            Line::from(vec![
                Span::styled("Contract ", Style::default().fg(Color::LightCyan)),
                Span::raw(contract.address.clone()),
            ]),
            Line::from(format!("Label: {}", label)),
            Line::from(format!("Methods: {}", contract.methods)),
            Line::from(format!("Tx count: {}", contract.tx_count)),
            Line::from(format!("Last call: #{}", contract.last_call)),
            Line::from(format!("Balance: {}", balance_str)),
            Line::from(format!("Delta: {}", delta_str)),
            Line::from(format!("Watched: {}", watched)),
        ];
        lines.extend(token_balance_lines(app, &contract.address));
        return lines;
    }
    Vec::new()
}

fn contract_inspector_lines(app: &App) -> Vec<Line<'static>> {
    let mut lines = contract_summary_lines(app);
    if !lines.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from("Enter: drill into contract tx list"));
        lines.push(Line::from("p: poke  w: watch  n: label  o: slot"));
    }
    lines
}

fn block_browser_lines(app: &App) -> Vec<Line<'static>> {
    let mut lines = block_summary_lines(app);
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Txs in this block are shown in the list panel.",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "Esc: back  Enter/t: trace",
        Style::default().fg(Color::DarkGray),
    )));
    if app.selected_tx().is_some() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Selected tx",
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        )));
        lines.extend(tx_inspector_lines(app));
    } else {
        lines.push(Line::from(""));
        lines.push(Line::from("No tx in this block (mock)"));
    }
    lines
}

fn address_browser_lines(app: &App) -> Vec<Line<'static>> {
    let mut lines = address_summary_lines(app);
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Txs touching this address are shown in the list panel.",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "Esc: back  Enter/t: trace",
        Style::default().fg(Color::DarkGray),
    )));
    if app.selected_tx().is_some() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Selected tx",
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        )));
        lines.extend(tx_inspector_lines(app));
    }
    lines
}

fn contract_browser_lines(app: &App) -> Vec<Line<'static>> {
    let mut lines = contract_summary_lines(app);
    lines.push(Line::from(""));
    if let Some(contract) = app.selected_contract() {
        let mut storage_lines: Vec<Line<'static>> = app
            .storage_cache
            .iter()
            .filter(|((addr, _), _)| addr.eq_ignore_ascii_case(&contract.address))
            .take(6)
            .map(|((_, slot), value)| {
                let preview = if value.len() > 18 {
                    format!("{}…", &value[..18])
                } else {
                    value.clone()
                };
                Line::from(vec![
                    Span::styled(slot.clone(), Style::default().fg(Color::LightCyan)),
                    Span::raw("  "),
                    Span::raw(preview),
                ])
            })
            .collect();
        lines.push(Line::from(Span::styled(
            "Storage (cached)",
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        )));
        if storage_lines.is_empty() {
            lines.push(Line::from("  (empty)"));
        } else {
            lines.append(&mut storage_lines);
        }
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "o: query storage slot",
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(""));
    }
    lines.push(Line::from(Span::styled(
        "Txs touching this contract are shown in the list panel.",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "Esc: back  Enter/t: trace",
        Style::default().fg(Color::DarkGray),
    )));
    if app.selected_tx().is_some() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Selected tx",
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        )));
        lines.extend(tx_inspector_lines(app));
    }
    lines
}

fn tx_detail_lines(app: &App) -> Vec<Line<'static>> {
    tx_inspector_lines(app)
}

fn trace_detail_lines(app: &App) -> Vec<Line<'static>> {
    if let Some(frame) = app.selected_trace() {
        let status = match frame.status {
            CallStatus::Ok => "ok",
            CallStatus::Revert => "revert",
        };
        let collapsed = if frame.collapsed { "yes" } else { "no" };
        let signature = frame.signature.as_deref().unwrap_or("(no local ABI match)");
        let method = frame
            .method
            .as_deref()
            .or(frame.selector.as_deref())
            .unwrap_or("(unknown)");
        let input_preview = frame
            .input
            .as_ref()
            .map(|input| {
                if input.len() > 18 {
                    format!("{}… ({} chars)", &input[..18], input.len())
                } else {
                    input.clone()
                }
            })
            .unwrap_or_else(|| "--".to_string());

        let mut lines = vec![
            Line::from(vec![
                Span::styled("Call ", Style::default().fg(Color::LightCyan)),
                Span::raw(frame.call.clone()),
            ]),
            Line::from(format!("From: {}", &frame.from)),
            Line::from(format!("To:   {}", &frame.to)),
            Line::from(format!("Method: {}", method)),
            Line::from(format!("Signature: {}", signature)),
            Line::from(format!("Value: {:.3} eth", frame.value)),
            Line::from(format!("Gas used: {}", frame.gas_used)),
            Line::from(format!("Status: {}", status)),
            Line::from(format!("Note: {}", frame.note)),
            Line::from(format!("Input: {}", input_preview)),
            Line::from(format!("Collapsed: {}", collapsed)),
        ];

        if let Some(args) = frame.decoded_args.as_ref() {
            if !args.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "Args",
                    Style::default()
                        .fg(Color::LightCyan)
                        .add_modifier(Modifier::BOLD),
                )));
                for arg in args.iter().take(10) {
                    let name = &arg.name;
                    let kind = &arg.kind;
                    let value = truncate_str(&arg.value, 72);
                    lines.push(Line::from(format!("{name} ({kind}) = {value}")));
                }
                if args.len() > 10 {
                    lines.push(Line::from(format!("… ({} args)", args.len())));
                }
            }
        } else if let Some(err) = frame.decode_error.as_ref() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "ABI decode failed",
                Style::default().fg(Color::LightYellow),
            )));
            lines.push(Line::from(truncate_str(err, 86)));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(
            "Enter/e toggles collapse, Esc closes trace view.",
        ));
        lines
    } else {
        Vec::new()
    }
}

fn token_balance_lines(app: &App, owner: &str) -> Vec<Line<'static>> {
    if app.tokens.is_empty() {
        return Vec::new();
    }
    let owner_key = normalize_hex(owner);
    let mut lines = Vec::new();
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Tokens",
        Style::default()
            .fg(Color::LightCyan)
            .add_modifier(Modifier::BOLD),
    )));
    for token in app.tokens.iter().take(8) {
        let key = (owner_key.clone(), token.normalized_address());
        let balance = app
            .token_balances
            .get(&key)
            .cloned()
            .unwrap_or_else(|| "--".to_string());
        lines.push(Line::from(format!(
            "{}: {}",
            token.display_symbol(),
            balance
        )));
    }
    if app.tokens.len() > 8 {
        lines.push(Line::from(format!("… ({} tokens)", app.tokens.len())));
    }
    lines
}

fn short_addr(value: &str) -> String {
    if value.len() <= 10 {
        return value.to_string();
    }
    let start: String = value.chars().take(6).collect();
    let end: String = value
        .chars()
        .rev()
        .take(4)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    format!("{}..{}", start, end)
}

fn short_hash(value: &str, len: usize) -> String {
    if value.len() <= len {
        return value.to_string();
    }
    value.chars().take(len).collect()
}

fn normalize_hex(value: &str) -> String {
    let trimmed = value.trim();
    let payload = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);
    format!("0x{}", payload.to_lowercase())
}

fn truncate_str(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    value.chars().take(max).collect::<String>() + "…"
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
