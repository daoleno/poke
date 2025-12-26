//! Ethereum address checksum (EIP-55)

use alloy::primitives::keccak256;
use super::ToolResult;
use crate::core::{Action, NotifyLevel};

/// Convert address to checksummed format
pub fn checksum(input: Option<String>) -> Action {
    let Some(input) = input else {
        return Action::Notify("Usage: :checksum <address>".into(), NotifyLevel::Warn);
    };

    let addr = input.trim().to_lowercase();
    let addr = addr.strip_prefix("0x").unwrap_or(&addr);

    if addr.len() != 40 || !addr.chars().all(|c| c.is_ascii_hexdigit()) {
        return Action::Notify("Invalid address (expected 40 hex chars)".into(), NotifyLevel::Error);
    }

    let checksummed = to_checksum_address(addr);

    let result = ToolResult::new("Checksum")
        .add("address", &checksummed)
        .add("valid", "✓");

    result.into_action()
}

fn to_checksum_address(addr: &str) -> String {
    let hash = keccak256(addr.as_bytes());
    let hash_hex = hex::encode(hash.as_slice());

    let mut result = String::with_capacity(42);
    result.push_str("0x");

    for (i, c) in addr.chars().enumerate() {
        let hash_char = hash_hex.chars().nth(i).unwrap_or('0');
        let hash_val = hash_char.to_digit(16).unwrap_or(0);

        if hash_val >= 8 {
            result.push(c.to_ascii_uppercase());
        } else {
            result.push(c);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checksum() {
        // Example addresses
        let addr = "0xfb6916095ca1df60bb79ce92ce3ea74c37c5d359";
        let expected = "0xfB6916095ca1df60bB79Ce92cE3Ea74c37c5d359";
        assert_eq!(to_checksum_address(&addr[2..]), expected);
    }
}
