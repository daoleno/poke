use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use crate::config::TokenSpec;
use crate::core::Context;
use crate::domain::abi::AbiRegistry;
use crate::AbiScanRequest;
use crate::store::LabelStore;

/// Main tabs in the application
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Home,
    Explorer,
    Toolkit,
    Ops,
    Anvil,
}

impl Tab {
    pub const ALL: [Tab; 5] = [Tab::Home, Tab::Explorer, Tab::Toolkit, Tab::Ops, Tab::Anvil];

    pub fn title(&self) -> &'static str {
        match self {
            Tab::Home => "Home",
            Tab::Explorer => "Explorer",
            Tab::Toolkit => "Toolkit",
            Tab::Ops => "Ops",
            Tab::Anvil => "Anvil",
        }
    }

    pub fn shortcut(&self) -> char {
        match self {
            Tab::Home => '1',
            Tab::Explorer => '2',
            Tab::Toolkit => '3',
            Tab::Ops => '4',
            Tab::Anvil => '5',
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Dashboard,
    Overview,
    BlockDetail,
    TxDetail,
    AddressDetail,
    ContractDetail,
    Trace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Section {
    Overview,
    Blocks,
    Transactions,
    Addresses,
    Contracts,
}

impl Section {
    pub const ALL: [Section; 5] = [
        Section::Overview,
        Section::Blocks,
        Section::Transactions,
        Section::Addresses,
        Section::Contracts,
    ];

    pub fn title(&self) -> &'static str {
        match self {
            Section::Overview => "Overview",
            Section::Blocks => "Blocks",
            Section::Transactions => "Transactions",
            Section::Addresses => "Addresses",
            Section::Contracts => "Contracts",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Sidebar,
    List,
    Details,
    Command,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Command,
    Prompt(PromptKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataMode {
    Mock,
    Rpc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptKind {
    StorageSlot,
    Label,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusLevel {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterKey {
    Block,
    Tx,
    Hash,
    From,
    To,
    Method,
    Status,
    Addr,
    Label,
    Miner,
}

#[derive(Debug, Clone)]
pub enum FilterToken {
    KeyValue(FilterKey, String),
    Free(String),
}

#[derive(Debug, Clone)]
pub struct FilterState {
    pub raw: String,
    pub tokens: Vec<FilterToken>,
}

impl FilterState {
    pub fn parse(input: &str) -> Option<Self> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return None;
        }
        let mut tokens = Vec::new();
        for part in trimmed.split_whitespace() {
            if let Some((key, value)) = part.split_once(':') {
                if let Some(key) = parse_filter_key(key) {
                    tokens.push(FilterToken::KeyValue(key, value.to_lowercase()));
                } else {
                    tokens.push(FilterToken::Free(part.to_lowercase()));
                }
            } else {
                tokens.push(FilterToken::Free(part.to_lowercase()));
            }
        }
        Some(Self {
            raw: trimmed.to_string(),
            tokens,
        })
    }
}

fn parse_filter_key(key: &str) -> Option<FilterKey> {
    match key.to_lowercase().as_str() {
        "block" | "blk" | "number" => Some(FilterKey::Block),
        "tx" => Some(FilterKey::Tx),
        "hash" => Some(FilterKey::Hash),
        "from" => Some(FilterKey::From),
        "to" => Some(FilterKey::To),
        "method" => Some(FilterKey::Method),
        "status" => Some(FilterKey::Status),
        "addr" | "address" => Some(FilterKey::Addr),
        "label" | "tag" => Some(FilterKey::Label),
        "miner" => Some(FilterKey::Miner),
        _ => None,
    }
}

#[derive(Debug, Clone)]
pub struct BlockInfo {
    pub number: u64,
    pub tx_count: u32,
    pub gas_used: u64,
    pub base_fee: u64,
    pub miner: String,
}

impl BlockInfo {
    fn mock(number: u64) -> Self {
        let tx_count = (number % 190) as u32 + 10;
        let gas_used = 12_000_000 + (number % 6) * 800_000;
        let base_fee = 12 + (number % 7);
        let miner = format!("0x{:0>40x}", number % 1024 + 2024);
        Self {
            number,
            tx_count,
            gas_used,
            base_fee,
            miner,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxStatus {
    Success,
    Revert,
    Unknown,
}

/// Decoded function argument
#[derive(Debug, Clone)]
pub struct DecodedArg {
    pub name: String,
    pub kind: String,
    pub value: String,
}

/// Token balance result
#[derive(Debug, Clone)]
pub struct TokenBalance {
    pub token: String,
    pub symbol: String,
    pub decimals: Option<u8>,
    pub balance: String,
}

#[derive(Debug, Clone)]
pub struct TxInfo {
    pub hash: String,
    pub from: String,
    pub to: String,
    pub value: f64,
    pub gas_used: u64,
    pub status: TxStatus,
    pub input: String,
    pub selector: String,
    pub method: String,
    pub signature: Option<String>,
    pub decoded_args: Option<Vec<DecodedArg>>,
    pub decode_error: Option<String>,
    pub block_number: u64,
}

impl TxInfo {
    fn mock(seed: u64, block_number: u64) -> Self {
        let hash = format!("0x{:0>64x}", seed * 1_000_007);
        let from = format!("0x{:0>40x}", seed * 37 + 100);
        let to = format!("0x{:0>40x}", seed * 91 + 300);
        let value = (seed % 5) as f64 * 0.32 + 0.05;
        let gas_used = 21_000 + (seed % 12) * 12_000;
        let status = if seed % 7 == 0 {
            TxStatus::Revert
        } else {
            TxStatus::Success
        };
        let method = match seed % 5 {
            0 => "transfer",
            1 => "swap",
            2 => "approve",
            3 => "mint",
            _ => "exec",
        };
        let selector = format!("0x{:08x}", (seed as u32).wrapping_mul(97));
        let input = format!("{selector}{}", "00".repeat(32));
        Self {
            hash,
            from,
            to,
            value,
            gas_used,
            status,
            input,
            selector,
            method: method.to_string(),
            signature: None,
            decoded_args: None,
            decode_error: None,
            block_number,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum AddressKind {
    Eoa,
    Contract,
}

#[derive(Debug, Clone)]
pub struct AddressInfo {
    pub address: String,
    pub label: Option<String>,
    pub balance: f64,
    pub delta: f64,
    pub kind: AddressKind,
}

impl AddressInfo {
    fn mock(seed: u64, kind: AddressKind) -> Self {
        let address = format!("0x{:0>40x}", seed * 19 + 400);
        let label = if seed % 4 == 0 {
            Some(format!("Vault-{}", seed % 13))
        } else if seed % 6 == 0 {
            Some(format!("Deployer-{}", seed % 7))
        } else {
            None
        };
        let balance = (seed % 20) as f64 * 1.2 + 0.4;
        let delta = if seed % 3 == 0 { 0.25 } else { -0.12 };
        Self {
            address,
            label,
            balance,
            delta,
            kind,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ContractInfo {
    pub address: String,
    pub label: Option<String>,
    pub methods: u32,
    pub tx_count: u32,
    pub last_call: u64,
    pub balance: f64,
    pub delta: f64,
}

impl ContractInfo {
    fn mock(seed: u64) -> Self {
        let address = format!("0x{:0>40x}", seed * 53 + 700);
        let label = if seed % 2 == 0 {
            Some(format!("Router-{}", seed % 9))
        } else {
            Some(format!("Vault-{}", seed % 5))
        };
        let methods = (seed % 12 + 4) as u32;
        let tx_count = (seed % 48 + 10) as u32;
        let last_call = 12_000_000 + seed;
        let balance = (seed % 15) as f64 * 0.9 + 0.2;
        let delta = if seed % 2 == 0 { 0.08 } else { -0.05 };
        Self {
            address,
            label,
            methods,
            tx_count,
            last_call,
            balance,
            delta,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallStatus {
    Ok,
    Revert,
}

#[derive(Debug, Clone)]
pub struct TraceFrame {
    pub depth: usize,
    pub call: String,
    pub from: String,
    pub to: String,
    pub value: f64,
    pub gas_used: u64,
    pub status: CallStatus,
    pub note: String,
    pub collapsed: bool,
    pub input: Option<String>,
    pub selector: Option<String>,
    pub method: Option<String>,
    pub signature: Option<String>,
    pub decoded_args: Option<Vec<DecodedArg>>,
    pub decode_error: Option<String>,
}

impl TraceFrame {
    fn mock(depth: usize, seed: u64) -> Self {
        let call = match seed % 4 {
            0 => "CALL",
            1 => "DELEGATECALL",
            2 => "STATICCALL",
            _ => "CALL",
        };
        let from = format!("0x{:0>40x}", seed * 13 + 123);
        let to = format!("0x{:0>40x}", seed * 29 + 555);
        let value = (seed % 4) as f64 * 0.15;
        let gas_used = 12_000 + (seed % 7) * 5_000;
        let status = if seed % 5 == 0 {
            CallStatus::Revert
        } else {
            CallStatus::Ok
        };
        let note = if status == CallStatus::Revert {
            "revert: insufficient output"
        } else {
            "ok"
        };
        let selector = format!("0x{:08x}", (seed as u32).wrapping_mul(13));
        let input = format!("{selector}{}", "00".repeat(16));
        Self {
            depth,
            call: call.to_string(),
            from,
            to,
            value,
            gas_used,
            status,
            note: note.to_string(),
            collapsed: false,
            input: Some(input),
            selector: Some(selector),
            method: None,
            signature: None,
            decoded_args: None,
            decode_error: None,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct CommandBar {
    pub input: String,
    pub last: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StatusMessage {
    pub text: String,
    pub level: StatusLevel,
    pub since: Instant,
}

#[derive(Debug, Clone)]
pub struct PendingChord {
    pub key: char,
    pub since: Instant,
}

#[derive(Debug, Clone)]
pub struct StorageRequest {
    pub address: String,
    pub slot: String,
}

/// Sections in the Explorer tab sidebar
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplorerSection {
    Blocks,
    Transactions,
    Addresses,
    Contracts,
}

impl ExplorerSection {
    pub const ALL: [ExplorerSection; 4] = [
        ExplorerSection::Blocks,
        ExplorerSection::Transactions,
        ExplorerSection::Addresses,
        ExplorerSection::Contracts,
    ];

    pub fn title(&self) -> &'static str {
        match self {
            ExplorerSection::Blocks => "Blocks",
            ExplorerSection::Transactions => "Txs",
            ExplorerSection::Addresses => "Addrs",
            ExplorerSection::Contracts => "Contracts",
        }
    }
}

/// Tools in the Toolkit tab
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolkitTool {
    Encode,
    Decode,
    Hash,
    Hex,
    Selector,
    FourByte,
    Convert,
    Timestamp,
    Call,
    Gas,
    Slot,
    Create,
    Create2,
    Checksum,
}

impl ToolkitTool {
    pub const ALL: [ToolkitTool; 14] = [
        ToolkitTool::Encode,
        ToolkitTool::Decode,
        ToolkitTool::Hash,
        ToolkitTool::Hex,
        ToolkitTool::Selector,
        ToolkitTool::FourByte,
        ToolkitTool::Convert,
        ToolkitTool::Timestamp,
        ToolkitTool::Call,
        ToolkitTool::Gas,
        ToolkitTool::Slot,
        ToolkitTool::Create,
        ToolkitTool::Create2,
        ToolkitTool::Checksum,
    ];

    pub fn title(&self) -> &'static str {
        match self {
            ToolkitTool::Encode => "encode",
            ToolkitTool::Decode => "decode",
            ToolkitTool::Hash => "hash",
            ToolkitTool::Hex => "hex",
            ToolkitTool::Selector => "selector",
            ToolkitTool::FourByte => "4byte",
            ToolkitTool::Convert => "convert",
            ToolkitTool::Timestamp => "time",
            ToolkitTool::Call => "call",
            ToolkitTool::Gas => "gas",
            ToolkitTool::Slot => "slot",
            ToolkitTool::Create => "create",
            ToolkitTool::Create2 => "create2",
            ToolkitTool::Checksum => "checksum",
        }
    }

    pub fn category(&self) -> &'static str {
        match self {
            ToolkitTool::Encode | ToolkitTool::Decode => "ABI",
            ToolkitTool::Hash | ToolkitTool::Hex | ToolkitTool::Selector | ToolkitTool::FourByte => "HASH",
            ToolkitTool::Convert | ToolkitTool::Timestamp | ToolkitTool::Checksum => "CONVERT",
            ToolkitTool::Call | ToolkitTool::Gas | ToolkitTool::Slot | ToolkitTool::Create | ToolkitTool::Create2 => "CONTRACT",
        }
    }
}

/// State for the Toolkit tab
#[derive(Debug, Clone)]
pub struct ToolkitState {
    pub selected_tool: ToolkitTool,
    pub input: String,
    pub output: String,
    pub history: Vec<String>,
    pub history_index: Option<usize>,
}

impl Default for ToolkitState {
    fn default() -> Self {
        Self {
            selected_tool: ToolkitTool::Encode,
            input: String::new(),
            output: String::new(),
            history: Vec::new(),
            history_index: None,
        }
    }
}

/// Sections in the Ops tab sidebar
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpsSection {
    Health,
    Peers,
    Mempool,
    Logs,
    Metrics,
    Alerts,
    RpcStats,
}

impl OpsSection {
    pub const ALL: [OpsSection; 7] = [
        OpsSection::Health,
        OpsSection::Peers,
        OpsSection::Mempool,
        OpsSection::Logs,
        OpsSection::Metrics,
        OpsSection::Alerts,
        OpsSection::RpcStats,
    ];

    pub fn title(&self) -> &'static str {
        match self {
            OpsSection::Health => "health",
            OpsSection::Peers => "peers",
            OpsSection::Mempool => "mempool",
            OpsSection::Logs => "logs",
            OpsSection::Metrics => "metrics",
            OpsSection::Alerts => "alerts",
            OpsSection::RpcStats => "rpc-stats",
        }
    }
}

#[derive(Debug, Clone)]
pub struct RpcEndpointOption {
    pub label: String,
    pub display: String,
}

#[derive(Debug)]
pub struct App {
    /// Shared context for modules
    pub ctx: Context,
    /// Current active tab
    pub current_tab: Tab,
    pub dashboard: crate::modules::dashboard::Dashboard,
    /// Explorer tab sidebar selection (Blocks, Txs, Addrs, Contracts)
    pub explorer_section: ExplorerSection,
    /// Toolkit tab state
    pub toolkit_state: ToolkitState,
    /// Ops tab state
    pub ops_section: OpsSection,
    pub view_stack: Vec<View>,
    pub active_section: Section,
    pub focus: Focus,
    pub input_mode: InputMode,
    pub data_mode: DataMode,
    pub abi_registry: Option<AbiRegistry>,
    pub abi_reload_sender: Option<Sender<AbiScanRequest>>,
    pub abi_scan_roots: Vec<std::path::PathBuf>,
    pub tokens: Vec<TokenSpec>,
    pub labels: BTreeMap<String, String>,
    pub label_store: Option<LabelStore>,
    pub blocks: Vec<BlockInfo>,
    pub txs: Vec<TxInfo>,
    pub addresses: Vec<AddressInfo>,
    pub contracts: Vec<ContractInfo>,
    pub traces: Vec<TraceFrame>,
    pub selected_block: usize,
    pub selected_tx: usize,
    pub selected_address: usize,
    pub selected_contract: usize,
    pub selected_trace: usize,
    pub pinned_blocks: BTreeSet<u64>,
    pub watched_addresses: BTreeSet<String>,
    pub paused: bool,
    pub follow_blocks: bool,
    pub follow_txs: bool,
    pub max_blocks: usize,
    pub max_txs: usize,
    pub command: CommandBar,
    pub active_filter: Option<FilterState>,
    pub rpc_endpoint: String,
    pub rpc_endpoints: Vec<RpcEndpointOption>,
    pub rpc_endpoint_index: usize,
    pub node_kind: String,
    pub last_rtt_ms: Option<u64>,
    pub peer_count: u32,
    pub sync_progress: f64,
    pub status: Option<StatusMessage>,
    pub pending_chord: Option<PendingChord>,
    pub pending_endpoint_switch: Option<usize>,
    pub pending_trace_request: Option<String>,
    pub pending_refresh_request: bool,
    pub pending_balance_request: Option<String>,
    pub pending_storage_request: Option<StorageRequest>,
    pub token_balances: BTreeMap<(String, String), String>,
    pub storage_cache: BTreeMap<(String, String), String>,
    /// Cache of resolved function signatures: selector -> (name, full_signature)
    pub signature_cache: BTreeMap<String, (String, String)>,
    pub prompt_context: Option<String>,
    pub settings_open: bool,
    pub help_open: bool,
    pub should_quit: bool,
    last_block_at: Instant,
    block_interval: Duration,
    next_block_number: u64,
    next_tx_seed: u64,
}

impl App {
    pub fn new() -> Self {
        let mut app = Self {
            ctx: Context::new(),
            current_tab: Tab::Home,
            dashboard: crate::modules::dashboard::Dashboard::new(),
            explorer_section: ExplorerSection::Blocks,
            toolkit_state: ToolkitState::default(),
            ops_section: OpsSection::Health,
            view_stack: vec![View::Dashboard],
            active_section: Section::Overview,
            focus: Focus::Sidebar,
            input_mode: InputMode::Normal,
            data_mode: DataMode::Mock,
            abi_registry: None,
            abi_reload_sender: None,
            abi_scan_roots: Vec::new(),
            tokens: Vec::new(),
            labels: BTreeMap::new(),
            label_store: None,
            blocks: Vec::new(),
            txs: Vec::new(),
            addresses: Vec::new(),
            contracts: Vec::new(),
            traces: Vec::new(),
            selected_block: 0,
            selected_tx: 0,
            selected_address: 0,
            selected_contract: 0,
            selected_trace: 0,
            pinned_blocks: BTreeSet::new(),
            watched_addresses: BTreeSet::new(),
            paused: false,
            follow_blocks: true,
            follow_txs: true,
            max_blocks: 50,
            max_txs: 120,
            command: CommandBar::default(),
            active_filter: None,
            rpc_endpoint: "localhost:8545".to_string(),
            rpc_endpoints: Vec::new(),
            rpc_endpoint_index: 0,
            node_kind: "unknown".to_string(),
            last_rtt_ms: None,
            peer_count: 0,
            sync_progress: 0.0,
            status: None,
            pending_chord: None,
            pending_endpoint_switch: None,
            pending_trace_request: None,
            pending_refresh_request: false,
            pending_balance_request: None,
            pending_storage_request: None,
            token_balances: BTreeMap::new(),
            storage_cache: BTreeMap::new(),
            signature_cache: BTreeMap::new(),
            prompt_context: None,
            settings_open: false,
            help_open: false,
            should_quit: false,
            last_block_at: Instant::now(),
            block_interval: Duration::from_secs(1),
            next_block_number: 12_000_000,
            next_tx_seed: 900_000,
        };
        app.seed_mock();
        app
    }

    /// Sync context with app state
    pub fn sync_context(&mut self) {
        self.ctx.labels = self.labels.clone();
        self.ctx.rpc_endpoint = self.rpc_endpoint.clone();
        self.ctx.node_kind = self.node_kind.clone();
        self.ctx.paused = self.paused;

        // Update selected based on current view and selection
        self.ctx.selected = match self.current_view() {
            View::BlockDetail => {
                if let Some(block) = self.blocks.get(self.selected_block) {
                    crate::core::Selected::Block(block.number)
                } else {
                    crate::core::Selected::None
                }
            }
            View::TxDetail => {
                if let Some(tx) = self.txs.get(self.selected_tx) {
                    crate::core::Selected::Transaction(tx.hash.clone())
                } else {
                    crate::core::Selected::None
                }
            }
            View::AddressDetail | View::ContractDetail => {
                if let Some(addr) = self.addresses.get(self.selected_address) {
                    crate::core::Selected::Address(addr.address.clone())
                } else if let Some(contract) = self.contracts.get(self.selected_contract) {
                    crate::core::Selected::Address(contract.address.clone())
                } else {
                    crate::core::Selected::None
                }
            }
            View::Trace => crate::core::Selected::TraceFrame,
            _ => crate::core::Selected::None,
        };
    }

    pub fn current_view(&self) -> View {
        *self.view_stack.last().unwrap_or(&View::Overview)
    }

    pub fn view_breadcrumb(&self) -> String {
        let mut parts = Vec::new();

        parts.push(self.active_section.title().to_string());

        for view in self.view_stack.iter().skip(1) {
            match view {
                View::BlockDetail => {
                    if let Some(block) = self.selected_block() {
                        parts.push(format!("#{}", block.number));
                    } else {
                        parts.push("Block".to_string());
                    }
                }
                View::TxDetail => {
                    if let Some(tx) = self.selected_tx() {
                        parts.push(short_hash(&tx.hash, 16));
                    } else {
                        parts.push("Tx".to_string());
                    }
                }
                View::AddressDetail => {
                    if let Some(addr) = self.selected_address() {
                        parts.push(short_addr(&addr.address));
                    } else {
                        parts.push("Address".to_string());
                    }
                }
                View::ContractDetail => {
                    if let Some(contract) = self.selected_contract() {
                        parts.push(short_addr(&contract.address));
                    } else {
                        parts.push("Contract".to_string());
                    }
                }
                View::Trace => {
                    parts.push("Trace".to_string());
                }
                View::Dashboard => {}
                View::Overview => {}
            }
        }

        parts.join(" / ")
    }

    pub fn focus_label(&self) -> &'static str {
        match self.focus {
            Focus::Sidebar => "Sidebar",
            Focus::List => "List",
            Focus::Details => "Inspector",
            Focus::Command => "Command",
        }
    }

    pub fn set_status(&mut self, text: impl Into<String>, level: StatusLevel) {
        self.status = Some(StatusMessage {
            text: text.into(),
            level,
            since: Instant::now(),
        });
    }

    pub fn status_text(&self) -> Option<(&str, StatusLevel)> {
        self.status
            .as_ref()
            .map(|status| (status.text.as_str(), status.level))
    }

    pub fn on_tick(&mut self) {
        self.clear_expired_chord();
        if let Some(status) = self.status.as_ref() {
            if status.since.elapsed() > Duration::from_secs(3) {
                self.status = None;
            }
        }
        if self.paused {
            return;
        }
        if self.data_mode == DataMode::Mock && self.last_block_at.elapsed() >= self.block_interval {
            self.append_block();
            self.peer_count = 8 + (self.next_block_number % 7) as u32;
            self.last_rtt_ms = Some(18 + (self.next_block_number % 9) as u64);
            if self.sync_progress < 1.0 {
                self.sync_progress = (self.sync_progress + 0.01).min(1.0);
            }
            self.last_block_at = Instant::now();
        }
        self.clamp_all_selections();
    }

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
        if self.paused {
            self.set_status("Paused live updates", StatusLevel::Warn);
        } else {
            self.set_status("Resumed live updates", StatusLevel::Info);
        }
    }

    pub fn refresh(&mut self) {
        match self.data_mode {
            DataMode::Mock => {
                self.blocks.clear();
                self.txs.clear();
                self.traces.clear();
                self.next_block_number = 12_000_000;
                self.next_tx_seed = 900_000;
                self.seed_mock();
                self.selected_block = self.blocks.len().saturating_sub(1);
                self.selected_tx = self.txs.len().saturating_sub(1);
                self.follow_blocks = true;
                self.follow_txs = true;
                self.set_status("Refreshed dashboard", StatusLevel::Info);
                self.clamp_all_selections();
            }
            DataMode::Rpc => {
                self.pending_refresh_request = true;
                self.blocks.clear();
                self.txs.clear();
                self.traces.clear();
                self.contracts.clear();
                self.token_balances.clear();
                self.storage_cache.clear();
                self.selected_block = 0;
                self.selected_tx = 0;
                self.selected_trace = 0;
                self.follow_blocks = true;
                self.follow_txs = true;
                self.set_status("Refreshing from RPC…", StatusLevel::Info);
            }
        }
    }

    pub fn cycle_section(&mut self, forward: bool) {
        let index = Section::ALL
            .iter()
            .position(|section| *section == self.active_section)
            .unwrap_or(0);
        let next = if forward {
            (index + 1) % Section::ALL.len()
        } else {
            (index + Section::ALL.len() - 1) % Section::ALL.len()
        };
        self.active_section = Section::ALL[next];
        self.reset_view();
    }

    pub fn set_section(&mut self, section: Section) {
        self.active_section = section;
        self.reset_view();
    }

    pub fn move_selection_up(&mut self) {
        match self.list_kind() {
            ListKind::Blocks => {
                if self.selected_block > 0 {
                    self.selected_block -= 1;
                }
                self.follow_blocks = false;
            }
            ListKind::Transactions => {
                if self.selected_tx > 0 {
                    self.selected_tx -= 1;
                }
                self.follow_txs = false;
            }
            ListKind::Addresses => {
                if self.selected_address > 0 {
                    self.selected_address -= 1;
                }
            }
            ListKind::Contracts => {
                if self.selected_contract > 0 {
                    self.selected_contract -= 1;
                }
            }
            ListKind::Trace => {
                if self.selected_trace > 0 {
                    self.selected_trace -= 1;
                }
            }
        }
    }

    pub fn move_selection_down(&mut self) {
        let list_len = self.list_len();
        match self.list_kind() {
            ListKind::Blocks => {
                if self.selected_block + 1 < list_len {
                    self.selected_block += 1;
                }
                self.follow_blocks = self.selected_block + 1 == list_len;
            }
            ListKind::Transactions => {
                if self.selected_tx + 1 < list_len {
                    self.selected_tx += 1;
                }
                self.follow_txs = self.selected_tx + 1 == list_len;
            }
            ListKind::Addresses => {
                if self.selected_address + 1 < list_len {
                    self.selected_address += 1;
                }
            }
            ListKind::Contracts => {
                if self.selected_contract + 1 < list_len {
                    self.selected_contract += 1;
                }
            }
            ListKind::Trace => {
                if self.selected_trace + 1 < list_len {
                    self.selected_trace += 1;
                }
            }
        }
    }

    pub fn toggle_pin(&mut self) {
        if let Some(number) = self.selected_block().map(|block| block.number) {
            if self.pinned_blocks.contains(&number) {
                self.pinned_blocks.remove(&number);
                self.set_status("Unpinned block", StatusLevel::Info);
            } else {
                self.pinned_blocks.insert(number);
                self.set_status("Pinned block", StatusLevel::Info);
            }
        }
    }

    #[allow(dead_code)]
    pub fn toggle_watch(&mut self) {
        let address = match self.list_kind() {
            ListKind::Addresses => self.selected_address().map(|item| item.address.clone()),
            ListKind::Contracts => self.selected_contract().map(|item| item.address.clone()),
            _ => None,
        };
        if let Some(address) = address {
            if self.watched_addresses.contains(&address) {
                self.watched_addresses.remove(&address);
                self.set_status("Removed watch", StatusLevel::Info);
            } else {
                self.watched_addresses.insert(address);
                self.set_status("Watching address", StatusLevel::Info);
            }
        }
    }

    pub fn selected_block(&self) -> Option<&BlockInfo> {
        self.selected_block_index()
            .and_then(|idx| self.blocks.get(idx))
    }

    pub fn selected_tx(&self) -> Option<&TxInfo> {
        self.selected_tx_index().and_then(|idx| self.txs.get(idx))
    }

    pub fn selected_address(&self) -> Option<&AddressInfo> {
        self.selected_address_index()
            .and_then(|idx| self.addresses.get(idx))
    }

    pub fn selected_contract(&self) -> Option<&ContractInfo> {
        self.selected_contract_index()
            .and_then(|idx| self.contracts.get(idx))
    }

    pub fn selected_trace(&self) -> Option<&TraceFrame> {
        self.trace_visible_indices()
            .get(self.selected_trace)
            .and_then(|idx| self.traces.get(*idx))
    }

    pub fn enter_command(&mut self) {
        self.input_mode = InputMode::Command;
        self.focus = Focus::Command;
        self.command.input.clear();
    }

    pub fn exit_command(&mut self) {
        self.input_mode = InputMode::Normal;
        self.focus = Focus::List;
        self.command.input.clear();
    }

    pub fn enter_prompt(&mut self, kind: PromptKind, context: String) {
        self.input_mode = InputMode::Prompt(kind);
        self.focus = Focus::Command;
        self.prompt_context = Some(context);
        self.command.input.clear();
    }

    pub fn exit_prompt(&mut self) {
        self.input_mode = InputMode::Normal;
        self.focus = Focus::List;
        self.prompt_context = None;
        self.command.input.clear();
    }

    pub fn apply_command(&mut self) {
        let input = self.command.input.trim().to_string();
        if input.is_empty() {
            self.exit_command();
            return;
        }

        // Try as a : command first (blocks, encode, health, etc.)
        let cmd = crate::core::parse_command(&input);
        if !matches!(cmd, crate::core::Command::Unknown(_)) {
            let action = self.execute_command(&cmd);
            self.apply_action(action);
            self.command.last = Some(input);
            self.exit_command();
            return;
        }

        let lowered = input.to_lowercase();
        if matches!(lowered.as_str(), "clear" | "reset" | "none") {
            self.active_filter = None;
            self.command.last = Some(input);
            self.set_status("Filter cleared", StatusLevel::Info);
            self.exit_command();
            self.clamp_all_selections();
            return;
        }
        if let Some(target) = parse_search_target(&input) {
            self.active_filter = None;
            self.command.last = Some(input.clone());
            let jumped = match target {
                SearchTarget::Block(number) => self.jump_to_block(number),
                SearchTarget::Tx(hash) => self.jump_to_tx(hash),
                SearchTarget::Address(addr) => self.jump_to_address(addr),
            };
            if !jumped {
                self.set_status("Search target not found", StatusLevel::Error);
            }
            self.exit_command();
            self.clamp_all_selections();
            return;
        }
        if let Some(filter) = FilterState::parse(&input) {
            self.command.last = Some(filter.raw.clone());
            self.active_filter = Some(filter);
            self.follow_blocks = false;
            self.follow_txs = false;
            self.set_status(format!("Filter applied: {input}"), StatusLevel::Info);
            self.exit_command();
            self.clamp_all_selections();
        } else {
            self.set_status("Filter ignored: empty input", StatusLevel::Warn);
            self.exit_command();
        }
    }

    pub fn apply_prompt(&mut self, kind: PromptKind) {
        let input = self.command.input.trim();
        match kind {
            PromptKind::StorageSlot => {
                let Some(address) = self.prompt_context.clone() else {
                    self.set_status("Missing storage context", StatusLevel::Error);
                    self.exit_prompt();
                    return;
                };
                let Some(slot) = normalize_storage_slot(input) else {
                    self.set_status("Invalid slot (use 0x.. or decimal)", StatusLevel::Warn);
                    self.exit_prompt();
                    return;
                };
                self.request_storage_at(address, slot);
                self.exit_prompt();
            }
            PromptKind::Label => {
                let Some(address) = self.prompt_context.clone() else {
                    self.set_status("Missing label context", StatusLevel::Error);
                    self.exit_prompt();
                    return;
                };
                let normalized = normalize_hex_address(&address);
                let label = input.trim();
                if label.is_empty() {
                    self.labels.remove(&normalized);
                    for item in &mut self.addresses {
                        if item.address.eq_ignore_ascii_case(&address) {
                            item.label = None;
                        }
                    }
                    for item in &mut self.contracts {
                        if item.address.eq_ignore_ascii_case(&address) {
                            item.label = None;
                        }
                    }
                    if let Some(store) = self.label_store.as_ref() {
                        if let Err(err) = store.remove_label(&normalized) {
                            self.set_status(format!("Label db error: {err}"), StatusLevel::Warn);
                        }
                    }
                    self.set_status("Label removed", StatusLevel::Info);
                    self.exit_prompt();
                    return;
                }

                let label = label.to_string();
                self.labels.insert(normalized.clone(), label.clone());
                for item in &mut self.addresses {
                    if item.address.eq_ignore_ascii_case(&address) {
                        item.label = Some(label.clone());
                    }
                }
                for item in &mut self.contracts {
                    if item.address.eq_ignore_ascii_case(&address) {
                        item.label = Some(label.clone());
                    }
                }
                if let Some(store) = self.label_store.as_ref() {
                    if let Err(err) = store.set_label(&normalized, &label) {
                        self.set_status(format!("Label db error: {err}"), StatusLevel::Warn);
                    }
                }
                self.set_status("Label saved", StatusLevel::Info);
                self.exit_prompt();
            }
        }
    }

    pub fn push_view(&mut self, view: View) {
        self.view_stack.push(view);
    }

    pub fn pop_view(&mut self) {
        if self.view_stack.len() > 1 {
            self.view_stack.pop();
        }
    }

    pub fn enter_detail(&mut self) {
        if self.current_view() != View::Overview {
            return;
        }
        match self.active_section {
            Section::Overview | Section::Blocks => {
                self.push_view(View::BlockDetail);
                self.selected_tx = 0;
                self.follow_blocks = false;
                self.follow_txs = false;
                self.clamp_all_selections();
            }
            Section::Transactions => {
                self.push_view(View::TxDetail);
                self.follow_blocks = false;
                self.follow_txs = false;
                self.clamp_all_selections();
            }
            Section::Addresses => {
                // Auto-fetch balance when entering address detail
                if let Some(addr) = self.selected_address() {
                    let address = addr.address.clone();
                    self.pending_balance_request = Some(address.clone());
                    self.set_status(format!("Fetching balance for {}...", &address[..10]), StatusLevel::Info);
                }
                self.push_view(View::AddressDetail);
                self.selected_tx = 0;
                self.follow_blocks = false;
                self.follow_txs = false;
                self.clamp_all_selections();
            }
            Section::Contracts => {
                // Auto-fetch balance when entering contract detail
                if let Some(contract) = self.selected_contract() {
                    let address = contract.address.clone();
                    self.pending_balance_request = Some(address.clone());
                    self.set_status(format!("Fetching balance for {}...", &address[..10]), StatusLevel::Info);
                }
                self.push_view(View::ContractDetail);
                self.selected_tx = 0;
                self.follow_blocks = false;
                self.follow_txs = false;
                self.clamp_all_selections();
            }
        }
    }

    pub fn enter_trace(&mut self) {
        if self.current_view() == View::Trace {
            return;
        }
        match self.data_mode {
            DataMode::Mock => {
                self.build_trace();
                self.push_view(View::Trace);
                self.focus = Focus::List;
            }
            DataMode::Rpc => {
                let Some(tx_hash) = self.selected_tx().map(|tx| tx.hash.clone()) else {
                    self.set_status("No transaction selected", StatusLevel::Warn);
                    return;
                };
                self.traces.clear();
                self.selected_trace = 0;
                self.pending_trace_request = Some(tx_hash);
                self.push_view(View::Trace);
                self.focus = Focus::List;
                self.set_status("Loading trace…", StatusLevel::Info);
            }
        }
    }

    pub fn can_enter_trace(&self) -> bool {
        matches!(self.list_kind(), ListKind::Transactions) && !self.filtered_tx_indices().is_empty()
    }

    pub fn toggle_trace_collapse(&mut self) {
        if self.current_view() != View::Trace {
            return;
        }
        if let Some(idx) = self
            .trace_visible_indices()
            .get(self.selected_trace)
            .copied()
        {
            if let Some(frame) = self.traces.get_mut(idx) {
                frame.collapsed = !frame.collapsed;
                self.clamp_all_selections();
            }
        }
    }

    pub fn list_kind(&self) -> ListKind {
        if self.current_view() == View::Trace {
            return ListKind::Trace;
        }
        if matches!(
            self.current_view(),
            View::BlockDetail | View::AddressDetail | View::ContractDetail
        ) {
            return ListKind::Transactions;
        }
        match self.active_section {
            Section::Overview | Section::Blocks => ListKind::Blocks,
            Section::Transactions => ListKind::Transactions,
            Section::Addresses => ListKind::Addresses,
            Section::Contracts => ListKind::Contracts,
        }
    }

    pub fn list_is_empty(&self) -> bool {
        self.list_len() == 0
    }

    pub fn list_len(&self) -> usize {
        match self.list_kind() {
            ListKind::Blocks => self.filtered_block_indices().len(),
            ListKind::Transactions => self.filtered_tx_indices().len(),
            ListKind::Addresses => self.filtered_address_indices().len(),
            ListKind::Contracts => self.filtered_contract_indices().len(),
            ListKind::Trace => self.trace_visible_indices().len(),
        }
    }

    pub fn filtered_block_indices(&self) -> Vec<usize> {
        self.blocks
            .iter()
            .enumerate()
            .filter(|(_, block)| self.matches_block(block))
            .map(|(idx, _)| idx)
            .collect()
    }

    pub fn filtered_tx_indices(&self) -> Vec<usize> {
        let mut indices: Vec<usize> = self
            .txs
            .iter()
            .enumerate()
            .filter(|(_, tx)| self.matches_tx(tx))
            .map(|(idx, _)| idx)
            .collect();

        match self.tx_scope() {
            TxScope::All => {}
            TxScope::Block(number) => indices.retain(|idx| {
                self.txs
                    .get(*idx)
                    .map(|tx| tx.block_number == number)
                    .unwrap_or(false)
            }),
            TxScope::Address(address) => indices.retain(|idx| {
                self.txs
                    .get(*idx)
                    .map(|tx| {
                        address_matches(&tx.from, address) || address_matches(&tx.to, address)
                    })
                    .unwrap_or(false)
            }),
        }

        indices
    }

    pub fn filtered_address_indices(&self) -> Vec<usize> {
        self.addresses
            .iter()
            .enumerate()
            .filter(|(_, addr)| self.matches_address(addr))
            .map(|(idx, _)| idx)
            .collect()
    }

    pub fn filtered_contract_indices(&self) -> Vec<usize> {
        self.contracts
            .iter()
            .enumerate()
            .filter(|(_, contract)| self.matches_contract(contract))
            .map(|(idx, _)| idx)
            .collect()
    }

    pub fn trace_visible_indices(&self) -> Vec<usize> {
        let mut visible = Vec::new();
        let mut hidden_depth: Option<usize> = None;
        for (idx, frame) in self.traces.iter().enumerate() {
            if let Some(depth) = hidden_depth {
                if frame.depth > depth {
                    continue;
                }
                hidden_depth = None;
            }
            visible.push(idx);
            if frame.collapsed {
                hidden_depth = Some(frame.depth);
            }
        }
        visible
    }

    fn seed_mock(&mut self) {
        for _ in 0..18 {
            self.append_block();
        }
        for i in 0..16 {
            self.addresses
                .push(AddressInfo::mock(i + 1, AddressKind::Eoa));
        }
        for i in 0..12 {
            self.contracts.push(ContractInfo::mock(i + 1));
        }
        self.selected_block = self.blocks.len().saturating_sub(1);
        self.selected_tx = self.txs.len().saturating_sub(1);
        self.clamp_all_selections();
    }

    fn append_block(&mut self) {
        let number = self.next_block_number;
        self.next_block_number += 1;
        let info = BlockInfo::mock(number);
        let was_tail = self.follow_blocks || self.selected_block + 1 == self.blocks.len();
        self.blocks.push(info);
        self.append_txs_for_block(number);
        if self.blocks.len() > self.max_blocks {
            let overflow = self.blocks.len() - self.max_blocks;
            self.blocks.drain(0..overflow);
            if self.selected_block >= overflow {
                self.selected_block -= overflow;
            } else {
                self.selected_block = 0;
            }
        }
        if was_tail {
            self.selected_block = self.blocks.len().saturating_sub(1);
        }
    }

    fn append_txs_for_block(&mut self, block_number: u64) {
        let was_tail = self.follow_txs || self.selected_tx + 1 == self.txs.len();
        let mut watch_hits = BTreeSet::new();
        for _ in 0..4 {
            let seed = self.next_tx_seed;
            self.next_tx_seed += 1;
            let tx = TxInfo::mock(seed, block_number);
            if self.watched_addresses.contains(&tx.from) {
                watch_hits.insert(tx.from.clone());
            }
            if self.watched_addresses.contains(&tx.to) {
                watch_hits.insert(tx.to.clone());
            }
            self.txs.push(tx);
        }
        if !watch_hits.is_empty() {
            self.set_status(
                format!(
                    "Watch hit in #{}: {}",
                    block_number,
                    watch_hits
                        .iter()
                        .take(2)
                        .map(|addr| short_addr(addr))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                StatusLevel::Warn,
            );
        }
        if self.txs.len() > self.max_txs {
            let overflow = self.txs.len() - self.max_txs;
            self.txs.drain(0..overflow);
            if self.selected_tx >= overflow {
                self.selected_tx -= overflow;
            } else {
                self.selected_tx = 0;
            }
        }
        if was_tail {
            self.selected_tx = self.txs.len().saturating_sub(1);
        }
    }

    fn build_trace(&mut self) {
        self.traces.clear();
        let Some(tx) = self.selected_tx().cloned() else {
            return;
        };
        let seed = seed_from_str(&tx.hash);
        self.traces.push(TraceFrame {
            depth: 0,
            call: "CALL".to_string(),
            from: tx.from.clone(),
            to: tx.to.clone(),
            value: tx.value,
            gas_used: tx.gas_used / 2,
            status: if matches!(tx.status, TxStatus::Revert) {
                CallStatus::Revert
            } else {
                CallStatus::Ok
            },
            note: if matches!(tx.status, TxStatus::Revert) {
                "revert: execution reverted"
            } else {
                "ok"
            }
            .to_string(),
            collapsed: false,
            input: Some(tx.input.clone()),
            selector: Some(tx.selector.clone()),
            method: Some(tx.method.clone()),
            signature: tx.signature.clone(),
            decoded_args: tx.decoded_args.clone(),
            decode_error: tx.decode_error.clone(),
        });

        let branches = 2 + (seed % 3) as usize;
        for i in 0..branches {
            let branch_seed = seed + i as u64 * 13;
            let depth = 1;
            self.traces.push(TraceFrame::mock(depth, branch_seed));
            let leaves = 1 + (branch_seed % 3) as usize;
            for j in 0..leaves {
                let leaf_seed = branch_seed + j as u64 * 7;
                self.traces.push(TraceFrame::mock(depth + 1, leaf_seed));
            }
        }
        self.selected_trace = 0;
    }

    fn tx_scope(&self) -> TxScope<'_> {
        match self.current_view() {
            View::BlockDetail => self
                .selected_block()
                .map(|block| TxScope::Block(block.number))
                .unwrap_or(TxScope::All),
            View::AddressDetail => self
                .selected_address()
                .map(|addr| TxScope::Address(addr.address.as_str()))
                .unwrap_or(TxScope::All),
            View::ContractDetail => self
                .selected_contract()
                .map(|contract| TxScope::Address(contract.address.as_str()))
                .unwrap_or(TxScope::All),
            _ => TxScope::All,
        }
    }

    pub fn jump_to_block(&mut self, number: u64) -> bool {
        let Some(raw_idx) = self.blocks.iter().position(|block| block.number == number) else {
            return false;
        };
        self.active_filter = None;
        self.active_section = Section::Blocks;
        self.reset_view();
        let filtered = self.filtered_block_indices();
        self.selected_block = filtered.iter().position(|idx| *idx == raw_idx).unwrap_or(0);
        self.push_view(View::BlockDetail);
        self.focus = Focus::List;
        self.set_status(format!("Jumped to block #{number}"), StatusLevel::Info);
        true
    }

    pub fn jump_to_tx(&mut self, hash: &str) -> bool {
        let Some(raw_idx) = self
            .txs
            .iter()
            .position(|tx| tx.hash.eq_ignore_ascii_case(hash))
        else {
            return false;
        };
        self.active_filter = None;
        self.active_section = Section::Transactions;
        self.reset_view();
        let filtered = self.filtered_tx_indices();
        self.selected_tx = filtered.iter().position(|idx| *idx == raw_idx).unwrap_or(0);
        self.push_view(View::TxDetail);
        self.focus = Focus::List;
        self.set_status(
            format!("Jumped to tx {}", short_hash(hash, 16)),
            StatusLevel::Info,
        );
        true
    }

    pub fn jump_to_address(&mut self, address: &str) -> bool {
        let raw_idx = match self
            .addresses
            .iter()
            .position(|item| item.address.eq_ignore_ascii_case(address))
        {
            Some(idx) => idx,
            None => {
                let label = self.labels.get(&normalize_hex_address(address)).cloned();
                self.addresses.push(AddressInfo {
                    address: address.to_string(),
                    label,
                    balance: 0.0,
                    delta: 0.0,
                    kind: AddressKind::Eoa,
                });
                self.addresses.len().saturating_sub(1)
            }
        };
        self.active_filter = None;
        self.active_section = Section::Addresses;
        self.reset_view();
        let filtered = self.filtered_address_indices();
        self.selected_address = filtered.iter().position(|idx| *idx == raw_idx).unwrap_or(0);
        self.push_view(View::AddressDetail);
        self.focus = Focus::List;
        self.set_status(
            format!("Jumped to address {}", short_addr(address)),
            StatusLevel::Info,
        );
        true
    }

    fn reset_view(&mut self) {
        self.view_stack.clear();
        self.view_stack.push(View::Overview);
    }

    fn selected_block_index(&self) -> Option<usize> {
        self.filtered_block_indices()
            .get(self.selected_block)
            .copied()
    }

    fn selected_tx_index(&self) -> Option<usize> {
        self.filtered_tx_indices().get(self.selected_tx).copied()
    }

    fn selected_address_index(&self) -> Option<usize> {
        self.filtered_address_indices()
            .get(self.selected_address)
            .copied()
    }

    fn selected_contract_index(&self) -> Option<usize> {
        self.filtered_contract_indices()
            .get(self.selected_contract)
            .copied()
    }

    fn clamp_all_selections(&mut self) {
        let blocks_len = self.filtered_block_indices().len();
        let tx_len = self.filtered_tx_indices().len();
        let addr_len = self.filtered_address_indices().len();
        let contract_len = self.filtered_contract_indices().len();
        let trace_len = self.trace_visible_indices().len();

        Self::clamp_selection(&mut self.selected_block, blocks_len);
        Self::clamp_selection(&mut self.selected_tx, tx_len);
        Self::clamp_selection(&mut self.selected_address, addr_len);
        Self::clamp_selection(&mut self.selected_contract, contract_len);
        Self::clamp_selection(&mut self.selected_trace, trace_len);
    }

    fn clamp_selection(selection: &mut usize, len: usize) {
        if len == 0 {
            *selection = 0;
        } else if *selection >= len {
            *selection = len - 1;
        }
    }

    fn matches_block(&self, block: &BlockInfo) -> bool {
        let Some(filter) = self.active_filter.as_ref() else {
            return true;
        };
        filter.tokens.iter().all(|token| match token {
            FilterToken::KeyValue(key, value) => match key {
                FilterKey::Block => number_matches(block.number, value),
                FilterKey::Miner | FilterKey::Addr => {
                    contains_case_insensitive(&block.miner, value)
                }
                FilterKey::Tx => number_matches(block.tx_count as u64, value),
                _ => false,
            },
            FilterToken::Free(value) => {
                number_matches(block.number, value)
                    || contains_case_insensitive(&block.miner, value)
                    || number_matches(block.tx_count as u64, value)
            }
        })
    }

    fn matches_tx(&self, tx: &TxInfo) -> bool {
        let Some(filter) = self.active_filter.as_ref() else {
            return true;
        };
        filter.tokens.iter().all(|token| match token {
            FilterToken::KeyValue(key, value) => match key {
                FilterKey::Block => number_matches(tx.block_number, value),
                FilterKey::Tx | FilterKey::Hash => contains_case_insensitive(&tx.hash, value),
                FilterKey::From => contains_case_insensitive(&tx.from, value),
                FilterKey::To => contains_case_insensitive(&tx.to, value),
                FilterKey::Method => {
                    contains_case_insensitive(&tx.method, value)
                        || contains_case_insensitive(&tx.selector, value)
                        || tx
                            .signature
                            .as_ref()
                            .map(|sig| contains_case_insensitive(sig, value))
                            .unwrap_or(false)
                }
                FilterKey::Status => match value.as_str() {
                    "ok" | "success" => matches!(tx.status, TxStatus::Success),
                    "revert" | "fail" => matches!(tx.status, TxStatus::Revert),
                    "unknown" | "pending" | "?" | "??" => matches!(tx.status, TxStatus::Unknown),
                    _ => false,
                },
                _ => false,
            },
            FilterToken::Free(value) => {
                contains_case_insensitive(&tx.hash, value)
                    || contains_case_insensitive(&tx.from, value)
                    || contains_case_insensitive(&tx.to, value)
                    || contains_case_insensitive(&tx.method, value)
                    || contains_case_insensitive(&tx.selector, value)
                    || tx
                        .signature
                        .as_ref()
                        .map(|sig| contains_case_insensitive(sig, value))
                        .unwrap_or(false)
                    || number_matches(tx.block_number, value)
            }
        })
    }

    fn matches_address(&self, addr: &AddressInfo) -> bool {
        let Some(filter) = self.active_filter.as_ref() else {
            return true;
        };
        filter.tokens.iter().all(|token| match token {
            FilterToken::KeyValue(key, value) => match key {
                FilterKey::Addr => contains_case_insensitive(&addr.address, value),
                FilterKey::Label => addr
                    .label
                    .as_ref()
                    .map(|label| contains_case_insensitive(label, value))
                    .unwrap_or(false),
                _ => false,
            },
            FilterToken::Free(value) => {
                contains_case_insensitive(&addr.address, value)
                    || addr
                        .label
                        .as_ref()
                        .map(|label| contains_case_insensitive(label, value))
                        .unwrap_or(false)
            }
        })
    }

    fn matches_contract(&self, contract: &ContractInfo) -> bool {
        let Some(filter) = self.active_filter.as_ref() else {
            return true;
        };
        filter.tokens.iter().all(|token| match token {
            FilterToken::KeyValue(key, value) => match key {
                FilterKey::Addr => contains_case_insensitive(&contract.address, value),
                FilterKey::Label => contract
                    .label
                    .as_ref()
                    .map(|label| contains_case_insensitive(label, value))
                    .unwrap_or(false),
                _ => false,
            },
            FilterToken::Free(value) => {
                contains_case_insensitive(&contract.address, value)
                    || contract
                        .label
                        .as_ref()
                        .map(|label| contains_case_insensitive(label, value))
                        .unwrap_or(false)
            }
        })
    }
}

impl App {
    /// Execute a parsed command
    pub fn execute_command(&mut self, cmd: &crate::core::Command) -> crate::core::Action {
        use crate::core::{Action, Command, NavigateTarget, NotifyLevel};

        match cmd {
            // Navigation commands
            Command::Blocks => Action::Navigate(NavigateTarget::Blocks),
            Command::Transactions => Action::Navigate(NavigateTarget::Transactions),
            Command::Address(addr) => Action::Navigate(NavigateTarget::Address(addr.clone())),
            Command::Trace(hash) => Action::Navigate(NavigateTarget::Trace(hash.clone())),

            // Toolkit commands - implemented
            Command::Convert(args) => crate::modules::toolkit::convert::convert(args.clone()),
            Command::Hex(args) => crate::modules::toolkit::hex::hex_convert(args.clone()),
            Command::Hash(args) => crate::modules::toolkit::hash::hash(args.clone()),
            Command::Selector(args) => crate::modules::toolkit::selector::selector(args.clone()),
            Command::FourByte(args) => crate::modules::toolkit::fourbyte::fourbyte(args.clone(), &self.signature_cache),
            Command::Timestamp(args) => crate::modules::toolkit::timestamp::timestamp(args.clone()),
            Command::Checksum(args) => crate::modules::toolkit::checksum::checksum(args.clone()),

            // Toolkit commands - Phase 3
            Command::Encode(args) => crate::modules::toolkit::encode::encode(args.clone()),
            Command::Decode(args) => crate::modules::toolkit::decode::decode(args.clone(), &self.signature_cache),

            // Toolkit commands - Phase 3 (all implemented)
            Command::Slot(args) => crate::modules::toolkit::slot::slot(args.clone()),
            Command::Create(args) => crate::modules::toolkit::create::create_address(args.clone()),
            Command::Create2(args) => crate::modules::toolkit::create2::create2_address(args.clone()),
            Command::Call(args) => crate::modules::toolkit::call::call(args.clone()),
            Command::Gas(args) => crate::modules::toolkit::gas::estimate_gas(args.clone()),

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

            // Ops commands - Phase 5
            Command::Logs => crate::modules::ops::logs::logs(None),
            Command::Metrics => crate::modules::ops::metrics::metrics_unavailable(),
            Command::Alerts => {
                // Use current app state for alert checking
                crate::modules::ops::alerts::alerts(
                    self.peer_count,
                    self.last_rtt_ms,
                    self.sync_progress < 1.0,
                )
            }

            // Workflow commands - implemented
            Command::Connect(url) => {
                // Connection is handled by setting pending_endpoint_switch
                // This just acknowledges the command
                Action::Notify(format!("Connecting to {}...", url), NotifyLevel::Info)
            }
            Command::Anvil(args) => {
                let args_str = if args.is_empty() {
                    None
                } else {
                    Some(args.join(" "))
                };
                crate::modules::workflow::anvil::anvil(args_str)
            }
            Command::Impersonate(addr) => crate::modules::workflow::anvil_control::impersonate(Some(addr.clone())),
            Command::Mine(count) => crate::modules::workflow::anvil_control::mine(count.map(|c| c.to_string())),
            Command::Snapshot => crate::modules::workflow::anvil_control::snapshot(),
            Command::Revert(id) => crate::modules::workflow::anvil_control::revert(id.clone()),

            Command::Unknown(s) => Action::Notify(format!("Unknown command: {}", s), NotifyLevel::Warn),
        }
    }

    /// Apply an action returned by a command or module
    pub fn apply_action(&mut self, action: crate::core::Action) {
        use crate::core::{Action, NavigateTarget, NotifyLevel};

        match action {
            Action::None => {}
            Action::Navigate(target) => match target {
                NavigateTarget::Blocks => {
                    self.view_stack = vec![View::Overview];
                    self.active_section = Section::Blocks;
                    self.focus = Focus::List;
                }
                NavigateTarget::Transactions => {
                    self.view_stack = vec![View::Overview];
                    self.active_section = Section::Transactions;
                    self.focus = Focus::List;
                }
                NavigateTarget::Block(num) => {
                    if let Some(idx) = self.blocks.iter().position(|b| b.number == num) {
                        self.selected_block = idx;
                        self.active_section = Section::Blocks;
                        self.push_view(View::BlockDetail);
                    }
                }
                NavigateTarget::Transaction(hash) => {
                    if let Some(idx) = self.txs.iter().position(|t| t.hash == hash) {
                        self.selected_tx = idx;
                        self.active_section = Section::Transactions;
                        self.push_view(View::TxDetail);
                    }
                }
                NavigateTarget::Address(addr) => {
                    self.set_status(format!("Navigate to address: {}", addr), StatusLevel::Info);
                }
                NavigateTarget::Trace(hash) => {
                    self.pending_trace_request = Some(hash);
                }
            },
            Action::Notify(msg, level) => {
                let level = match level {
                    NotifyLevel::Info => StatusLevel::Info,
                    NotifyLevel::Warn => StatusLevel::Warn,
                    NotifyLevel::Error => StatusLevel::Error,
                };
                self.set_status(msg, level);
            }
        }
    }

    pub fn take_endpoint_switch_request(&mut self) -> Option<usize> {
        self.pending_endpoint_switch.take()
    }

    pub fn take_trace_request(&mut self) -> Option<String> {
        self.pending_trace_request.take()
    }

    pub fn take_refresh_request(&mut self) -> bool {
        if self.pending_refresh_request {
            self.pending_refresh_request = false;
            true
        } else {
            false
        }
    }

    pub fn take_balance_request(&mut self) -> Option<String> {
        self.pending_balance_request.take()
    }

    pub fn take_storage_request(&mut self) -> Option<StorageRequest> {
        self.pending_storage_request.take()
    }

    pub fn apply_abi_registry(&mut self, registry: AbiRegistry) {
        let count = registry.len();
        let scan_ms = registry.scan_ms;
        self.abi_registry = Some(registry);
        self.decode_all_txs();
        self.set_status(
            format!("ABI loaded: {count} selectors ({scan_ms}ms)"),
            StatusLevel::Info,
        );
    }

    pub fn request_abi_reload(&mut self) {
        let Some(sender) = self.abi_reload_sender.as_ref() else {
            self.set_status("ABI reload unavailable", StatusLevel::Warn);
            return;
        };
        if self.abi_scan_roots.is_empty() {
            self.set_status("ABI scan roots not configured", StatusLevel::Warn);
            return;
        }
        match sender.send(AbiScanRequest {
            roots: self.abi_scan_roots.clone(),
        }) {
            Ok(()) => self.set_status("Scanning ABI…", StatusLevel::Info),
            Err(err) => self.set_status(format!("ABI reload failed: {err}"), StatusLevel::Warn),
        }
    }

    pub fn cycle_rpc_endpoint(&mut self, forward: bool) {
        if self.rpc_endpoints.is_empty() {
            self.set_status("No RPC endpoints configured", StatusLevel::Warn);
            return;
        }
        let len = self.rpc_endpoints.len();
        let next = if forward {
            (self.rpc_endpoint_index + 1) % len
        } else {
            (self.rpc_endpoint_index + len - 1) % len
        };
        self.rpc_endpoint_index = next;
        self.pending_endpoint_switch = Some(next);
        let label = self
            .rpc_endpoints
            .get(next)
            .map(|endpoint| endpoint.label.as_str())
            .unwrap_or("--");
        self.set_status(format!("Switching RPC endpoint: {label}"), StatusLevel::Info);
    }

    pub fn request_balance(&mut self, address: String) {
        if self.data_mode != DataMode::Rpc {
            self.set_status("Balance poke requires RPC mode", StatusLevel::Warn);
            return;
        }
        self.pending_balance_request = Some(address);
        self.set_status("Fetching asset snapshot…", StatusLevel::Info);
    }

    pub fn request_storage_at(&mut self, address: String, slot: String) {
        if self.data_mode != DataMode::Rpc {
            self.set_status("Storage poke requires RPC mode", StatusLevel::Warn);
            return;
        }
        self.pending_storage_request = Some(StorageRequest { address, slot });
        self.set_status("Fetching storage…", StatusLevel::Info);
    }

    pub fn apply_rpc_connected(
        &mut self,
        endpoint: String,
        node_kind: String,
        accounts: Vec<String>,
    ) {
        self.data_mode = DataMode::Rpc;
        self.rpc_endpoint = endpoint;
        if let Some(index) = self
            .rpc_endpoints
            .iter()
            .position(|candidate| candidate.display.eq_ignore_ascii_case(&self.rpc_endpoint))
        {
            self.rpc_endpoint_index = index;
        }
        self.node_kind = node_kind;
        self.blocks.clear();
        self.txs.clear();
        self.traces.clear();
        self.addresses.clear();
        self.contracts.clear();
        self.token_balances.clear();
        self.storage_cache.clear();
        for addr in accounts {
            let label = self.labels.get(&normalize_hex_address(&addr)).cloned();
            self.addresses.push(AddressInfo {
                address: addr,
                label,
                balance: 0.0,
                delta: 0.0,
                kind: AddressKind::Eoa,
            });
        }
        self.follow_blocks = true;
        self.follow_txs = true;
        self.selected_block = 0;
        self.selected_tx = 0;
        self.selected_address = 0;
        self.selected_contract = 0;
        self.selected_trace = 0;
        self.set_status("Connected", StatusLevel::Info);
        self.clamp_all_selections();
    }

    pub fn apply_rpc_status(
        &mut self,
        rtt_ms: Option<u64>,
        peer_count: Option<u32>,
        sync_progress: Option<f64>,
    ) {
        if let Some(ms) = rtt_ms {
            self.last_rtt_ms = Some(ms);
        }
        if let Some(peers) = peer_count {
            self.peer_count = peers;
        }
        if let Some(progress) = sync_progress {
            self.sync_progress = progress;
        }
    }

    pub fn ingest_block(&mut self, block: BlockInfo, mut txs: Vec<TxInfo>) {
        self.decorate_txs_with_abi(&mut txs);
        self.observe_contracts_from_txs(&txs, block.number);
        // Only auto-follow if explicitly enabled AND in Overview (not detail views)
        let in_detail_view = self.current_view() != View::Overview;
        let was_tail = self.follow_blocks
            && !in_detail_view
            && self.selected_block + 1 == self.filtered_block_indices().len();
        self.blocks.push(block);
        if self.blocks.len() > self.max_blocks {
            let overflow = self.blocks.len() - self.max_blocks;
            self.blocks.drain(0..overflow);
        }

        let was_tx_tail = self.follow_txs
            && !in_detail_view
            && self.selected_tx + 1 == self.filtered_tx_indices().len();
        let mut watch_hits = BTreeSet::new();
        for tx in &txs {
            if self.watched_addresses.contains(&tx.from) {
                watch_hits.insert(tx.from.clone());
            }
            if self.watched_addresses.contains(&tx.to) {
                watch_hits.insert(tx.to.clone());
            }
        }

        self.txs.extend(txs);
        if self.txs.len() > self.max_txs {
            let overflow = self.txs.len() - self.max_txs;
            self.txs.drain(0..overflow);
        }

        if !watch_hits.is_empty() {
            self.set_status(
                format!(
                    "Watch hit: {}",
                    watch_hits
                        .iter()
                        .take(2)
                        .map(|addr| short_addr(addr))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                StatusLevel::Warn,
            );
        }

        self.clamp_all_selections();
        if was_tail {
            let len = self.filtered_block_indices().len();
            if len > 0 {
                self.selected_block = len - 1;
            }
        }
        if was_tx_tail {
            let len = self.filtered_tx_indices().len();
            if len > 0 {
                self.selected_tx = len - 1;
            }
        }
    }

    pub fn ingest_trace(&mut self, frames: Vec<TraceFrame>) {
        self.traces = frames;
        self.decorate_trace_with_abi();
        self.selected_trace = 0;
        self.clamp_all_selections();
        self.set_status("Trace loaded", StatusLevel::Info);
    }

    pub fn apply_balance(&mut self, address: String, balance_eth: f64) {
        let mut applied = false;
        for item in &mut self.addresses {
            if item.address.eq_ignore_ascii_case(&address) {
                let previous = item.balance;
                item.balance = balance_eth;
                item.delta = balance_eth - previous;
                applied = true;
            }
        }
        for item in &mut self.contracts {
            if item.address.eq_ignore_ascii_case(&address) {
                let previous = item.balance;
                item.balance = balance_eth;
                item.delta = balance_eth - previous;
                applied = true;
            }
        }
        if applied {
            self.set_status(
                format!("Balance updated: {:.6} ETH", balance_eth),
                StatusLevel::Info,
            );
        } else {
            self.set_status("Balance received (address not in list)", StatusLevel::Warn);
        }
    }

    pub fn apply_token_balances(&mut self, address: String, balances: Vec<TokenBalance>) {
        let owner = normalize_hex_address(&address);
        for balance in balances {
            let token = normalize_hex_address(&balance.token);
            self.token_balances
                .insert((owner.clone(), token), balance.balance);
            if let Some(spec) = self.tokens.iter_mut().find(|spec| {
                normalize_hex_address(&spec.address) == normalize_hex_address(&balance.token)
            }) {
                if spec
                    .symbol
                    .as_ref()
                    .map(|s| s.trim().is_empty())
                    .unwrap_or(true)
                {
                    spec.symbol = Some(balance.symbol.clone());
                }
                if spec.decimals.is_none() {
                    spec.decimals = balance.decimals;
                }
            }
        }
        self.set_status("Token balances updated", StatusLevel::Info);
    }

    pub fn apply_storage_value(&mut self, address: String, slot: String, value: String) {
        self.storage_cache.insert((address, slot), value);
        self.set_status("Storage updated", StatusLevel::Info);
    }

    /// Apply a resolved function signature from 4byte lookup
    pub fn apply_signature(&mut self, selector: String, name: String, signature: String) {
        // Store in cache
        self.signature_cache
            .insert(selector.clone(), (name.clone(), signature.clone()));

        // Update all transactions with this selector
        for tx in &mut self.txs {
            if tx.selector == selector && tx.signature.is_none() {
                tx.method = name.clone();
                tx.signature = Some(signature.clone());
            }
        }

        // Update traces with this selector
        for frame in &mut self.traces {
            if let Some(ref sel) = frame.selector {
                if sel == &selector && frame.signature.is_none() {
                    frame.method = Some(name.clone());
                    frame.signature = Some(signature.clone());
                }
            }
        }
    }

    pub fn apply_rpc_error(&mut self, message: String) {
        self.set_status(message, StatusLevel::Error);
    }

    pub fn set_list_selection(&mut self, selection: usize) {
        match self.list_kind() {
            ListKind::Blocks => {
                self.selected_block = selection;
                self.follow_blocks = selection + 1 == self.filtered_block_indices().len();
            }
            ListKind::Transactions => {
                self.selected_tx = selection;
                self.follow_txs = selection + 1 == self.filtered_tx_indices().len();
            }
            ListKind::Addresses => self.selected_address = selection,
            ListKind::Contracts => self.selected_contract = selection,
            ListKind::Trace => self.selected_trace = selection,
        }
        self.clamp_all_selections();
    }

    pub fn go_to_top(&mut self) {
        self.set_list_selection(0);
        self.follow_blocks = false;
        self.follow_txs = false;
    }

    pub fn go_to_bottom(&mut self) {
        let len = self.list_len();
        if len == 0 {
            return;
        }
        self.set_list_selection(len - 1);
    }

    pub fn page_up(&mut self, amount: usize) {
        let current = self.current_selection();
        self.set_list_selection(current.saturating_sub(amount));
        self.follow_blocks = false;
        self.follow_txs = false;
    }

    pub fn page_down(&mut self, amount: usize) {
        let len = self.list_len();
        if len == 0 {
            return;
        }
        let current = self.current_selection();
        self.set_list_selection((current + amount).min(len - 1));
    }

    pub fn current_selection(&self) -> usize {
        match self.list_kind() {
            ListKind::Blocks => self.selected_block,
            ListKind::Transactions => self.selected_tx,
            ListKind::Addresses => self.selected_address,
            ListKind::Contracts => self.selected_contract,
            ListKind::Trace => self.selected_trace,
        }
    }

    pub fn set_chord(&mut self, key: char) {
        self.pending_chord = Some(PendingChord {
            key,
            since: Instant::now(),
        });
    }

    pub fn consume_chord(&mut self, key: char) -> bool {
        let Some(chord) = self.pending_chord.as_ref() else {
            return false;
        };
        if chord.key != key {
            return false;
        }
        if chord.since.elapsed() > Duration::from_millis(800) {
            self.pending_chord = None;
            return false;
        }
        self.pending_chord = None;
        true
    }

    pub fn clear_chord(&mut self) {
        self.pending_chord = None;
    }

    fn clear_expired_chord(&mut self) {
        let Some(chord) = self.pending_chord.as_ref() else {
            return;
        };
        if chord.since.elapsed() > Duration::from_millis(800) {
            self.pending_chord = None;
        }
    }

    fn decode_all_txs(&mut self) {
        for tx in &mut self.txs {
            // First try local ABI registry
            if let Some(registry) = self.abi_registry.as_ref() {
                if let Some(function) = registry.lookup_hex(&tx.selector) {
                    tx.method = function.name.clone();
                    tx.signature = Some(function.signature.clone());
                    match decode_calldata_hex(function, &tx.input) {
                        Ok(args) => {
                            tx.decoded_args = Some(args);
                            tx.decode_error = None;
                        }
                        Err(err) => {
                            tx.decoded_args = None;
                            tx.decode_error = Some(err.to_string());
                        }
                    }
                    continue;
                }
            }

            // Fallback to signature cache (from 4byte lookups)
            if let Some((name, sig)) = self.signature_cache.get(&tx.selector) {
                tx.method = name.clone();
                tx.signature = Some(sig.clone());
            } else {
                tx.decoded_args = None;
                tx.decode_error = None;
            }
        }
    }

    fn decorate_txs_with_abi(&self, txs: &mut [TxInfo]) {
        for tx in txs {
            // First try local ABI registry
            if let Some(registry) = self.abi_registry.as_ref() {
                if let Some(function) = registry.lookup_hex(&tx.selector) {
                    tx.method = function.name.clone();
                    tx.signature = Some(function.signature.clone());
                    match decode_calldata_hex(function, &tx.input) {
                        Ok(args) => {
                            tx.decoded_args = Some(args);
                            tx.decode_error = None;
                        }
                        Err(err) => {
                            tx.decoded_args = None;
                            tx.decode_error = Some(err.to_string());
                        }
                    }
                    continue;
                }
            }

            // Fallback to signature cache (from 4byte lookups)
            if let Some((name, sig)) = self.signature_cache.get(&tx.selector) {
                tx.method = name.clone();
                tx.signature = Some(sig.clone());
            }
        }
    }

    fn decorate_trace_with_abi(&mut self) {
        for frame in &mut self.traces {
            let Some(selector) = frame.selector.as_ref() else {
                continue;
            };

            // First try local ABI registry
            if let Some(registry) = self.abi_registry.as_ref() {
                if let Some(function) = registry.lookup_hex(selector) {
                    frame.method = Some(function.name.clone());
                    frame.signature = Some(function.signature.clone());
                    if let Some(input) = frame.input.as_ref() {
                        match decode_calldata_hex(function, input) {
                            Ok(args) => {
                                frame.decoded_args = Some(args);
                                frame.decode_error = None;
                            }
                            Err(err) => {
                                frame.decoded_args = None;
                                frame.decode_error = Some(err.to_string());
                            }
                        }
                    }
                    continue;
                }
            }

            // Fallback to signature cache (from 4byte lookups)
            if let Some((name, sig)) = self.signature_cache.get(selector) {
                frame.method = Some(name.clone());
                frame.signature = Some(sig.clone());
            }
        }
    }

    fn observe_contracts_from_txs(&mut self, txs: &[TxInfo], block_number: u64) {
        if self.data_mode != DataMode::Rpc {
            return;
        }
        for tx in txs {
            if !is_address_like(&tx.to) {
                continue;
            }
            if tx
                .to
                .eq_ignore_ascii_case("0x0000000000000000000000000000000000000000")
            {
                continue;
            }
            if let Some(existing) = self
                .contracts
                .iter_mut()
                .find(|contract| contract.address.eq_ignore_ascii_case(&tx.to))
            {
                existing.tx_count = existing.tx_count.saturating_add(1);
                existing.last_call = block_number;
                continue;
            }
            let label = self.labels.get(&normalize_hex_address(&tx.to)).cloned();
            self.contracts.push(ContractInfo {
                address: tx.to.clone(),
                label,
                methods: 0,
                tx_count: 1,
                last_call: block_number,
                balance: 0.0,
                delta: 0.0,
            });
        }
    }
}

fn is_address_like(value: &str) -> bool {
    let value = value.trim();
    let Some(rest) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    else {
        return false;
    };
    rest.len() == 40 && rest.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn normalize_hex_address(value: &str) -> String {
    let trimmed = value.trim();
    let payload = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);
    format!("0x{}", payload.to_lowercase())
}

fn normalize_storage_slot(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut hex = if let Some(rest) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        rest.to_string()
    } else if trimmed.chars().all(|ch| ch.is_ascii_digit()) {
        let value = trimmed.parse::<u128>().ok()?;
        format!("{value:x}")
    } else {
        return None;
    };

    hex = hex.trim_start_matches('0').to_string();
    if hex.is_empty() {
        hex = "0".to_string();
    }
    if hex.len() > 64 {
        return None;
    }
    if !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return None;
    }

    Some(format!("0x{:0>64}", hex.to_lowercase()))
}

fn contains_case_insensitive(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(needle)
}

fn number_matches(number: u64, needle: &str) -> bool {
    if let Ok(value) = needle.parse::<u64>() {
        number == value
    } else {
        number.to_string().contains(needle)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListKind {
    Blocks,
    Transactions,
    Addresses,
    Contracts,
    Trace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TxScope<'a> {
    All,
    Block(u64),
    Address(&'a str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchTarget<'a> {
    Block(u64),
    Tx(&'a str),
    Address(&'a str),
}

fn parse_search_target(input: &str) -> Option<SearchTarget<'_>> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut parts = trimmed.split_whitespace();
    let token = parts.next()?;
    if parts.next().is_some() {
        return None;
    }

    if let Some(rest) = token
        .strip_prefix("block:")
        .or_else(|| token.strip_prefix("blk:"))
    {
        return rest.trim().parse::<u64>().ok().map(SearchTarget::Block);
    }
    if let Some(rest) = token
        .strip_prefix("tx:")
        .or_else(|| token.strip_prefix("hash:"))
    {
        let candidate = rest.trim();
        if is_tx_hash(candidate) {
            return Some(SearchTarget::Tx(candidate));
        }
        return None;
    }
    if let Some(rest) = token
        .strip_prefix("addr:")
        .or_else(|| token.strip_prefix("address:"))
    {
        let candidate = rest.trim();
        if is_address(candidate) {
            return Some(SearchTarget::Address(candidate));
        }
        return None;
    }

    if is_tx_hash(token) {
        return Some(SearchTarget::Tx(token));
    }
    if is_address(token) {
        return Some(SearchTarget::Address(token));
    }
    if let Ok(number) = token.parse::<u64>() {
        return Some(SearchTarget::Block(number));
    }
    None
}

fn is_tx_hash(value: &str) -> bool {
    let Some(rest) = value.strip_prefix("0x") else {
        return false;
    };
    rest.len() == 64 && rest.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn is_address(value: &str) -> bool {
    let Some(rest) = value.strip_prefix("0x") else {
        return false;
    };
    rest.len() == 40 && rest.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn short_hash(value: &str, len: usize) -> String {
    if value.len() <= len {
        return value.to_string();
    }
    value.chars().take(len).collect()
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

fn address_matches(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

fn seed_from_str(value: &str) -> u64 {
    value.as_bytes().iter().fold(0u64, |acc, byte| {
        acc.wrapping_mul(16777619) ^ u64::from(*byte)
    })
}

/// Decode calldata from hex string using alloy-dyn-abi
fn decode_calldata_hex(
    function: &crate::domain::abi::FunctionSignature,
    input_hex: &str,
) -> anyhow::Result<Vec<DecodedArg>> {
    use alloy_dyn_abi::{DynSolType, DynSolValue};

    // Parse hex string to bytes
    let normalized = input_hex
        .strip_prefix("0x")
        .or_else(|| input_hex.strip_prefix("0X"))
        .unwrap_or(input_hex);
    let data = hex::decode(normalized)
        .map_err(|e| anyhow::anyhow!("Invalid hex input: {}", e))?;

    if data.len() < 4 {
        anyhow::bail!("calldata too short (need at least 4 bytes for selector)");
    }

    let args_data = &data[4..];

    // Parse types from function inputs
    let types: Vec<DynSolType> = function
        .inputs
        .iter()
        .map(|param| {
            param.kind.parse::<DynSolType>()
                .map_err(|e| anyhow::anyhow!("Failed to parse type '{}': {}", param.kind, e))
        })
        .collect::<Result<Vec<_>, _>>()?;

    // Decode the arguments
    let decoded_values = if types.is_empty() {
        Vec::new()
    } else {
        let tuple_type = DynSolType::Tuple(types);
        let decoded = tuple_type
            .abi_decode(args_data)
            .map_err(|e| anyhow::anyhow!("Failed to decode calldata: {}", e))?;

        match decoded {
            DynSolValue::Tuple(values) => values,
            other => vec![other],
        }
    };

    // Build decoded arguments
    let arguments: Vec<DecodedArg> = function
        .inputs
        .iter()
        .zip(decoded_values.iter())
        .enumerate()
        .map(|(idx, (param, value))| {
            let name = if param.name.trim().is_empty() {
                format!("arg{}", idx)
            } else {
                param.name.clone()
            };

            DecodedArg {
                name,
                kind: param.kind.clone(),
                value: format_dyn_sol_value(value),
            }
        })
        .collect();

    Ok(arguments)
}

/// Format a DynSolValue for display
fn format_dyn_sol_value(value: &alloy_dyn_abi::DynSolValue) -> String {
    use alloy_dyn_abi::DynSolValue;
    match value {
        DynSolValue::Bool(b) => b.to_string(),
        DynSolValue::Int(i, _) => i.to_string(),
        DynSolValue::Uint(u, _) => {
            let s = u.to_string();
            if s.len() > 20 {
                format!("0x{:x}", u)
            } else {
                s
            }
        }
        DynSolValue::FixedBytes(word, size) => {
            let bytes = &word.as_slice()[..(*size).min(32)];
            format!("0x{}", hex::encode(bytes))
        }
        DynSolValue::Address(addr) => format!("{:?}", addr),
        DynSolValue::Function(func) => format!("0x{}", hex::encode(func.as_slice())),
        DynSolValue::Bytes(bytes) => {
            if bytes.len() <= 32 {
                format!("0x{}", hex::encode(bytes))
            } else {
                format!("0x{}… ({} bytes)", hex::encode(&bytes[..32]), bytes.len())
            }
        }
        DynSolValue::String(s) => {
            if s.len() <= 64 {
                format!("\"{}\"", s)
            } else {
                format!("\"{}…\" ({} chars)", &s[..64], s.len())
            }
        }
        DynSolValue::Array(arr) | DynSolValue::FixedArray(arr) => {
            let max_items = 10;
            let items: Vec<String> = arr
                .iter()
                .take(max_items)
                .map(format_dyn_sol_value)
                .collect();
            if arr.len() > max_items {
                format!("[{}, …] ({} items)", items.join(", "), arr.len())
            } else {
                format!("[{}]", items.join(", "))
            }
        }
        DynSolValue::Tuple(fields) => {
            let items: Vec<String> = fields.iter().map(format_dyn_sol_value).collect();
            format!("({})", items.join(", "))
        }
    }
}
