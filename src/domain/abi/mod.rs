//! ABI domain models and contracts
//!
//! This module defines the traits and types for ABI decoding,
//! independent of the underlying implementation (alloy-dyn-abi).

mod decoder;
mod registry;

pub use decoder::{AbiDecoder, DecodedArg, DecodedCall};
pub use registry::{AbiRegistry, FunctionSignature, ParamSpec};
