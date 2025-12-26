//! Function selector calculation

use alloy::primitives::keccak256;
use super::ToolResult;
use crate::core::{Action, NotifyLevel};

/// Compute function selector from signature
pub fn selector(input: Option<String>) -> Action {
    let Some(input) = input else {
        return Action::Notify("Usage: :selector <function_signature>".into(), NotifyLevel::Warn);
    };

    let sig = input.trim();
    if sig.is_empty() {
        return Action::Notify("Usage: :selector transfer(address,uint256)".into(), NotifyLevel::Warn);
    }

    // Normalize: remove spaces, ensure no returns clause
    let normalized = normalize_signature(sig);

    let hash = keccak256(normalized.as_bytes());
    let selector = &hash[..4];
    let selector_hex = format!("0x{}", hex::encode(selector));

    let result = ToolResult::new("Selector")
        .add("signature", normalized)
        .add("selector", selector_hex);

    result.into_action()
}

fn normalize_signature(sig: &str) -> String {
    // Remove returns clause if present
    let sig = if let Some(pos) = sig.find("returns") {
        sig[..pos].trim()
    } else {
        sig
    };

    // Remove spaces around parentheses and commas
    sig.replace(" ", "")
        .replace(",(", "(")
        .replace(",)", ")")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_signature() {
        assert_eq!(normalize_signature("transfer(address, uint256)"), "transfer(address,uint256)");
        assert_eq!(normalize_signature("transfer(address,uint256) returns (bool)"), "transfer(address,uint256)");
    }
}
