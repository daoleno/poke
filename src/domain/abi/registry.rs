//! ABI registry - stores function signatures by selector

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// A function parameter specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamSpec {
    /// Parameter name (may be empty)
    pub name: String,
    /// Solidity type (e.g., "address", "uint256", "(uint256,address)")
    pub kind: String,
}

/// A function signature with its metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionSignature {
    /// 4-byte function selector
    pub selector: [u8; 4],
    /// Function name
    pub name: String,
    /// Full signature string (e.g., "transfer(address,uint256)")
    pub signature: String,
    /// Input parameters
    pub inputs: Vec<ParamSpec>,
    /// Source file where this ABI was found
    pub source: PathBuf,
}

impl FunctionSignature {
    /// Get selector as hex string
    pub fn selector_hex(&self) -> String {
        format!("0x{}", hex::encode(self.selector))
    }
}

/// Registry of function signatures indexed by selector
#[derive(Debug, Default, Clone)]
pub struct AbiRegistry {
    /// Functions indexed by 4-byte selector
    functions: HashMap<[u8; 4], FunctionSignature>,
    /// Number of files scanned
    pub scanned_files: usize,
    /// Number of functions loaded
    pub loaded_functions: usize,
    /// Scan errors
    pub errors: Vec<String>,
    /// Scan duration in milliseconds
    pub scan_ms: u128,
}

impl AbiRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a function signature
    ///
    /// Note: First function for a given selector wins (no overwrite)
    pub fn insert(&mut self, function: FunctionSignature) {
        self.functions.entry(function.selector).or_insert(function);
    }

    /// Look up a function by selector
    pub fn lookup(&self, selector: [u8; 4]) -> Option<&FunctionSignature> {
        self.functions.get(&selector)
    }

    /// Look up a function by selector hex string (e.g., "0xa9059cbb")
    pub fn lookup_hex(&self, selector_hex: &str) -> Option<&FunctionSignature> {
        let normalized = selector_hex
            .strip_prefix("0x")
            .or_else(|| selector_hex.strip_prefix("0X"))
            .unwrap_or(selector_hex);

        if normalized.len() != 8 {
            return None;
        }

        let bytes = hex::decode(normalized).ok()?;
        if bytes.len() != 4 {
            return None;
        }

        let selector: [u8; 4] = bytes.try_into().ok()?;
        self.lookup(selector)
    }

    /// Get the number of registered functions
    pub fn len(&self) -> usize {
        self.functions.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.functions.is_empty()
    }

    /// Merge another registry into this one
    ///
    /// Functions from the other registry are only added if their
    /// selector is not already present (first wins).
    pub fn merge(&mut self, other: Self) {
        self.scanned_files = self.scanned_files.saturating_add(other.scanned_files);
        self.errors.extend(other.errors);
        for (selector, function) in other.functions {
            self.functions.entry(selector).or_insert(function);
        }
        self.loaded_functions = self.functions.len();
    }

    /// Get all selectors
    pub fn selectors(&self) -> impl Iterator<Item = &[u8; 4]> {
        self.functions.keys()
    }

    /// Get all functions
    pub fn functions(&self) -> impl Iterator<Item = &FunctionSignature> {
        self.functions.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_insert_lookup() {
        let mut registry = AbiRegistry::new();
        let func = FunctionSignature {
            selector: [0xa9, 0x05, 0x9c, 0xbb],
            name: "transfer".to_string(),
            signature: "transfer(address,uint256)".to_string(),
            inputs: vec![
                ParamSpec {
                    name: "to".to_string(),
                    kind: "address".to_string(),
                },
                ParamSpec {
                    name: "amount".to_string(),
                    kind: "uint256".to_string(),
                },
            ],
            source: PathBuf::from("test.json"),
        };

        registry.insert(func.clone());

        assert_eq!(registry.len(), 1);
        assert!(registry.lookup([0xa9, 0x05, 0x9c, 0xbb]).is_some());
        assert!(registry.lookup_hex("0xa9059cbb").is_some());
        assert!(registry.lookup_hex("0xdeadbeef").is_none());
    }

    #[test]
    fn test_first_wins() {
        let mut registry = AbiRegistry::new();

        let func1 = FunctionSignature {
            selector: [0xa9, 0x05, 0x9c, 0xbb],
            name: "transfer".to_string(),
            signature: "transfer(address,uint256)".to_string(),
            inputs: vec![],
            source: PathBuf::from("first.json"),
        };

        let func2 = FunctionSignature {
            selector: [0xa9, 0x05, 0x9c, 0xbb],
            name: "transferV2".to_string(),
            signature: "transferV2(address,uint256)".to_string(),
            inputs: vec![],
            source: PathBuf::from("second.json"),
        };

        registry.insert(func1);
        registry.insert(func2);

        assert_eq!(registry.len(), 1);
        let found = registry.lookup([0xa9, 0x05, 0x9c, 0xbb]).unwrap();
        assert_eq!(found.name, "transfer");
    }
}
