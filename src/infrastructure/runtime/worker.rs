//! Async worker - runs in Tokio runtime and handles RPC operations

use std::collections::HashSet;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

use alloy::primitives::{Address, B256, U256};
use alloy::rpc::types::{Block, TransactionRequest};
use anyhow::{Context, Result};
use tokio::time::interval;

use crate::infrastructure::abi::{AbiResolver, AbiScanner};
use crate::infrastructure::ethereum::{
    create_provider, EthereumProvider, ProviderConfig, RawBlock, RawTransaction,
};
use crate::infrastructure::runtime::bridge::{
    BlockInfo, RuntimeCommand, RuntimeEvent, TokenBalance, TokenConfig, TxInfo, TxStatus,
};

/// Run the async worker loop
pub async fn run_async_worker(
    endpoints: Vec<ProviderConfig>,
    cmd_rx: Receiver<RuntimeCommand>,
    evt_tx: Sender<RuntimeEvent>,
) -> Result<()> {
    if endpoints.is_empty() {
        anyhow::bail!("No endpoints configured");
    }

    let mut endpoint_index = 0usize;
    let mut provider: Option<Box<dyn EthereumProvider>> = None;
    let mut last_block: Option<u64> = None;
    let mut last_status_check = Instant::now() - Duration::from_secs(10);
    let mut block_subscription: Option<tokio::sync::mpsc::Receiver<Block>> = None;

    // ABI resolver for 4byte and Sourcify lookups
    let resolver = Arc::new(AbiResolver::new());

    // Track selectors we've already queued for resolution
    let mut pending_selectors: HashSet<String> = HashSet::new();

    // Polling interval for HTTP endpoints
    let mut poll_interval = interval(Duration::from_millis(500));

    loop {
        // Try to connect if not connected
        if provider.is_none() {
            let config = endpoints[endpoint_index].clone();
            match connect_to_endpoint(config.clone(), &evt_tx).await {
                Ok((p, sub)) => {
                    provider = Some(p);
                    block_subscription = sub;
                    last_block = None;

                    // Fetch initial snapshot
                    if let Some(ref p) = provider {
                        if let Ok(head) = p.block_number().await {
                            fetch_snapshot(
                                p.as_ref(),
                                head,
                                &evt_tx,
                                &resolver,
                                &mut pending_selectors,
                            )
                            .await;
                            last_block = Some(head);
                        }
                    }
                }
                Err(err) => {
                    let _ = evt_tx.send(RuntimeEvent::Error {
                        message: format!("Connection failed ({}): {:#}", config.display(), err),
                    });

                    // Try next endpoint if available
                    if endpoints.len() > 1 {
                        endpoint_index = (endpoint_index + 1) % endpoints.len();
                    }

                    tokio::time::sleep(Duration::from_millis(900)).await;
                    continue;
                }
            }
        }

        // Process commands (non-blocking)
        while let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
                RuntimeCommand::Shutdown => return Ok(()),

                RuntimeCommand::SwitchEndpoint { index } => {
                    if index >= endpoints.len() {
                        let _ = evt_tx.send(RuntimeEvent::Error {
                            message: format!(
                                "Invalid endpoint index {} ({} total)",
                                index,
                                endpoints.len()
                            ),
                        });
                        continue;
                    }
                    endpoint_index = index;
                    provider = None;
                    block_subscription = None;
                    last_block = None;
                }

                RuntimeCommand::Refresh => {
                    if let Some(ref p) = provider {
                        if let Ok(head) = p.block_number().await {
                            fetch_snapshot(
                                p.as_ref(),
                                head,
                                &evt_tx,
                                &resolver,
                                &mut pending_selectors,
                            )
                            .await;
                            last_block = Some(head);
                        }
                    }
                }

                RuntimeCommand::FetchTrace { tx_hash } => {
                    if let Some(ref p) = provider {
                        let hash = parse_b256(&tx_hash);
                        if let Some(hash) = hash {
                            match p.debug_trace_transaction(hash).await {
                                Ok(frames) => {
                                    // frames is already Vec<TraceFrame> from provider
                                    let _ = evt_tx.send(RuntimeEvent::TraceReady { tx_hash, frames });
                                }
                                Err(err) => {
                                    let _ = evt_tx.send(RuntimeEvent::Error {
                                        message: format!("Trace failed: {:#}", err),
                                    });
                                }
                            }
                        }
                    }
                }

                RuntimeCommand::FetchBalance { address } => {
                    if let Some(ref p) = provider {
                        if let Some(addr) = parse_address(&address) {
                            match p.get_balance(addr).await {
                                Ok(balance) => {
                                    let balance_eth = wei_to_eth(balance);
                                    let _ = evt_tx.send(RuntimeEvent::BalanceReady {
                                        address,
                                        balance: balance_eth,
                                    });
                                }
                                Err(err) => {
                                    let _ = evt_tx.send(RuntimeEvent::Error {
                                        message: format!("Balance fetch failed: {:#}", err),
                                    });
                                }
                            }
                        }
                    }
                }

                RuntimeCommand::FetchTokenBalances { address, tokens } => {
                    if let Some(ref p) = provider {
                        if let Some(owner) = parse_address(&address) {
                            let balances = fetch_token_balances(p.as_ref(), owner, &tokens).await;
                            let _ = evt_tx.send(RuntimeEvent::TokenBalancesReady { address, balances });
                        }
                    }
                }

                RuntimeCommand::FetchStorage { address, slot } => {
                    if let Some(ref p) = provider {
                        if let (Some(addr), Some(slot_u256)) =
                            (parse_address(&address), parse_u256(&slot))
                        {
                            match p.get_storage_at(addr, slot_u256).await {
                                Ok(value) => {
                                    let _ = evt_tx.send(RuntimeEvent::StorageReady {
                                        address,
                                        slot,
                                        value: format!("{:?}", value),
                                    });
                                }
                                Err(err) => {
                                    let _ = evt_tx.send(RuntimeEvent::Error {
                                        message: format!("Storage fetch failed: {:#}", err),
                                    });
                                }
                            }
                        }
                    }
                }

                RuntimeCommand::ReloadAbi { roots } => {
                    // Spawn ABI scanning as a blocking task
                    let evt_tx = evt_tx.clone();
                    tokio::task::spawn_blocking(move || {
                        let registry = AbiScanner::scan_roots(&roots);
                        let _ = evt_tx.send(RuntimeEvent::AbiRegistryReady { registry });
                    });
                }

                RuntimeCommand::ResolveSelector { selector } => {
                    // Resolve selector via 4byte API
                    if !pending_selectors.contains(&selector) {
                        pending_selectors.insert(selector.clone());
                        let resolver = Arc::clone(&resolver);
                        let evt_tx = evt_tx.clone();
                        let sel = selector.clone();
                        tokio::spawn(async move {
                            if let Ok(selector_bytes) = parse_selector(&sel) {
                                if let Ok(Some(sig)) = resolver.lookup_selector(selector_bytes).await
                                {
                                    let _ = evt_tx.send(RuntimeEvent::SignatureResolved {
                                        selector: sel,
                                        name: sig.name,
                                        signature: sig.signature,
                                    });
                                }
                            }
                        });
                    }
                }

                RuntimeCommand::ResolveAbi { chain_id, address } => {
                    // Resolve ABI via Sourcify
                    let resolver = Arc::clone(&resolver);
                    let evt_tx = evt_tx.clone();
                    tokio::spawn(async move {
                        if let Ok(Some(abi)) = resolver.lookup_abi(chain_id, &address).await {
                            let _ = evt_tx.send(RuntimeEvent::AbiResolved {
                                chain_id,
                                address: abi.address,
                                abi_json: abi.abi_json,
                                contract_name: abi.contract_name,
                            });
                        }
                    });
                }
            }
        }

        // Check for new blocks
        if let Some(ref p) = provider {
            // Try WebSocket subscription first
            if let Some(ref mut sub) = block_subscription {
                while let Ok(block) = sub.try_recv() {
                    let block_number = block.header.number;

                    // Fetch full block with transactions
                    if let Ok(Some(full_block)) = p.get_block(block_number).await {
                        let (block_info, txs, selectors) =
                            process_block(p.as_ref(), &full_block).await;
                        let _ = evt_tx.send(RuntimeEvent::NewBlock { block: block_info, txs });

                        // Auto-resolve any new selectors
                        for selector in selectors {
                            if !pending_selectors.contains(&selector) {
                                pending_selectors.insert(selector.clone());
                                let resolver = Arc::clone(&resolver);
                                let evt_tx = evt_tx.clone();
                                tokio::spawn(async move {
                                    if let Ok(sel_bytes) = parse_selector(&selector) {
                                        if let Ok(Some(sig)) =
                                            resolver.lookup_selector(sel_bytes).await
                                        {
                                            let _ = evt_tx.send(RuntimeEvent::SignatureResolved {
                                                selector,
                                                name: sig.name,
                                                signature: sig.signature,
                                            });
                                        }
                                    }
                                });
                            }
                        }
                        last_block = Some(block_number);
                    }
                }
            }

            // Fall back to polling for HTTP endpoints
            if !p.supports_subscriptions() {
                poll_interval.tick().await;

                match p.block_number().await {
                    Ok(head) => {
                        if let Some(last) = last_block {
                            if head > last {
                                // Fetch missing blocks
                                for number in (last + 1)..=head {
                                    if let Ok(Some(block)) = p.get_block(number).await {
                                        let (block_info, txs, selectors) =
                                            process_block(p.as_ref(), &block).await;
                                        let _ = evt_tx.send(RuntimeEvent::NewBlock {
                                            block: block_info,
                                            txs,
                                        });

                                        // Auto-resolve any new selectors
                                        for selector in selectors {
                                            if !pending_selectors.contains(&selector) {
                                                pending_selectors.insert(selector.clone());
                                                let resolver = Arc::clone(&resolver);
                                                let evt_tx = evt_tx.clone();
                                                tokio::spawn(async move {
                                                    if let Ok(sel_bytes) = parse_selector(&selector)
                                                    {
                                                        if let Ok(Some(sig)) =
                                                            resolver.lookup_selector(sel_bytes).await
                                                        {
                                                            let _ = evt_tx.send(
                                                                RuntimeEvent::SignatureResolved {
                                                                    selector,
                                                                    name: sig.name,
                                                                    signature: sig.signature,
                                                                },
                                                            );
                                                        }
                                                    }
                                                });
                                            }
                                        }
                                    }
                                }
                                last_block = Some(head);
                            }
                        } else {
                            last_block = Some(head);
                        }
                    }
                    Err(err) => {
                        let _ = evt_tx.send(RuntimeEvent::Error {
                            message: format!("RPC error: {:#}", err),
                        });
                        provider = None;
                        block_subscription = None;

                        // Try next endpoint
                        if endpoints.len() > 1 {
                            endpoint_index = (endpoint_index + 1) % endpoints.len();
                        }
                        continue;
                    }
                }
            }

            // Periodic status update
            if last_status_check.elapsed() >= Duration::from_secs(2) {
                if let Ok(block_number) = p.block_number().await {
                    let peer_count = p.peer_count().await.ok().map(|c| c as u32);
                    let sync_progress = p.sync_progress().await.ok().flatten();

                    let _ = evt_tx.send(RuntimeEvent::Status {
                        rtt_ms: Some(last_status_check.elapsed().as_millis() as u64),
                        peer_count,
                        sync_progress,
                    });
                    let _ = block_number; // suppress unused warning
                }
                last_status_check = Instant::now();
            }
        }

        // Small yield to prevent busy loop
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// Connect to an endpoint and return the provider and optional block subscription
async fn connect_to_endpoint(
    config: ProviderConfig,
    evt_tx: &Sender<RuntimeEvent>,
) -> Result<(Box<dyn EthereumProvider>, Option<tokio::sync::mpsc::Receiver<Block>>)> {
    let provider = create_provider(config.clone()).await?;

    // Get node info
    let client_version = provider
        .client_version()
        .await
        .context("Failed to get client version")?;
    let node_kind = detect_node_kind(&client_version);
    let accounts = provider.accounts().await.unwrap_or_default();
    let supports_subscriptions = provider.supports_subscriptions();

    // Set up block subscription if supported
    let subscription = if supports_subscriptions {
        Some(provider.subscribe_blocks().await?)
    } else {
        None
    };

    // Convert accounts to strings
    let accounts: Vec<String> = accounts.iter().map(|a| format!("{:?}", a)).collect();

    let _ = evt_tx.send(RuntimeEvent::Connected {
        endpoint: provider.endpoint_name(),
        node_kind,
        accounts,
    });

    Ok((provider, subscription))
}

/// Detect node kind from client version string
fn detect_node_kind(version: &str) -> String {
    let lower = version.to_lowercase();
    if lower.contains("anvil") {
        "anvil".to_string()
    } else if lower.contains("reth") {
        "reth".to_string()
    } else if lower.contains("geth") || lower.contains("go-ethereum") {
        "geth".to_string()
    } else {
        version.to_string()
    }
}

/// Fetch initial snapshot of recent blocks
async fn fetch_snapshot(
    provider: &dyn EthereumProvider,
    head: u64,
    evt_tx: &Sender<RuntimeEvent>,
    resolver: &Arc<AbiResolver>,
    pending_selectors: &mut HashSet<String>,
) {
    let start = head.saturating_sub(10);
    for number in start..=head {
        if let Ok(Some(block)) = provider.get_block(number).await {
            let (block_info, txs, selectors) = process_block(provider, &block).await;
            let _ = evt_tx.send(RuntimeEvent::NewBlock { block: block_info, txs });

            // Auto-resolve any new selectors
            for selector in selectors {
                if !pending_selectors.contains(&selector) {
                    pending_selectors.insert(selector.clone());
                    let resolver = Arc::clone(resolver);
                    let evt_tx = evt_tx.clone();
                    tokio::spawn(async move {
                        if let Ok(sel_bytes) = parse_selector(&selector) {
                            if let Ok(Some(sig)) = resolver.lookup_selector(sel_bytes).await {
                                let _ = evt_tx.send(RuntimeEvent::SignatureResolved {
                                    selector,
                                    name: sig.name,
                                    signature: sig.signature,
                                });
                            }
                        }
                    });
                }
            }
        }
    }
}

/// Process a block and fetch transaction receipts
/// Returns (BlockInfo, Vec<TxInfo>, Vec<selectors_to_resolve>)
async fn process_block(
    provider: &dyn EthereumProvider,
    block: &RawBlock,
) -> (BlockInfo, Vec<TxInfo>, Vec<String>) {
    let block_number = block.number;
    let tx_count = block.transactions.len();

    let block_info = BlockInfo {
        number: block_number,
        tx_count: tx_count as u32,
        gas_used: block.gas_used,
        base_fee: block.base_fee_per_gas.map(|f| f / 1_000_000_000).unwrap_or(0),
        miner: block.miner.clone(),
    };

    let mut txs = Vec::new();
    let mut selectors = HashSet::new();

    for (i, raw_tx) in block.transactions.iter().enumerate() {
        // Parse tx hash for receipt lookup
        let tx_hash = parse_b256(&raw_tx.hash);

        // Only fetch receipts for first 20 txs to avoid overload
        let receipt = if i < 20 {
            if let Some(hash) = tx_hash {
                provider.get_receipt(hash).await.ok().flatten()
            } else {
                None
            }
        } else {
            None
        };

        let tx_info = convert_raw_tx(raw_tx, receipt.as_ref(), block_number);

        // Collect selector for resolution if it's a contract call
        if tx_info.input.len() > 10 {
            // 0x + at least 4 bytes
            selectors.insert(tx_info.selector.clone());
        }

        txs.push(tx_info);
    }

    (block_info, txs, selectors.into_iter().collect())
}

/// Convert a raw transaction to TxInfo
fn convert_raw_tx(
    tx: &RawTransaction,
    receipt: Option<&alloy::rpc::types::TransactionReceipt>,
    block_number: u64,
) -> TxInfo {
    let input = &tx.input;
    let selector = if input.len() >= 4 {
        format!("0x{}", hex::encode(&input[..4]))
    } else {
        "0x".to_string()
    };

    let method = if input.len() >= 4 {
        selector.clone()
    } else {
        "(transfer)".to_string()
    };

    let status = receipt
        .map(|r| {
            if r.status() {
                TxStatus::Success
            } else {
                TxStatus::Revert
            }
        })
        .unwrap_or(TxStatus::Unknown);

    let value = wei_to_eth(tx.value);

    let to_addr = tx
        .to
        .clone()
        .unwrap_or_else(|| "CREATE".to_string());

    TxInfo {
        hash: tx.hash.clone(),
        from: tx.from.clone(),
        to: to_addr,
        value,
        gas_used: receipt.map(|r| r.gas_used).unwrap_or(21000),
        status,
        input: format!("0x{}", hex::encode(input)),
        selector,
        method,
        signature: None,
        decoded_args: None,
        decode_error: None,
        block_number,
    }
}

/// Fetch token balances for an address
async fn fetch_token_balances(
    provider: &dyn EthereumProvider,
    owner: Address,
    tokens: &[TokenConfig],
) -> Vec<TokenBalance> {
    let mut balances = Vec::new();

    for token in tokens {
        let Some(token_addr) = parse_address(&token.address) else {
            continue;
        };

        // balanceOf(address) selector: 0x70a08231
        let calldata = encode_balance_of(owner);

        let request = TransactionRequest::default()
            .to(token_addr)
            .input(calldata.into());

        let balance_str = match provider.call(request).await {
            Ok(data) => {
                if data.len() >= 32 {
                    let value = U256::from_be_slice(&data[..32]);
                    format_token_balance(value, token.decimals)
                } else {
                    "(decode err)".to_string()
                }
            }
            Err(_) => "(rpc err)".to_string(),
        };

        // Get symbol if not provided
        let symbol = token.symbol.clone().unwrap_or_else(|| {
            format!("0x{}…", &token.address[2..10])
        });

        balances.push(TokenBalance {
            token: token.address.clone(),
            symbol,
            decimals: token.decimals,
            balance: balance_str,
        });
    }

    balances
}

/// Encode balanceOf(address) call
fn encode_balance_of(owner: Address) -> Vec<u8> {
    // balanceOf(address): 0x70a08231
    let mut data = vec![0x70, 0xa0, 0x82, 0x31];
    // Pad address to 32 bytes
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(owner.as_slice());
    data
}

/// Format token balance with decimals
fn format_token_balance(value: U256, decimals: Option<u8>) -> String {
    let Some(decimals) = decimals else {
        return value.to_string();
    };

    if decimals == 0 {
        return value.to_string();
    }

    let divisor = U256::from(10u64).pow(U256::from(decimals));
    let whole = value / divisor;
    let frac = value % divisor;

    if frac.is_zero() {
        whole.to_string()
    } else {
        let frac_str = format!("{:0>width$}", frac, width = decimals as usize);
        let trimmed = frac_str.trim_end_matches('0');
        if trimmed.is_empty() {
            whole.to_string()
        } else {
            format!("{}.{}", whole, trimmed)
        }
    }
}

/// Convert Wei to ETH as f64
fn wei_to_eth(wei: U256) -> f64 {
    let eth_in_wei = U256::from(1_000_000_000_000_000_000u64);
    let whole = wei / eth_in_wei;
    let frac = wei % eth_in_wei;

    let whole_f64: f64 = whole.to_string().parse().unwrap_or(0.0);
    let frac_f64: f64 = frac.to_string().parse().unwrap_or(0.0);

    whole_f64 + frac_f64 / 1e18
}

/// Parse a hex address string to Address
fn parse_address(s: &str) -> Option<Address> {
    let normalized = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")).unwrap_or(s);
    if normalized.len() != 40 {
        return None;
    }
    let bytes = hex::decode(normalized).ok()?;
    Some(Address::from_slice(&bytes))
}

/// Parse a hex hash string to B256
fn parse_b256(s: &str) -> Option<B256> {
    let normalized = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")).unwrap_or(s);
    if normalized.len() != 64 {
        return None;
    }
    let bytes = hex::decode(normalized).ok()?;
    Some(B256::from_slice(&bytes))
}

/// Parse a hex or decimal string to U256
fn parse_u256(s: &str) -> Option<U256> {
    let trimmed = s.trim();
    if let Some(hex_str) = trimmed.strip_prefix("0x").or_else(|| trimmed.strip_prefix("0X")) {
        let bytes = hex::decode(hex_str).ok()?;
        Some(U256::from_be_slice(&bytes))
    } else {
        trimmed.parse::<u128>().ok().map(U256::from)
    }
}

/// Parse a hex selector string to 4-byte array
fn parse_selector(s: &str) -> Result<[u8; 4]> {
    let normalized = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")).unwrap_or(s);
    if normalized.len() != 8 {
        anyhow::bail!("Invalid selector length");
    }
    let bytes = hex::decode(normalized)?;
    let mut arr = [0u8; 4];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}
