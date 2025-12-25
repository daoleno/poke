//! ABI file scanner - discovers and parses ABI files from the filesystem

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use alloy_json_abi::JsonAbi;
use alloy_primitives::keccak256;
use walkdir::WalkDir;

use crate::domain::abi::{AbiRegistry, FunctionSignature, ParamSpec};

/// ABI file scanner
pub struct AbiScanner;

impl AbiScanner {
    /// Scan a single root directory for ABI files
    pub fn scan(root: impl AsRef<Path>) -> AbiRegistry {
        let started = Instant::now();
        let root = root.as_ref();
        let mut registry = AbiRegistry::new();
        let mut scanned_files = 0;
        let mut errors = Vec::new();

        for entry in WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| !Self::is_ignored_dir(e.path()))
        {
            let entry = match entry {
                Ok(entry) => entry,
                Err(err) => {
                    errors.push(err.to_string());
                    continue;
                }
            };

            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();

            // Only process JSON files in out/ or artifacts/ directories
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            if !Self::path_contains_any(path, &["out", "artifacts"]) {
                continue;
            }

            // Skip files larger than 5MB
            let metadata = match entry.metadata() {
                Ok(meta) => meta,
                Err(err) => {
                    errors.push(format!("{}: {}", path.display(), err));
                    continue;
                }
            };
            if metadata.len() > 5 * 1024 * 1024 {
                continue;
            }

            scanned_files += 1;

            if let Err(err) = Self::load_abi_file(path, &mut registry) {
                errors.push(format!("{}: {}", path.display(), err));
            }
        }

        registry.scanned_files = scanned_files;
        registry.loaded_functions = registry.len();
        registry.errors = errors;
        registry.scan_ms = started.elapsed().as_millis();

        registry
    }

    /// Scan multiple root directories
    pub fn scan_roots(roots: &[PathBuf]) -> AbiRegistry {
        let started = Instant::now();
        let mut registry = AbiRegistry::new();

        for root in roots {
            registry.merge(Self::scan(root));
        }

        registry.loaded_functions = registry.len();
        registry.scan_ms = started.elapsed().as_millis();

        registry
    }

    /// Load a single ABI file
    fn load_abi_file(path: &Path, registry: &mut AbiRegistry) -> anyhow::Result<()> {
        let content = fs::read_to_string(path)?;
        let value: serde_json::Value = serde_json::from_str(&content)?;

        // Try to extract ABI - either raw array or nested in "abi" field
        let abi_value = if value.is_array() {
            value
        } else if let Some(abi) = value.get("abi") {
            abi.clone()
        } else {
            return Ok(()); // No ABI found, skip silently
        };

        // Parse as JsonAbi
        let abi: JsonAbi = serde_json::from_value(abi_value)?;

        // Extract functions
        for function in abi.functions() {
            let signature = function.signature();
            let selector = Self::compute_selector(&signature);

            let inputs: Vec<ParamSpec> = function
                .inputs
                .iter()
                .map(|input| ParamSpec {
                    name: input.name.clone(),
                    kind: input.ty.to_string(),
                })
                .collect();

            let func_sig = FunctionSignature {
                selector,
                name: function.name.clone(),
                signature,
                inputs,
                source: path.to_path_buf(),
            };

            registry.insert(func_sig);
        }

        Ok(())
    }

    /// Compute the 4-byte function selector from a signature
    fn compute_selector(signature: &str) -> [u8; 4] {
        let hash = keccak256(signature.as_bytes());
        [hash[0], hash[1], hash[2], hash[3]]
    }

    /// Check if a path should be ignored
    fn is_ignored_dir(path: &Path) -> bool {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|name| {
                matches!(
                    name,
                    ".git" | "target" | "node_modules" | ".next" | "dist" | "build"
                )
            })
            .unwrap_or(false)
    }

    /// Check if path contains any of the given names
    fn path_contains_any(path: &Path, names: &[&str]) -> bool {
        path.components().any(|component| {
            if let std::path::Component::Normal(value) = component {
                if let Some(value) = value.to_str() {
                    return names.iter().any(|name| *name == value);
                }
            }
            false
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_selector() {
        // transfer(address,uint256) -> 0xa9059cbb
        let selector = AbiScanner::compute_selector("transfer(address,uint256)");
        assert_eq!(selector, [0xa9, 0x05, 0x9c, 0xbb]);

        // approve(address,uint256) -> 0x095ea7b3
        let selector = AbiScanner::compute_selector("approve(address,uint256)");
        assert_eq!(selector, [0x09, 0x5e, 0xa7, 0xb3]);
    }

    #[test]
    fn test_is_ignored_dir() {
        assert!(AbiScanner::is_ignored_dir(Path::new(".git")));
        assert!(AbiScanner::is_ignored_dir(Path::new("node_modules")));
        assert!(!AbiScanner::is_ignored_dir(Path::new("src")));
        assert!(!AbiScanner::is_ignored_dir(Path::new("out")));
    }

    #[test]
    fn test_path_contains_any() {
        assert!(AbiScanner::path_contains_any(
            Path::new("/project/out/Contract.json"),
            &["out", "artifacts"]
        ));
        assert!(AbiScanner::path_contains_any(
            Path::new("/project/artifacts/contracts/Token.json"),
            &["out", "artifacts"]
        ));
        assert!(!AbiScanner::path_contains_any(
            Path::new("/project/src/Contract.sol"),
            &["out", "artifacts"]
        ));
    }
}
