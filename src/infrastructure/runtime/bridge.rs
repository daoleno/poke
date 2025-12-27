//! Runtime bridge - connects sync TUI thread with async Tokio runtime
#![allow(dead_code)]
//!
//! This module provides a bridge between the synchronous TUI (ratatui) thread
//! and the asynchronous Tokio runtime that handles RPC operations.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use tokio::runtime::Runtime;

use crate::domain::abi::AbiRegistry;
use crate::infrastructure::ethereum::ProviderConfig;
use crate::infrastructure::runtime::worker::run_async_worker;

/// Commands sent from the TUI to the async worker
#[derive(Debug, Clone)]
pub enum RuntimeCommand {
    /// Switch to a different endpoint
    SwitchEndpoint { index: usize },
    /// Fetch transaction trace
    FetchTrace { tx_hash: String },
    /// Fetch account balance
    FetchBalance { address: String },
    /// Fetch token balances
    FetchTokenBalances {
        address: String,
        tokens: Vec<TokenConfig>,
    },
    /// Fetch storage slot
    FetchStorage { address: String, slot: String },
    /// Resolve function selector via 4byte API
    ResolveSelector { selector: String },
    /// Resolve contract ABI via Sourcify
    ResolveAbi { chain_id: u64, address: String },
    /// Force refresh (re-fetch current block)
    Refresh,
    /// Reload ABI from specified roots
    ReloadAbi { roots: Vec<PathBuf> },
    /// Shutdown the worker
    Shutdown,
}

/// Token configuration for balance fetching
#[derive(Debug, Clone)]
pub struct TokenConfig {
    pub address: String,
    pub symbol: Option<String>,
    pub decimals: Option<u8>,
}

/// Events sent from the async worker to the TUI
#[derive(Debug, Clone)]
pub enum RuntimeEvent {
    /// Successfully connected to a node
    Connected {
        endpoint: String,
        node_kind: String,
        accounts: Vec<String>,
    },
    /// Node status update
    Status {
        rtt_ms: Option<u64>,
        peer_count: Option<u32>,
        sync_progress: Option<f64>,
    },
    /// New block received
    NewBlock {
        block: BlockInfo,
        txs: Vec<TxInfo>,
    },
    /// Trace ready
    TraceReady {
        tx_hash: String,
        frames: Vec<TraceFrame>,
    },
    /// Balance ready
    BalanceReady { address: String, balance: f64 },
    /// Token balances ready
    TokenBalancesReady {
        address: String,
        balances: Vec<TokenBalance>,
    },
    /// Storage value ready
    StorageReady {
        address: String,
        slot: String,
        value: String,
    },
    /// ABI registry updated
    AbiRegistryReady { registry: AbiRegistry },
    /// Function signature resolved from 4byte
    SignatureResolved {
        selector: String,
        name: String,
        signature: String,
    },
    /// Contract ABI resolved from Sourcify
    AbiResolved {
        chain_id: u64,
        address: String,
        abi_json: String,
        contract_name: Option<String>,
    },
    /// Error occurred
    Error { message: String },
}

/// Block information compatible with app.rs
#[derive(Debug, Clone)]
pub struct BlockInfo {
    pub number: u64,
    pub tx_count: u32,
    pub gas_used: u64,
    pub base_fee: u64,
    pub miner: String,
}

/// Transaction status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxStatus {
    Success,
    Revert,
    Unknown,
}

/// Transaction information compatible with app.rs
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

/// Decoded argument
#[derive(Debug, Clone)]
pub struct DecodedArg {
    pub name: String,
    pub kind: String,
    pub value: String,
}

/// Call trace status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallStatus {
    Ok,
    Revert,
}

/// Trace frame compatible with app.rs
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

/// Token balance result
#[derive(Debug, Clone)]
pub struct TokenBalance {
    pub token: String,
    pub symbol: String,
    pub decimals: Option<u8>,
    pub balance: String,
}

/// Bridge between sync TUI thread and async Tokio runtime
pub struct RuntimeBridge {
    cmd_tx: Sender<RuntimeCommand>,
    evt_rx: Receiver<RuntimeEvent>,
}

impl RuntimeBridge {
    /// Create a new runtime bridge with the given endpoint configurations
    pub fn new(endpoints: Vec<ProviderConfig>) -> anyhow::Result<Self> {
        let (cmd_tx, cmd_rx) = mpsc::channel::<RuntimeCommand>();
        let (evt_tx, evt_rx) = mpsc::channel::<RuntimeEvent>();

        // Spawn the worker thread with its own Tokio runtime
        thread::spawn(move || {
            let rt = Runtime::new().expect("Failed to create Tokio runtime");
            rt.block_on(async {
                if let Err(err) = run_async_worker(endpoints, cmd_rx, evt_tx.clone()).await {
                    let _ = evt_tx.send(RuntimeEvent::Error {
                        message: format!("Worker exited: {:#}", err),
                    });
                }
            });
        });

        Ok(Self { cmd_tx, evt_rx })
    }

    /// Send a command to the async worker
    pub fn send(&self, cmd: RuntimeCommand) -> anyhow::Result<()> {
        self.cmd_tx
            .send(cmd)
            .map_err(|_| anyhow::anyhow!("Worker channel closed"))
    }

    /// Poll for events (non-blocking)
    pub fn poll_events(&self) -> Vec<RuntimeEvent> {
        let mut events = Vec::new();
        while let Ok(evt) = self.evt_rx.try_recv() {
            events.push(evt);
        }
        events
    }

    /// Try to receive a single event (non-blocking)
    pub fn try_recv(&self) -> Option<RuntimeEvent> {
        self.evt_rx.try_recv().ok()
    }
}

impl Drop for RuntimeBridge {
    fn drop(&mut self) {
        // Try to send shutdown command
        let _ = self.cmd_tx.send(RuntimeCommand::Shutdown);
    }
}
