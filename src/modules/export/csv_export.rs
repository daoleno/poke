//! CSV Export
//!
//! Writes blocks, transactions, and addresses to CSV files.

use crate::app::{AddressInfo, BlockInfo, TxInfo, TxStatus};
use std::path::Path;

/// Write blocks to CSV file
pub fn write_blocks(path: &Path, blocks: &[BlockInfo]) -> Result<usize, Box<dyn std::error::Error>> {
    let mut wtr = csv::Writer::from_path(path)?;

    // Write header
    wtr.write_record(["number", "tx_count", "gas_used", "base_fee", "miner"])?;

    // Write data rows
    for block in blocks {
        wtr.write_record([
            block.number.to_string(),
            block.tx_count.to_string(),
            block.gas_used.to_string(),
            block.base_fee.to_string(),
            block.miner.clone(),
        ])?;
    }

    wtr.flush()?;
    Ok(blocks.len())
}

/// Write transactions to CSV file
pub fn write_transactions(path: &Path, txs: &[TxInfo]) -> Result<usize, Box<dyn std::error::Error>> {
    let mut wtr = csv::Writer::from_path(path)?;

    // Write header
    wtr.write_record([
        "hash",
        "block_number",
        "from",
        "to",
        "value",
        "gas_used",
        "status",
        "method",
        "selector",
    ])?;

    // Write data rows
    for tx in txs {
        let status = match tx.status {
            TxStatus::Success => "success",
            TxStatus::Revert => "revert",
            TxStatus::Unknown => "unknown",
        };

        wtr.write_record([
            tx.hash.clone(),
            tx.block_number.to_string(),
            tx.from.clone(),
            tx.to.clone(),
            tx.value.to_string(),
            tx.gas_used.to_string(),
            status.to_string(),
            tx.method.clone(),
            tx.selector.clone(),
        ])?;
    }

    wtr.flush()?;
    Ok(txs.len())
}

/// Write addresses to CSV file
pub fn write_addresses(path: &Path, addresses: &[AddressInfo]) -> Result<usize, Box<dyn std::error::Error>> {
    let mut wtr = csv::Writer::from_path(path)?;

    // Write header
    wtr.write_record(["address", "label", "balance", "delta", "kind"])?;

    // Write data rows
    for addr in addresses {
        wtr.write_record([
            addr.address.clone(),
            addr.label.clone().unwrap_or_default(),
            addr.balance.to_string(),
            addr.delta.to_string(),
            format!("{:?}", addr.kind),
        ])?;
    }

    wtr.flush()?;
    Ok(addresses.len())
}
