//! Storage slot calculation for Solidity

use alloy::primitives::{keccak256, Address, U256};
use super::ToolResult;
use crate::core::{Action, NotifyLevel};

/// Calculate storage slot
pub fn slot(input: Option<String>) -> Action {
    let Some(input) = input else {
        return Action::Notify(
            "Usage: :slot mapping <slot> <key> | :slot array <slot> <index>".into(),
            NotifyLevel::Warn,
        );
    };

    let parts: Vec<&str> = input.split_whitespace().collect();

    match parts.as_slice() {
        ["mapping", slot_str, key_str] => {
            calc_mapping_slot(slot_str, key_str)
        }
        ["array", slot_str, index_str] => {
            calc_array_slot(slot_str, index_str)
        }
        _ => Action::Notify(
            "Usage: :slot mapping <slot> <key> | :slot array <slot> <index>".into(),
            NotifyLevel::Warn,
        ),
    }
}

fn calc_mapping_slot(slot_str: &str, key_str: &str) -> Action {
    // Parse slot number
    let slot: u64 = match slot_str.parse() {
        Ok(s) => s,
        Err(_) => return Action::Notify(format!("Invalid slot: {}", slot_str), NotifyLevel::Error),
    };

    // Parse key (address or uint)
    let key_bytes = if key_str.starts_with("0x") || key_str.starts_with("0X") {
        // Try as address first (20 bytes)
        if let Ok(addr) = key_str.parse::<Address>() {
            let mut buf = [0u8; 32];
            buf[12..32].copy_from_slice(addr.as_slice());
            buf.to_vec()
        } else {
            // Try as bytes32
            match hex::decode(&key_str[2..]) {
                Ok(bytes) => {
                    if bytes.len() > 32 {
                        return Action::Notify("Key too large".into(), NotifyLevel::Error);
                    }
                    let mut buf = [0u8; 32];
                    let start = 32 - bytes.len();
                    buf[start..].copy_from_slice(&bytes);
                    buf.to_vec()
                }
                Err(e) => return Action::Notify(format!("Invalid hex key: {}", e), NotifyLevel::Error),
            }
        }
    } else {
        // Try as decimal number
        match key_str.parse::<u128>() {
            Ok(n) => {
                let mut buf = [0u8; 32];
                buf[16..].copy_from_slice(&n.to_be_bytes());
                buf.to_vec()
            }
            Err(_) => return Action::Notify(format!("Invalid key: {}", key_str), NotifyLevel::Error),
        }
    };

    // Calculate: keccak256(key ++ slot)
    let slot_bytes = U256::from(slot).to_be_bytes::<32>();
    let mut concat = Vec::with_capacity(64);
    concat.extend_from_slice(&key_bytes);
    concat.extend_from_slice(&slot_bytes);

    let result_hash = keccak256(&concat);
    let result_slot = U256::from_be_bytes(*result_hash.as_ref());

    ToolResult::new("Storage Slot")
        .add("type", "mapping", )
        .add("base_slot", slot.to_string())
        .add("key", key_str)
        .add("slot", format!("{}", result_slot))
        .add("slot_hex", format!("0x{:x}", result_slot))
        .into_action()
}

fn calc_array_slot(slot_str: &str, index_str: &str) -> Action {
    // Parse slot number
    let slot: u64 = match slot_str.parse() {
        Ok(s) => s,
        Err(_) => return Action::Notify(format!("Invalid slot: {}", slot_str), NotifyLevel::Error),
    };

    // Parse index
    let index: u64 = match index_str.parse() {
        Ok(i) => i,
        Err(_) => return Action::Notify(format!("Invalid index: {}", index_str), NotifyLevel::Error),
    };

    // Calculate: keccak256(slot) + index
    let slot_bytes = U256::from(slot).to_be_bytes::<32>();
    let array_start = keccak256(&slot_bytes);
    let array_start_uint = U256::from_be_bytes(*array_start.as_ref());
    let result_slot = array_start_uint + U256::from(index);

    ToolResult::new("Storage Slot")
        .add("type", "array")
        .add("base_slot", slot.to_string())
        .add("index", index.to_string())
        .add("slot", format!("{}", result_slot))
        .add("slot_hex", format!("0x{:x}", result_slot))
        .into_action()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slot_parsing() {
        // Just verify it doesn't panic
        let _ = slot(Some("mapping 0 0x1234567890123456789012345678901234567890".into()));
        let _ = slot(Some("array 1 5".into()));
    }
}
