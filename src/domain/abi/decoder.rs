//! ABI decoder trait and types

use serde::{Deserialize, Serialize};

use super::FunctionSignature;

/// A decoded function argument
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodedArg {
    /// Parameter name (or "arg{n}" if unnamed)
    pub name: String,
    /// Solidity type (e.g., "address", "uint256", "(uint256,address)")
    pub kind: String,
    /// Decoded value as a formatted string
    pub value: String,
}

/// Result of decoding a function call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodedCall {
    /// Function name
    pub function_name: String,
    /// Full function signature (e.g., "transfer(address,uint256)")
    pub signature: String,
    /// Decoded arguments
    pub arguments: Vec<DecodedArg>,
}

/// Trait for ABI decoding implementations
///
/// This trait abstracts over the actual ABI decoding implementation,
/// allowing us to swap out alloy-dyn-abi for a different library if needed.
pub trait AbiDecoder: Send + Sync {
    /// Decode calldata given a function signature
    ///
    /// # Arguments
    /// * `function` - The function signature to decode with
    /// * `data` - The calldata bytes (including the 4-byte selector)
    ///
    /// # Returns
    /// * `Ok(DecodedCall)` - The decoded function call
    /// * `Err(...)` - If decoding fails
    fn decode_calldata(
        &self,
        function: &FunctionSignature,
        data: &[u8],
    ) -> anyhow::Result<DecodedCall>;

    /// Decode calldata by looking up the selector
    ///
    /// # Arguments
    /// * `selector` - The 4-byte function selector
    /// * `data` - The calldata bytes (including the 4-byte selector)
    ///
    /// # Returns
    /// * `Ok(Some(DecodedCall))` - If the selector was found and decoding succeeded
    /// * `Ok(None)` - If the selector was not found
    /// * `Err(...)` - If decoding fails
    fn decode_by_selector(
        &self,
        selector: [u8; 4],
        data: &[u8],
    ) -> anyhow::Result<Option<DecodedCall>>;
}
