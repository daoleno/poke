//! Ethereum provider abstraction and Alloy implementations
//!
//! Uses raw JSON requests for block fetching to support all EVM chains
//! including L2s like Optimism/Base that have non-standard transaction types.

use std::path::PathBuf;

use alloy::network::Ethereum;
use alloy::primitives::{Address, Bytes, B256, U256};
use alloy::providers::{
    fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller},
    Identity, Provider, ProviderBuilder, RootProvider,
};
use alloy::rpc::types::trace::geth::{GethDebugTracingOptions, GethTrace};
use alloy::rpc::types::{Block, TransactionReceipt, TransactionRequest};
use anyhow::{Context, Result};
use futures::StreamExt;
use tokio::sync::mpsc;

use crate::infrastructure::ethereum::types::convert_trace_frames;
use crate::infrastructure::runtime::TraceFrame;

/// Raw block data parsed from JSON - works with any EVM chain
#[derive(Debug, Clone)]
pub struct RawBlock {
    pub number: u64,
    pub hash: String,
    pub parent_hash: String,
    pub timestamp: u64,
    pub gas_used: u64,
    pub gas_limit: u64,
    pub base_fee_per_gas: Option<u64>,
    pub miner: String,
    pub transactions: Vec<RawTransaction>,
}

/// Raw transaction data parsed from JSON - chain agnostic
#[derive(Debug, Clone)]
pub struct RawTransaction {
    pub hash: String,
    pub from: String,
    pub to: Option<String>,
    pub value: U256,
    pub input: Bytes,
    pub gas: u64,
    pub nonce: u64,
    pub tx_type: Option<u8>,
}

/// Provider configuration
#[derive(Debug, Clone)]
pub enum ProviderConfig {
    /// HTTP JSON-RPC endpoint
    Http(String),
    /// WebSocket endpoint
    WebSocket(String),
    /// IPC socket path (Unix only)
    #[cfg(unix)]
    Ipc(PathBuf),
}

impl ProviderConfig {
    /// Get display name for this endpoint
    pub fn display(&self) -> String {
        match self {
            ProviderConfig::Http(url) => url.clone(),
            ProviderConfig::WebSocket(url) => url.clone(),
            #[cfg(unix)]
            ProviderConfig::Ipc(path) => path.display().to_string(),
        }
    }

    /// Check if this is a WebSocket endpoint
    pub fn is_websocket(&self) -> bool {
        matches!(self, ProviderConfig::WebSocket(_))
    }
}

/// Abstract Ethereum provider trait
///
/// This trait defines all the operations we need for the TUI,
/// abstracting over the specific Alloy transport.
#[async_trait::async_trait]
pub trait EthereumProvider: Send + Sync + 'static {
    /// Get the current block number
    async fn block_number(&self) -> Result<u64>;

    /// Get client version (for node detection)
    async fn client_version(&self) -> Result<String>;

    /// Get available accounts (for Anvil/dev nodes)
    async fn accounts(&self) -> Result<Vec<Address>>;

    /// Get peer count
    async fn peer_count(&self) -> Result<u64>;

    /// Get sync status (returns None if not syncing, Some(progress) if syncing)
    async fn sync_progress(&self) -> Result<Option<f64>>;

    /// Get a block by number with full transactions (chain-agnostic raw format)
    async fn get_block(&self, number: u64) -> Result<Option<RawBlock>>;

    /// Get transaction receipt
    async fn get_receipt(&self, hash: B256) -> Result<Option<TransactionReceipt>>;

    /// Get account balance
    async fn get_balance(&self, address: Address) -> Result<U256>;

    /// Execute a call (eth_call)
    async fn call(&self, request: TransactionRequest) -> Result<Bytes>;

    /// Get storage at a specific slot
    async fn get_storage_at(&self, address: Address, slot: U256) -> Result<B256>;

    /// Debug trace transaction (for trace view)
    async fn debug_trace_transaction(&self, hash: B256) -> Result<Vec<TraceFrame>>;

    /// Subscribe to new blocks (for WebSocket)
    async fn subscribe_blocks(&self) -> Result<mpsc::Receiver<Block>>;

    /// Check if subscriptions are supported
    fn supports_subscriptions(&self) -> bool;

    /// Get endpoint display name
    fn endpoint_name(&self) -> String;
}

// Type aliases for the filled providers
type HttpFillProvider = FillProvider<
    JoinFill<
        Identity,
        JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
    >,
    RootProvider,
    Ethereum,
>;

type WsFillProvider = FillProvider<
    JoinFill<
        Identity,
        JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
    >,
    RootProvider,
    Ethereum,
>;

#[cfg(unix)]
type IpcFillProvider = FillProvider<
    JoinFill<
        Identity,
        JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
    >,
    RootProvider,
    Ethereum,
>;

/// Enum-based provider that stores concrete types for each transport
/// This allows calling extension traits like DebugApi
pub enum AlloyProvider {
    Http {
        provider: HttpFillProvider,
        endpoint: String,
    },
    WebSocket {
        provider: WsFillProvider,
        endpoint: String,
    },
    #[cfg(unix)]
    Ipc {
        provider: IpcFillProvider,
        endpoint: String,
    },
}

/// Create a provider from configuration
pub async fn create_provider(config: ProviderConfig) -> Result<Box<dyn EthereumProvider>> {
    match config {
        ProviderConfig::Http(url) => {
            let rpc_url = url.parse().context("Invalid HTTP URL")?;
            let provider = ProviderBuilder::new().connect_http(rpc_url);
            Ok(Box::new(AlloyProvider::Http {
                provider,
                endpoint: url,
            }))
        }
        ProviderConfig::WebSocket(url) => {
            let provider = ProviderBuilder::new()
                .connect(&url)
                .await
                .context("Failed to create WebSocket provider")?;
            Ok(Box::new(AlloyProvider::WebSocket {
                provider,
                endpoint: url,
            }))
        }
        #[cfg(unix)]
        ProviderConfig::Ipc(path) => {
            use alloy::providers::IpcConnect;
            let ipc_path = path.to_string_lossy().to_string();
            let ipc = IpcConnect::new(ipc_path);
            let provider = ProviderBuilder::new()
                .connect_ipc(ipc)
                .await
                .context("Failed to create IPC provider")?;
            let display = path.display().to_string();
            Ok(Box::new(AlloyProvider::Ipc {
                provider,
                endpoint: display,
            }))
        }
    }
}

// Macro to reduce code duplication for provider method implementations
macro_rules! impl_provider_method {
    ($self:ident, $method:ident $(, $arg:expr)*) => {
        match $self {
            AlloyProvider::Http { provider, .. } => provider.$method($($arg),*).await,
            AlloyProvider::WebSocket { provider, .. } => provider.$method($($arg),*).await,
            #[cfg(unix)]
            AlloyProvider::Ipc { provider, .. } => provider.$method($($arg),*).await,
        }
    };
}

#[async_trait::async_trait]
impl EthereumProvider for AlloyProvider {
    async fn block_number(&self) -> Result<u64> {
        Ok(impl_provider_method!(self, get_block_number)?)
    }

    async fn client_version(&self) -> Result<String> {
        Ok(impl_provider_method!(self, get_client_version)?)
    }

    async fn accounts(&self) -> Result<Vec<Address>> {
        Ok(impl_provider_method!(self, get_accounts)?)
    }

    async fn peer_count(&self) -> Result<u64> {
        // Call net_peerCount RPC method - returns hex string like "0x19"
        let result: std::result::Result<U256, _> = match self {
            AlloyProvider::Http { provider, .. } => {
                provider.raw_request("net_peerCount".into(), ()).await
            }
            AlloyProvider::WebSocket { provider, .. } => {
                provider.raw_request("net_peerCount".into(), ()).await
            }
            #[cfg(unix)]
            AlloyProvider::Ipc { provider, .. } => {
                provider.raw_request("net_peerCount".into(), ()).await
            }
        };

        // Fall back to 0 if the RPC call fails (some nodes don't support net_peerCount)
        Ok(result.map(|v| v.to::<u64>()).unwrap_or(0))
    }

    async fn sync_progress(&self) -> Result<Option<f64>> {
        let syncing = impl_provider_method!(self, syncing)?;
        match syncing {
            alloy::rpc::types::SyncStatus::None => Ok(None),
            alloy::rpc::types::SyncStatus::Info(info) => {
                let current = info.current_block.to::<u64>();
                let highest = info.highest_block.to::<u64>();
                if highest == 0 {
                    Ok(Some(0.0))
                } else {
                    Ok(Some((current as f64 / highest as f64).clamp(0.0, 1.0)))
                }
            }
        }
    }

    async fn get_block(&self, number: u64) -> Result<Option<RawBlock>> {
        // Use raw_request to support all EVM chains including L2s (Optimism/Base)
        let block_num_hex = format!("0x{:x}", number);
        let json: serde_json::Value = match self {
            AlloyProvider::Http { provider, .. } => {
                provider
                    .raw_request("eth_getBlockByNumber".into(), (&block_num_hex, true))
                    .await?
            }
            AlloyProvider::WebSocket { provider, .. } => {
                provider
                    .raw_request("eth_getBlockByNumber".into(), (&block_num_hex, true))
                    .await?
            }
            #[cfg(unix)]
            AlloyProvider::Ipc { provider, .. } => {
                provider
                    .raw_request("eth_getBlockByNumber".into(), (&block_num_hex, true))
                    .await?
            }
        };

        if json.is_null() {
            return Ok(None);
        }

        Ok(Some(parse_raw_block(&json)?))
    }

    async fn get_receipt(&self, hash: B256) -> Result<Option<TransactionReceipt>> {
        Ok(impl_provider_method!(self, get_transaction_receipt, hash)?)
    }

    async fn get_balance(&self, address: Address) -> Result<U256> {
        Ok(impl_provider_method!(self, get_balance, address)?)
    }

    async fn call(&self, request: TransactionRequest) -> Result<Bytes> {
        match self {
            AlloyProvider::Http { provider, .. } => Ok(provider.call(request.clone()).await?),
            AlloyProvider::WebSocket { provider, .. } => Ok(provider.call(request.clone()).await?),
            #[cfg(unix)]
            AlloyProvider::Ipc { provider, .. } => Ok(provider.call(request).await?),
        }
    }

    async fn get_storage_at(&self, address: Address, slot: U256) -> Result<B256> {
        let value = impl_provider_method!(self, get_storage_at, address, slot)?;
        Ok(B256::from(value))
    }

    async fn debug_trace_transaction(&self, hash: B256) -> Result<Vec<TraceFrame>> {
        use alloy::rpc::types::trace::geth::{
            GethDebugBuiltInTracerType, GethDebugTracerType,
        };

        let opts = GethDebugTracingOptions {
            tracer: Some(GethDebugTracerType::BuiltInTracer(
                GethDebugBuiltInTracerType::CallTracer,
            )),
            ..Default::default()
        };

        // Use raw_request to call debug_traceTransaction RPC method
        let trace: GethTrace = match self {
            AlloyProvider::Http { provider, .. } => {
                provider
                    .raw_request("debug_traceTransaction".into(), (hash, &opts))
                    .await?
            }
            AlloyProvider::WebSocket { provider, .. } => {
                provider
                    .raw_request("debug_traceTransaction".into(), (hash, &opts))
                    .await?
            }
            #[cfg(unix)]
            AlloyProvider::Ipc { provider, .. } => {
                provider
                    .raw_request("debug_traceTransaction".into(), (hash, &opts))
                    .await?
            }
        };

        Ok(convert_trace_frames(trace))
    }

    async fn subscribe_blocks(&self) -> Result<mpsc::Receiver<Block>> {
        match self {
            AlloyProvider::Http { .. } => {
                // HTTP doesn't support subscriptions, return immediately closed channel
                let (_, rx) = mpsc::channel(1);
                Ok(rx)
            }
            AlloyProvider::WebSocket { provider, .. } => {
                let sub = provider.subscribe_blocks().await?;
                let (tx, rx) = mpsc::channel(100);

                tokio::spawn(async move {
                    let mut stream = sub.into_stream();
                    while let Some(block) = stream.next().await {
                        let full_block = Block {
                            header: block,
                            transactions: Default::default(),
                            uncles: Default::default(),
                            withdrawals: None,
                        };
                        if tx.send(full_block).await.is_err() {
                            break;
                        }
                    }
                });

                Ok(rx)
            }
            #[cfg(unix)]
            AlloyProvider::Ipc { provider, .. } => {
                let sub = provider.subscribe_blocks().await?;
                let (tx, rx) = mpsc::channel(100);

                tokio::spawn(async move {
                    let mut stream = sub.into_stream();
                    while let Some(block) = stream.next().await {
                        let full_block = Block {
                            header: block,
                            transactions: Default::default(),
                            uncles: Default::default(),
                            withdrawals: None,
                        };
                        if tx.send(full_block).await.is_err() {
                            break;
                        }
                    }
                });

                Ok(rx)
            }
        }
    }

    fn supports_subscriptions(&self) -> bool {
        match self {
            AlloyProvider::Http { .. } => false,
            AlloyProvider::WebSocket { .. } => true,
            #[cfg(unix)]
            AlloyProvider::Ipc { .. } => true,
        }
    }

    fn endpoint_name(&self) -> String {
        match self {
            AlloyProvider::Http { endpoint, .. } => endpoint.clone(),
            AlloyProvider::WebSocket { endpoint, .. } => endpoint.clone(),
            #[cfg(unix)]
            AlloyProvider::Ipc { endpoint, .. } => endpoint.clone(),
        }
    }
}

/// Parse raw JSON block response to our chain-agnostic RawBlock type
fn parse_raw_block(json: &serde_json::Value) -> Result<RawBlock> {
    let number = parse_hex_u64(json.get("number").and_then(|v| v.as_str()).unwrap_or("0x0"))?;
    let hash = json
        .get("hash")
        .and_then(|v| v.as_str())
        .unwrap_or("0x0")
        .to_string();
    let parent_hash = json
        .get("parentHash")
        .and_then(|v| v.as_str())
        .unwrap_or("0x0")
        .to_string();
    let timestamp = parse_hex_u64(json.get("timestamp").and_then(|v| v.as_str()).unwrap_or("0x0"))?;
    let gas_used = parse_hex_u64(json.get("gasUsed").and_then(|v| v.as_str()).unwrap_or("0x0"))?;
    let gas_limit = parse_hex_u64(json.get("gasLimit").and_then(|v| v.as_str()).unwrap_or("0x0"))?;
    let base_fee_per_gas = json
        .get("baseFeePerGas")
        .and_then(|v| v.as_str())
        .map(|s| parse_hex_u64(s).unwrap_or(0));
    let miner = json
        .get("miner")
        .and_then(|v| v.as_str())
        .unwrap_or("0x0000000000000000000000000000000000000000")
        .to_string();

    let mut transactions = Vec::new();
    if let Some(txs) = json.get("transactions").and_then(|v| v.as_array()) {
        for tx_json in txs {
            if let Some(tx) = parse_raw_transaction(tx_json) {
                transactions.push(tx);
            }
        }
    }

    Ok(RawBlock {
        number,
        hash,
        parent_hash,
        timestamp,
        gas_used,
        gas_limit,
        base_fee_per_gas,
        miner,
        transactions,
    })
}

/// Parse a single transaction from JSON
fn parse_raw_transaction(json: &serde_json::Value) -> Option<RawTransaction> {
    let hash = json.get("hash")?.as_str()?.to_string();
    let from = json.get("from")?.as_str()?.to_string();
    let to = json.get("to").and_then(|v| v.as_str()).map(|s| s.to_string());

    let value_str = json.get("value").and_then(|v| v.as_str()).unwrap_or("0x0");
    let value = parse_hex_u256(value_str).unwrap_or(U256::ZERO);

    let input_str = json.get("input").and_then(|v| v.as_str()).unwrap_or("0x");
    let input_bytes = hex::decode(input_str.strip_prefix("0x").unwrap_or(input_str)).unwrap_or_default();
    let input = Bytes::from(input_bytes);

    let gas = parse_hex_u64(json.get("gas").and_then(|v| v.as_str()).unwrap_or("0x0")).unwrap_or(0);
    let nonce = parse_hex_u64(json.get("nonce").and_then(|v| v.as_str()).unwrap_or("0x0")).unwrap_or(0);

    let tx_type = json
        .get("type")
        .and_then(|v| v.as_str())
        .and_then(|s| parse_hex_u64(s).ok())
        .map(|n| n as u8);

    Some(RawTransaction {
        hash,
        from,
        to,
        value,
        input,
        gas,
        nonce,
        tx_type,
    })
}

/// Parse hex string to u64
fn parse_hex_u64(s: &str) -> Result<u64> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    u64::from_str_radix(s, 16).context("Failed to parse hex u64")
}

/// Parse hex string to U256
fn parse_hex_u256(s: &str) -> Result<U256> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    if s.is_empty() || s == "0" {
        return Ok(U256::ZERO);
    }
    // Pad to 64 chars for proper parsing
    let padded = format!("{:0>64}", s);
    let bytes = hex::decode(&padded).context("Failed to decode hex")?;
    Ok(U256::from_be_slice(&bytes))
}
