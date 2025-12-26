mod app;
mod config;
mod core;
mod domain;
mod infrastructure;
mod modules;
mod store;
mod ui;

use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Result;
use clap::Parser;
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
    KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;

use crate::app::{
    App, DataMode, Focus, InputMode, ListKind, PromptKind, RpcEndpointOption, Section, StatusLevel,
    View,
};
use crate::domain::abi::AbiRegistry;
use crate::infrastructure::ethereum::ProviderConfig;
use crate::infrastructure::runtime::{RuntimeBridge, RuntimeCommand, RuntimeEvent, TokenConfig};
use crate::store::LabelStore;

#[derive(Debug, Parser)]
#[command(
    name = "poke",
    version,
    about = "Poke: a local-first Ethereum node TUI tool"
)]
struct Args {
    /// HTTP JSON-RPC endpoint (e.g. http://localhost:8545)
    #[arg(long)]
    rpc: Option<String>,

    /// WebSocket endpoint (e.g. ws://localhost:8546)
    #[arg(long)]
    ws: Option<String>,

    /// IPC path (e.g. ~/.ethereum/geth.ipc). Unix only.
    #[arg(long)]
    ipc: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let config = config::load();
    let (rpc_endpoints, rpc_endpoint_options) = endpoints_from_args_and_config(&args, &config)?;
    let initial_endpoint_display = rpc_endpoints
        .first()
        .map(|endpoint| endpoint.display())
        .unwrap_or_else(|| "localhost:8545".to_string());

    let mut stdout = io::stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create the runtime bridge
    let runtime = RuntimeBridge::new(rpc_endpoints)?;

    // Spawn ABI scanner thread
    let abi_scan_roots = abi_scan_roots_from_config(&config);
    let (abi_scan_tx, abi_evt_rx) = spawn_abi_scanner(abi_scan_roots.clone());

    let mut app = App::new();
    app.data_mode = DataMode::Rpc;
    app.tokens = config.tokens;
    app.blocks.clear();
    app.txs.clear();
    app.traces.clear();
    app.rpc_endpoint = initial_endpoint_display;
    app.rpc_endpoints = rpc_endpoint_options;
    app.rpc_endpoint_index = 0;
    app.node_kind = "connecting".to_string();
    app.abi_reload_sender = Some(abi_scan_tx);
    app.abi_scan_roots = abi_scan_roots;
    app.set_status("Connecting…", StatusLevel::Info);

    if let Some(db_path) = config::labels_db_path() {
        if let Some(parent) = db_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        match LabelStore::open(&db_path).and_then(|store| {
            let labels = store.load_all()?;
            Ok((store, labels))
        }) {
            Ok((store, labels)) => {
                app.labels = labels;
                app.label_store = Some(store);
            }
            Err(err) => {
                app.set_status(format!("Label DB disabled: {err}"), StatusLevel::Warn);
            }
        }
    }

    let res = run_app(&mut terminal, app, runtime, abi_evt_rx);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("{err:?}");
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    runtime: RuntimeBridge,
    abi_evt_rx: std::sync::mpsc::Receiver<AbiRegistry>,
) -> Result<()> {
    let tick_rate = Duration::from_millis(200);
    let mut last_tick = Instant::now();

    loop {
        pump_background(&mut app, &runtime, &abi_evt_rx);
        app.sync_context();
        terminal.draw(|f| ui::draw(f, &mut app))?;
        if app.should_quit {
            let _ = runtime.send(RuntimeCommand::Shutdown);
            return Ok(());
        }

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) => handle_key(&mut app, key, &runtime),
                Event::Mouse(mouse) => handle_mouse(&mut app, mouse),
                Event::Resize(_, _) => {}
                _ => {}
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = Instant::now();
        }

        pump_background(&mut app, &runtime, &abi_evt_rx);
    }
}

fn pump_background(
    app: &mut App,
    runtime: &RuntimeBridge,
    abi_evt_rx: &std::sync::mpsc::Receiver<AbiRegistry>,
) {
    // Process runtime events
    for event in runtime.poll_events() {
        match event {
            RuntimeEvent::Connected {
                endpoint,
                node_kind,
                accounts,
            } => app.apply_rpc_connected(endpoint, node_kind, accounts),
            RuntimeEvent::Status {
                rtt_ms,
                peer_count,
                sync_progress,
            } => app.apply_rpc_status(rtt_ms, peer_count, sync_progress),
            RuntimeEvent::NewBlock { block, txs } => {
                if !app.paused {
                    // Convert infrastructure types to app types
                    let block = app::BlockInfo {
                        number: block.number,
                        tx_count: block.tx_count,
                        gas_used: block.gas_used,
                        base_fee: block.base_fee,
                        miner: block.miner,
                    };
                    let txs: Vec<app::TxInfo> = txs
                        .into_iter()
                        .map(|tx| app::TxInfo {
                            hash: tx.hash,
                            from: tx.from,
                            to: tx.to,
                            value: tx.value,
                            gas_used: tx.gas_used,
                            status: match tx.status {
                                infrastructure::TxStatus::Success => app::TxStatus::Success,
                                infrastructure::TxStatus::Revert => app::TxStatus::Revert,
                                infrastructure::TxStatus::Unknown => app::TxStatus::Unknown,
                            },
                            input: tx.input,
                            selector: tx.selector,
                            method: tx.method,
                            signature: tx.signature,
                            decoded_args: tx.decoded_args.map(|args| {
                                args.into_iter()
                                    .map(|a| app::DecodedArg {
                                        name: a.name,
                                        kind: a.kind,
                                        value: a.value,
                                    })
                                    .collect()
                            }),
                            decode_error: tx.decode_error,
                            block_number: tx.block_number,
                        })
                        .collect();
                    app.ingest_block(block, txs);
                }
            }
            RuntimeEvent::TraceReady { frames, .. } => {
                let frames: Vec<app::TraceFrame> = frames
                    .into_iter()
                    .map(|f| app::TraceFrame {
                        depth: f.depth,
                        call: f.call,
                        from: f.from,
                        to: f.to,
                        value: f.value,
                        gas_used: f.gas_used,
                        status: match f.status {
                            infrastructure::CallStatus::Ok => app::CallStatus::Ok,
                            infrastructure::CallStatus::Revert => app::CallStatus::Revert,
                        },
                        note: f.note,
                        collapsed: f.collapsed,
                        input: f.input,
                        selector: f.selector,
                        method: f.method,
                        signature: f.signature,
                        decoded_args: f.decoded_args.map(|args| {
                            args.into_iter()
                                .map(|a| app::DecodedArg {
                                    name: a.name,
                                    kind: a.kind,
                                    value: a.value,
                                })
                                .collect()
                        }),
                        decode_error: f.decode_error,
                    })
                    .collect();
                app.ingest_trace(frames);
            }
            RuntimeEvent::BalanceReady { address, balance } => app.apply_balance(address, balance),
            RuntimeEvent::TokenBalancesReady { address, balances } => {
                let balances: Vec<app::TokenBalance> = balances
                    .into_iter()
                    .map(|b| app::TokenBalance {
                        token: b.token,
                        symbol: b.symbol,
                        decimals: b.decimals,
                        balance: b.balance,
                    })
                    .collect();
                app.apply_token_balances(address, balances);
            }
            RuntimeEvent::StorageReady {
                address,
                slot,
                value,
            } => app.apply_storage_value(address, slot, value),
            RuntimeEvent::AbiRegistryReady { registry } => app.apply_abi_registry(registry),
            RuntimeEvent::SignatureResolved {
                selector,
                name,
                signature,
            } => {
                // Debug: Log event receipt
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("/tmp/poke-abi-debug.log")
                {
                    use std::io::Write;
                    let _ = writeln!(f, "[MAIN] Received SignatureResolved for {}: {} ({})", selector, name, signature);
                }
                app.apply_signature(selector, name, signature);
            }
            RuntimeEvent::AbiResolved { .. } => {
                // TODO: integrate full ABI into registry for better decoding
            }
            RuntimeEvent::Error { message } => app.apply_rpc_error(message),
        }
    }

    // Process pending commands
    if let Some(index) = app.take_endpoint_switch_request() {
        let _ = runtime.send(RuntimeCommand::SwitchEndpoint { index });
    }
    if let Some(tx_hash) = app.take_trace_request() {
        let _ = runtime.send(RuntimeCommand::FetchTrace { tx_hash });
    }
    if app.take_refresh_request() {
        let _ = runtime.send(RuntimeCommand::Refresh);
    }
    if let Some(address) = app.take_balance_request() {
        let _ = runtime.send(RuntimeCommand::FetchBalance {
            address: address.clone(),
        });
        if !app.tokens.is_empty() {
            let tokens: Vec<TokenConfig> = app
                .tokens
                .iter()
                .map(|t| TokenConfig {
                    address: t.address.clone(),
                    symbol: t.symbol.clone(),
                    decimals: t.decimals,
                })
                .collect();
            let _ = runtime.send(RuntimeCommand::FetchTokenBalances { address, tokens });
        }
    }
    if let Some(request) = app.take_storage_request() {
        let _ = runtime.send(RuntimeCommand::FetchStorage {
            address: request.address,
            slot: request.slot,
        });
    }

    // Process ABI registry updates
    while let Ok(registry) = abi_evt_rx.try_recv() {
        app.apply_abi_registry(registry);
    }
}

fn endpoints_from_args_and_config(
    args: &Args,
    config: &config::Config,
) -> Result<(Vec<ProviderConfig>, Vec<RpcEndpointOption>)> {
    use std::collections::BTreeSet;

    fn push_endpoint(
        endpoints: &mut Vec<ProviderConfig>,
        options: &mut Vec<RpcEndpointOption>,
        seen: &mut BTreeSet<String>,
        endpoint: ProviderConfig,
        name: Option<String>,
    ) {
        let display = endpoint.display();
        let key = display.to_lowercase();
        if !seen.insert(key) {
            return;
        }
        let label = name
            .clone()
            .filter(|value| !value.trim().is_empty())
            .map(|name| format!("{name} ({display})"))
            .unwrap_or_else(|| display.clone());
        options.push(RpcEndpointOption { label, display });
        endpoints.push(endpoint);
    }

    let mut endpoints = Vec::new();
    let mut options = Vec::new();
    let mut seen = BTreeSet::<String>::new();

    // CLI arguments take precedence
    if let Some(ipc) = args.ipc.clone() {
        #[cfg(unix)]
        {
            push_endpoint(
                &mut endpoints,
                &mut options,
                &mut seen,
                ProviderConfig::Ipc(ipc),
                Some("cli".to_string()),
            );
        }
        #[cfg(not(unix))]
        {
            return Err(anyhow::anyhow!("IPC is not supported on this platform"));
        }
    } else if let Some(ws) = args.ws.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        push_endpoint(
            &mut endpoints,
            &mut options,
            &mut seen,
            ProviderConfig::WebSocket(ws.to_string()),
            Some("cli".to_string()),
        );
    } else if let Some(rpc) = args.rpc.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        push_endpoint(
            &mut endpoints,
            &mut options,
            &mut seen,
            ProviderConfig::Http(normalize_http_endpoint(rpc)),
            Some("cli".to_string()),
        );
    }

    // Config file endpoints
    for (idx, entry) in config.endpoints.iter().enumerate() {
        let name = entry.name.clone().filter(|value| !value.trim().is_empty());
        if let Some(rpc) = entry.rpc.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            push_endpoint(
                &mut endpoints,
                &mut options,
                &mut seen,
                ProviderConfig::Http(normalize_http_endpoint(rpc)),
                name,
            );
            continue;
        }
        if let Some(ipc) = entry.ipc.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            #[cfg(unix)]
            {
                let Some(ipc_path) = expand_path(ipc) else {
                    continue;
                };
                push_endpoint(
                    &mut endpoints,
                    &mut options,
                    &mut seen,
                    ProviderConfig::Ipc(ipc_path),
                    name.or_else(|| Some(format!("ipc-{idx}"))),
                );
            }
            #[cfg(not(unix))]
            {
                let _ = idx;
            }
        }
    }

    // Default fallback
    if endpoints.is_empty() {
        push_endpoint(
            &mut endpoints,
            &mut options,
            &mut seen,
            ProviderConfig::Http(normalize_http_endpoint("localhost:8545")),
            Some("local".to_string()),
        );
    }

    Ok((endpoints, options))
}

fn normalize_http_endpoint(endpoint: &str) -> String {
    let trimmed = endpoint.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else {
        format!("http://{}", trimmed)
    }
}

fn abi_scan_roots_from_config(config: &config::Config) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for raw in &config.abi_paths {
        if let Some(path) = expand_path(raw) {
            roots.push(path);
        }
    }
    if roots.is_empty() {
        roots.push(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    }
    roots
}

fn expand_path(path: &str) -> Option<PathBuf> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(rest) = trimmed.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            return Some(home.join(rest));
        }
    }

    let mut buf = PathBuf::from(trimmed);
    if buf.is_relative() {
        if let Ok(cwd) = std::env::current_dir() {
            buf = cwd.join(buf);
        }
    }
    Some(buf)
}

/// Request to scan ABI files
#[derive(Debug, Clone)]
pub struct AbiScanRequest {
    pub roots: Vec<PathBuf>,
}

fn spawn_abi_scanner(
    initial_roots: Vec<PathBuf>,
) -> (Sender<AbiScanRequest>, std::sync::mpsc::Receiver<AbiRegistry>) {
    let (req_tx, req_rx) = std::sync::mpsc::channel::<AbiScanRequest>();
    let (evt_tx, evt_rx) = std::sync::mpsc::channel::<AbiRegistry>();

    thread::spawn(move || {
        while let Ok(mut request) = req_rx.recv() {
            while let Ok(next) = req_rx.try_recv() {
                request = next;
            }
            let registry = infrastructure::AbiScanner::scan_roots(&request.roots);
            let _ = evt_tx.send(registry);
        }
    });

    let _ = req_tx.send(AbiScanRequest {
        roots: initial_roots,
    });

    (req_tx, evt_rx)
}

fn reload_config(app: &mut App) {
    let config = config::load();
    let abi_scan_roots = abi_scan_roots_from_config(&config);
    app.tokens = config.tokens;
    app.token_balances.clear();
    app.abi_scan_roots = abi_scan_roots;
    app.set_status(
        format!(
            "Reloaded config: {} tokens, {} ABI roots",
            app.tokens.len(),
            app.abi_scan_roots.len()
        ),
        StatusLevel::Info,
    );
}

fn handle_key(app: &mut App, key: KeyEvent, _runtime: &RuntimeBridge) {
    if key.kind != KeyEventKind::Press {
        return;
    }

    if app.help_open {
        if matches!(key.code, KeyCode::Char('?') | KeyCode::Esc) {
            app.help_open = false;
        }
        return;
    }

    if app.settings_open {
        match key.code {
            KeyCode::Esc | KeyCode::Char('s') => app.settings_open = false,
            KeyCode::Char('r') => reload_config(app),
            KeyCode::Char('a') => app.request_abi_reload(),
            KeyCode::Char('[') => app.cycle_rpc_endpoint(false),
            KeyCode::Char(']') => app.cycle_rpc_endpoint(true),
            _ => {}
        }
        return;
    }

    match app.input_mode {
        InputMode::Normal => handle_normal_mode(app, key),
        InputMode::Command => handle_command_mode(app, key),
        InputMode::Prompt(kind) => handle_prompt_mode(app, key, kind),
    }
}

fn handle_mouse(app: &mut App, mouse: MouseEvent) {
    if app.help_open
        || app.settings_open
        || matches!(app.input_mode, InputMode::Command | InputMode::Prompt(_))
    {
        return;
    }
    let Some(size) = terminal_rect() else {
        return;
    };
    let areas = ui::layout::areas(size);
    let col = mouse.column;
    let row = mouse.row;

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => handle_click(app, areas, col, row),
        MouseEventKind::ScrollUp => handle_scroll(app, areas, col, row, true),
        MouseEventKind::ScrollDown => handle_scroll(app, areas, col, row, false),
        _ => {}
    }
}

fn handle_normal_mode(app: &mut App, key: KeyEvent) {
    if key.code != KeyCode::Char('g') {
        app.clear_chord();
    }

    match (key.code, key.modifiers) {
        (KeyCode::Char('q'), _) => app.should_quit = true,
        (KeyCode::Char('?'), _) => app.help_open = true,
        (KeyCode::Char('/'), _) => app.enter_command(),
        (KeyCode::Char('s'), _) => app.settings_open = true,
        (KeyCode::Char('r'), _) => app.refresh(),
        (KeyCode::Char('h'), _) => focus_left(app),
        (KeyCode::Char('l'), _) => focus_right(app),
        (KeyCode::Char('g'), _) => {
            if app.consume_chord('g') {
                app.go_to_top();
            } else {
                app.set_chord('g');
            }
        }
        (KeyCode::Char('G'), _) => app.go_to_bottom(),
        (KeyCode::Char('u'), mods) if mods.contains(KeyModifiers::CONTROL) => {
            app.page_up(page_amount(true));
        }
        (KeyCode::Char('d'), mods) if mods.contains(KeyModifiers::CONTROL) => {
            app.page_down(page_amount(true));
        }
        (KeyCode::Char('b'), mods) if mods.contains(KeyModifiers::CONTROL) => {
            app.page_up(page_amount(false));
        }
        (KeyCode::Char('f'), mods) if mods.contains(KeyModifiers::CONTROL) => {
            app.page_down(page_amount(false));
        }
        (KeyCode::Char('f'), _) => {
            // 'f' key behavior depends on current view
            if app.current_view() == View::Dashboard {
                // From Dashboard: enter Explorer (Blocks view)
                app.push_view(View::Overview);
                app.set_section(Section::Blocks);
                app.focus = Focus::List;
            } else if app.list_kind() == ListKind::Blocks {
                // In Explorer Blocks view: toggle pin
                app.toggle_pin();
            } else {
                app.set_status("Pin is available in Blocks list", StatusLevel::Warn);
            }
        }
        (KeyCode::Char('w'), _) => {
            if let Some(address) = context_address(app) {
                if app.watched_addresses.contains(&address) {
                    app.watched_addresses.remove(&address);
                    app.set_status("Removed watch", StatusLevel::Info);
                } else {
                    app.watched_addresses.insert(address);
                    app.set_status("Watching address", StatusLevel::Info);
                }
            } else {
                app.set_status(
                    "Watch is available for addresses/contracts",
                    StatusLevel::Warn,
                );
            }
        }
        (KeyCode::Char('n'), _) => {
            if let Some(address) = context_address(app) {
                app.enter_prompt(PromptKind::Label, address);
            } else {
                app.set_status(
                    "Label is available for addresses/contracts",
                    StatusLevel::Warn,
                );
            }
        }
        (KeyCode::Char('y'), _) => {
            // Copy selected item to clipboard
            handle_copy_to_clipboard(app);
        }
        (KeyCode::Char('p'), _) => {
            if let Some(address) = context_address(app) {
                app.request_balance(address);
            } else {
                app.set_status(
                    "Poke is available for addresses/contracts",
                    StatusLevel::Warn,
                );
            }
        }
        (KeyCode::Char('o'), _) => {
            if let Some(address) = context_contract(app) {
                app.enter_prompt(PromptKind::StorageSlot, address);
            } else {
                app.set_status("Storage poke is available for contracts", StatusLevel::Warn);
            }
        }
        (KeyCode::Char('t'), _) => {
            if app.can_enter_trace() {
                app.enter_trace();
            } else {
                app.set_status("Trace is available for transactions", StatusLevel::Warn);
            }
        }
        (KeyCode::Char('e'), _) => {
            if app.current_view() == View::Trace {
                app.toggle_trace_collapse();
            }
        }
        (KeyCode::Char(' '), _) => app.toggle_pause(),
        (KeyCode::Tab, _) => {
            // Tab behavior depends on current view
            if app.current_view() == View::Dashboard {
                // In Dashboard: navigate between panels
                use crate::core::Module;
                let key_event = crossterm::event::KeyEvent::new(
                    KeyCode::Tab,
                    crossterm::event::KeyModifiers::NONE,
                );
                app.dashboard.handle_key(key_event, &mut app.ctx);
            } else {
                // In Explorer: cycle focus
                cycle_focus(app);
            }
        }
        (KeyCode::Enter, _) => handle_enter(app),
        (KeyCode::Esc, _) => {
            // Esc behavior: return to Dashboard if we're in Explorer
            if app.current_view() != View::Dashboard && app.view_stack.first() == Some(&View::Dashboard) {
                // Pop back to Dashboard
                while app.view_stack.len() > 1 {
                    app.view_stack.pop();
                }
                app.focus = Focus::List;
            } else {
                // Normal pop behavior
                app.pop_view();
                app.focus = Focus::List;
            }
        }
        (KeyCode::Char('['), _) => app.cycle_section(false),
        (KeyCode::Char(']'), _) => app.cycle_section(true),
        (KeyCode::Char('1'), _) => app.set_section(Section::Overview),
        (KeyCode::Char('2'), _) => app.set_section(Section::Blocks),
        (KeyCode::Char('3'), _) => app.set_section(Section::Transactions),
        (KeyCode::Char('4'), _) => app.set_section(Section::Addresses),
        (KeyCode::Char('5'), _) => app.set_section(Section::Contracts),
        (KeyCode::Up | KeyCode::Char('k'), _) => {
            if app.current_view() == View::Dashboard {
                use crate::core::Module;
                let key_event = crossterm::event::KeyEvent::new(
                    key.code,
                    crossterm::event::KeyModifiers::NONE,
                );
                app.dashboard.handle_key(key_event, &mut app.ctx);
            } else {
                handle_nav_up(app);
            }
        }
        (KeyCode::Down | KeyCode::Char('j'), _) => {
            if app.current_view() == View::Dashboard {
                use crate::core::Module;
                let key_event = crossterm::event::KeyEvent::new(
                    key.code,
                    crossterm::event::KeyModifiers::NONE,
                );
                app.dashboard.handle_key(key_event, &mut app.ctx);
            } else {
                handle_nav_down(app);
            }
        }
        _ => {}
    }
}

fn handle_command_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => app.exit_command(),
        KeyCode::Enter => app.apply_command(),
        KeyCode::Backspace => {
            app.command.input.pop();
        }
        KeyCode::Char(ch) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                return;
            }
            app.command.input.push(ch);
        }
        _ => {}
    }
}

fn handle_prompt_mode(app: &mut App, key: KeyEvent, kind: PromptKind) {
    match key.code {
        KeyCode::Esc => app.exit_prompt(),
        KeyCode::Enter => app.apply_prompt(kind),
        KeyCode::Backspace => {
            app.command.input.pop();
        }
        KeyCode::Char(ch) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                return;
            }
            app.command.input.push(ch);
        }
        _ => {}
    }
}

fn cycle_focus(app: &mut App) {
    app.focus = match app.focus {
        Focus::Sidebar => Focus::List,
        Focus::List => Focus::Details,
        Focus::Details | Focus::Command => Focus::Sidebar,
    };
}

fn enter_detail(app: &mut App) {
    app.enter_detail();
    app.focus = Focus::Details;
}

fn handle_enter(app: &mut App) {
    // Handle Dashboard navigation
    if app.current_view() == View::Dashboard {
        use crate::core::Module;
        let key_event = crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Enter,
            crossterm::event::KeyModifiers::NONE,
        );
        let action = app.dashboard.handle_key(key_event, &mut app.ctx);
        app.apply_action(action);
        return;
    }

    if app.current_view() == View::Trace {
        app.toggle_trace_collapse();
        return;
    }
    if app.list_kind() == ListKind::Transactions {
        if app.can_enter_trace() {
            app.enter_trace();
        } else {
            app.set_status("Trace is available for transactions", StatusLevel::Warn);
        }
        return;
    }
    enter_detail(app);
}

fn context_address(app: &App) -> Option<String> {
    match app.current_view() {
        View::AddressDetail => app.selected_address().map(|addr| addr.address.clone()),
        View::ContractDetail => app
            .selected_contract()
            .map(|contract| contract.address.clone()),
        _ => match app.list_kind() {
            ListKind::Addresses => app.selected_address().map(|addr| addr.address.clone()),
            ListKind::Contracts => app
                .selected_contract()
                .map(|contract| contract.address.clone()),
            _ => None,
        },
    }
}

fn context_contract(app: &App) -> Option<String> {
    match app.current_view() {
        View::ContractDetail => app
            .selected_contract()
            .map(|contract| contract.address.clone()),
        _ => match app.list_kind() {
            ListKind::Contracts => app
                .selected_contract()
                .map(|contract| contract.address.clone()),
            _ => None,
        },
    }
}

fn focus_left(app: &mut App) {
    app.focus = match app.focus {
        Focus::Sidebar => Focus::Sidebar,
        Focus::List => Focus::Sidebar,
        Focus::Details | Focus::Command => Focus::List,
    };
}

fn focus_right(app: &mut App) {
    app.focus = match app.focus {
        Focus::Sidebar => Focus::List,
        Focus::List | Focus::Command => Focus::Details,
        Focus::Details => Focus::Details,
    };
}

fn terminal_rect() -> Option<Rect> {
    let (width, height) = crossterm::terminal::size().ok()?;
    Some(Rect {
        x: 0,
        y: 0,
        width,
        height,
    })
}

fn rect_contains(rect: Rect, col: u16, row: u16) -> bool {
    col >= rect.x
        && col < rect.x.saturating_add(rect.width)
        && row >= rect.y
        && row < rect.y.saturating_add(rect.height)
}

fn rect_inner(rect: Rect) -> Rect {
    Rect {
        x: rect.x.saturating_add(1),
        y: rect.y.saturating_add(1),
        width: rect.width.saturating_sub(2),
        height: rect.height.saturating_sub(2),
    }
}

fn list_visible_height(areas: ui::layout::UiAreas) -> usize {
    rect_inner(areas.list).height.max(1) as usize
}

fn page_amount(half: bool) -> usize {
    let Some(size) = terminal_rect() else {
        return 5;
    };
    let areas = ui::layout::areas(size);
    let height = list_visible_height(areas);
    if half {
        (height / 2).max(1)
    } else {
        height.max(1)
    }
}

fn handle_scroll(app: &mut App, areas: ui::layout::UiAreas, col: u16, row: u16, up: bool) {
    if rect_contains(areas.sidebar_sections, col, row) {
        app.focus = Focus::Sidebar;
        app.cycle_section(!up);
        return;
    }
    if rect_contains(areas.list, col, row) {
        app.focus = Focus::List;
        if up {
            app.move_selection_up();
        } else {
            app.move_selection_down();
        }
        return;
    }
    if up {
        handle_nav_up(app);
    } else {
        handle_nav_down(app);
    }
}

fn handle_click(app: &mut App, areas: ui::layout::UiAreas, col: u16, row: u16) {
    if rect_contains(areas.sidebar_sections, col, row) {
        let inner = rect_inner(areas.sidebar_sections);
        if !rect_contains(inner, col, row) {
            return;
        }
        let idx = (row - inner.y) as usize;
        if let Some(section) = Section::ALL.get(idx).copied() {
            app.set_section(section);
            app.focus = Focus::Sidebar;
        }
        return;
    }

    if rect_contains(areas.sidebar_watch, col, row) {
        let inner = rect_inner(areas.sidebar_watch);
        if !rect_contains(inner, col, row) {
            return;
        }
        let idx = (row - inner.y) as usize;
        if let Some(addr) = app.watched_addresses.iter().nth(idx) {
            let addr = addr.clone();
            let _ = app.jump_to_address(&addr);
            app.focus = Focus::List;
        }
        return;
    }

    if rect_contains(areas.list, col, row) {
        let inner = rect_inner(areas.list);
        if !rect_contains(inner, col, row) {
            return;
        }
        let row_idx = (row - inner.y) as usize;
        let list_len = app.list_len();
        if row_idx >= inner.height as usize || list_len == 0 {
            return;
        }
        let visible_height = inner.height.max(1) as usize;
        let selected = app.current_selection();
        let offset = if selected >= visible_height {
            selected.saturating_sub(visible_height.saturating_sub(1))
        } else {
            0
        };
        let clicked = offset + row_idx;
        if clicked < list_len {
            app.set_list_selection(clicked);
            app.focus = Focus::List;
        }
        return;
    }

    if rect_contains(areas.details, col, row) {
        app.focus = Focus::Details;
    }
}

fn handle_nav_up(app: &mut App) {
    match app.focus {
        Focus::Sidebar => app.cycle_section(false),
        Focus::List | Focus::Details => app.move_selection_up(),
        Focus::Command => {}
    }
}

fn handle_nav_down(app: &mut App) {
    match app.focus {
        Focus::Sidebar => app.cycle_section(true),
        Focus::List | Focus::Details => app.move_selection_down(),
        Focus::Command => {}
    }
}

fn handle_copy_to_clipboard(app: &mut App) {
    use arboard::Clipboard;
    use crate::core::Selected;

    let text_to_copy = match app.current_view() {
        View::Dashboard => {
            // In Dashboard, copy selected activity item
            match &app.ctx.selected {
                Selected::Block(num) => Some(format!("{}", num)),
                Selected::Transaction(hash) => Some(hash.clone()),
                Selected::Address(addr) => Some(addr.clone()),
                Selected::None => None,
                _ => None,
            }
        }
        View::BlockDetail => {
            // Copy block number
            app.selected_block().map(|block| format!("{}", block.number))
        }
        View::TxDetail => {
            // Copy transaction hash
            app.selected_tx().map(|tx| tx.hash.clone())
        }
        View::AddressDetail => {
            // Copy address
            app.selected_address().map(|addr| addr.address.clone())
        }
        View::ContractDetail => {
            // Copy contract address
            app.selected_contract().map(|contract| contract.address.clone())
        }
        _ => {
            // For other views, try to get context address
            context_address(app)
        }
    };

    if let Some(text) = text_to_copy {
        match Clipboard::new() {
            Ok(mut clipboard) => {
                if clipboard.set_text(&text).is_ok() {
                    app.ctx.set_clipboard(text.clone());
                    app.set_status(
                        format!("Copied: {}", if text.len() > 20 {
                            format!("{}...", &text[..20])
                        } else {
                            text
                        }),
                        StatusLevel::Info,
                    );
                } else {
                    app.set_status("Failed to copy to clipboard", StatusLevel::Error);
                }
            }
            Err(_) => {
                app.set_status("Clipboard not available", StatusLevel::Error);
            }
        }
    } else {
        app.set_status("Nothing to copy", StatusLevel::Warn);
    }
}
