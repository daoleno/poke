//! Infrastructure layer - External service integrations
//!
//! This layer contains:
//! - Alloy-based Ethereum provider implementations
//! - ABI scanning and decoding using alloy-dyn-abi
//! - Tokio runtime bridge for async operations

pub mod abi;
pub mod ethereum;
pub mod runtime;

// Re-export types used by main.rs
pub use abi::AbiScanner;
pub use runtime::{
    CallStatus, TxStatus,
};
