//! ABI infrastructure - Alloy-based ABI scanning and decoding

mod decoder;
mod resolver;
mod scanner;

pub use resolver::AbiResolver;
pub use scanner::AbiScanner;
