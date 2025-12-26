//! CREATE2 address calculation

use alloy::primitives::{keccak256, Address, B256};
use super::ToolResult;
use crate::core::{Action, NotifyLevel};

/// Calculate CREATE2 address
pub fn create2_address(input: Option<String>) -> Action {
    let Some(input) = input else {
        return Action::Notify(
            "Usage: :create2 <deployer> <salt> <initcode_hash>".into(),
            NotifyLevel::Warn,
        );
    };

    let parts: Vec<&str> = input.split_whitespace().collect();

    if parts.len() != 3 {
        return Action::Notify(
            "Usage: :create2 <deployer> <salt> <initcode_hash>".into(),
            NotifyLevel::Warn,
        );
    }

    // Parse deployer address
    let deployer: Address = match parts[0].parse() {
        Ok(addr) => addr,
        Err(e) => return Action::Notify(
            format!("Invalid deployer address: {}", e),
            NotifyLevel::Error,
        ),
    };

    // Parse salt
    let salt: B256 = match parts[1].parse() {
        Ok(s) => s,
        Err(e) => {
            // Try as decimal number
            match parts[1].parse::<u64>() {
                Ok(n) => {
                    let mut bytes = [0u8; 32];
                    bytes[24..].copy_from_slice(&n.to_be_bytes());
                    B256::from(bytes)
                }
                Err(_) => return Action::Notify(
                    format!("Invalid salt: {}", e),
                    NotifyLevel::Error,
                ),
            }
        }
    };

    // Parse initcode hash
    let initcode_hash: B256 = match parts[2].parse() {
        Ok(h) => h,
        Err(e) => return Action::Notify(
            format!("Invalid initcode hash: {}", e),
            NotifyLevel::Error,
        ),
    };

    // Calculate CREATE2 address
    let address = compute_create2_address(deployer, salt, initcode_hash);

    ToolResult::new("CREATE2 Address")
        .add("deployer", format!("{:?}", deployer))
        .add("salt", format!("{:?}", salt))
        .add("initcode_hash", format!("{:?}", initcode_hash))
        .add("address", format!("{:?}", address))
        .into_action()
}

fn compute_create2_address(deployer: Address, salt: B256, initcode_hash: B256) -> Address {
    // CREATE2 address = keccak256(0xff ++ deployer ++ salt ++ initcode_hash)[12:]
    let mut buffer = Vec::with_capacity(85);
    buffer.push(0xff);
    buffer.extend_from_slice(deployer.as_slice());
    buffer.extend_from_slice(salt.as_ref());
    buffer.extend_from_slice(initcode_hash.as_ref());

    let hash = keccak256(&buffer);
    Address::from_slice(&hash[12..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create2_address() {
        // Test with known values
        let deployer: Address = "0x0000000000000000000000000000000000000000".parse().unwrap();
        let salt = B256::ZERO;
        let initcode_hash = B256::ZERO;

        let addr = compute_create2_address(deployer, salt, initcode_hash);

        // Known result for all zeros
        assert_eq!(
            addr,
            "0xe33c0c7f7df4809055c3eba6c09cfe4baf1bd9e0".parse::<Address>().unwrap()
        );
    }

    #[test]
    fn test_create2_address_with_salt() {
        let deployer: Address = "0x0000000000000000000000000000000000000000".parse().unwrap();
        let salt: B256 = "0x0000000000000000000000000000000000000000000000000000000000000001".parse().unwrap();
        let initcode_hash = B256::ZERO;

        let addr = compute_create2_address(deployer, salt, initcode_hash);

        // Should produce different address
        assert_ne!(
            addr,
            "0xe33c0c7f7df4809055c3eba6c09cfe4baf1bd9e0".parse::<Address>().unwrap()
        );
    }
}
