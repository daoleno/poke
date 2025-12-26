//! CREATE address calculation

use alloy::primitives::Address;
use super::ToolResult;
use crate::core::{Action, NotifyLevel};

/// Calculate CREATE address from deployer + nonce
pub fn create_address(input: Option<String>) -> Action {
    let Some(input) = input else {
        return Action::Notify(
            "Usage: :create <deployer_address> <nonce>".into(),
            NotifyLevel::Warn,
        );
    };

    let parts: Vec<&str> = input.split_whitespace().collect();

    if parts.len() != 2 {
        return Action::Notify(
            "Usage: :create <deployer_address> <nonce>".into(),
            NotifyLevel::Warn,
        );
    }

    // Parse deployer address
    let deployer: Address = match parts[0].parse() {
        Ok(addr) => addr,
        Err(e) => return Action::Notify(
            format!("Invalid address: {}", e),
            NotifyLevel::Error,
        ),
    };

    // Parse nonce
    let nonce: u64 = match parts[1].parse() {
        Ok(n) => n,
        Err(e) => return Action::Notify(
            format!("Invalid nonce: {}", e),
            NotifyLevel::Error,
        ),
    };

    // Calculate CREATE address using alloy's built-in method
    let address = deployer.create(nonce);

    ToolResult::new("CREATE Address")
        .add("deployer", format!("{:?}", deployer))
        .add("nonce", nonce.to_string())
        .add("address", format!("{:?}", address))
        .into_action()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_address() {
        // Test with nonce 0
        let deployer: Address = "0x0000000000000000000000000000000000000000".parse().unwrap();
        let addr = deployer.create(0);
        // Known result for deployer=0x0, nonce=0
        let expected: Address = "0xbd770416a3345f91e4b34576cb804a576fa48eb1".parse().unwrap();
        assert_eq!(addr, expected);
    }

    #[test]
    fn test_create_address_nonce_1() {
        let deployer: Address = "0x0000000000000000000000000000000000000000".parse().unwrap();
        let addr = deployer.create(1);
        let expected: Address = "0x5a443704dd4b594b382c22a083e2bd3090a6fef3".parse().unwrap();
        assert_eq!(addr, expected);
    }
}
