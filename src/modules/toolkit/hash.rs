//! Keccak256 hashing

use alloy::primitives::keccak256;
use super::ToolResult;
use crate::core::{Action, NotifyLevel};

/// Compute keccak256 hash of input
pub fn hash(input: Option<String>) -> Action {
    let Some(input) = input else {
        return Action::Notify("Usage: :hash <data>".into(), NotifyLevel::Warn);
    };

    let input = input.trim();
    if input.is_empty() {
        return Action::Notify("Usage: :hash <data>".into(), NotifyLevel::Warn);
    }

    // Check if input is hex
    let data = if input.starts_with("0x") || input.starts_with("0X") {
        match hex::decode(&input[2..]) {
            Ok(bytes) => bytes,
            Err(e) => return Action::Notify(format!("Invalid hex: {}", e), NotifyLevel::Error),
        }
    } else {
        // Treat as string
        input.as_bytes().to_vec()
    };

    let hash = keccak256(&data);
    let hash_hex = format!("0x{}", hex::encode(hash.as_slice()));

    let result = ToolResult::new("Keccak256")
        .add("input", if data.len() > 32 {
            format!("{}... ({} bytes)", &input[..32.min(input.len())], data.len())
        } else {
            input.to_string()
        })
        .add("hash", hash_hex);

    result.into_action()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_string() {
        // keccak256("") = 0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470
        let _ = hash(Some("".to_string()));
    }
}
