//! Remote ABI resolution via 4byte and Sourcify APIs
#![allow(dead_code)]

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Resolved function signature from 4byte database
#[derive(Debug, Clone)]
pub struct ResolvedSignature {
    pub selector: [u8; 4],
    pub name: String,      // e.g., "transfer"
    pub signature: String, // e.g., "transfer(address,uint256)"
}

/// Contract ABI from Sourcify
#[derive(Debug, Clone)]
pub struct ResolvedAbi {
    pub address: String,
    pub chain_id: u64,
    pub abi_json: String,
    pub contract_name: Option<String>,
}

/// 4byte API response structures
#[derive(Debug, Deserialize)]
struct OpenChainResponse {
    ok: bool,
    result: OpenChainResult,
}

#[derive(Debug, Deserialize)]
struct OpenChainResult {
    function: HashMap<String, Vec<OpenChainSignature>>,
}

#[derive(Debug, Deserialize)]
struct OpenChainSignature {
    name: String,
    // filtered: bool, // not always present
}

/// Sourcify API response structures
#[derive(Debug, Deserialize)]
struct SourcifyResponse {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    abi: Option<serde_json::Value>,
    #[serde(default)]
    name: Option<String>,
}

/// Remote ABI resolver with in-memory caching
pub struct AbiResolver {
    http: reqwest::Client,
    /// Cache: selector hex -> signatures
    selector_cache: Arc<RwLock<HashMap<String, Vec<ResolvedSignature>>>>,
    /// Cache: (chain_id, address) -> ABI
    abi_cache: Arc<RwLock<HashMap<(u64, String), Option<ResolvedAbi>>>>,
    /// Pending lookups to avoid duplicate requests
    pending_selectors: Arc<RwLock<HashMap<String, bool>>>,
}

impl AbiResolver {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("Failed to create HTTP client"),
            selector_cache: Arc::new(RwLock::new(HashMap::new())),
            abi_cache: Arc::new(RwLock::new(HashMap::new())),
            pending_selectors: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Lookup function signature by 4-byte selector using OpenChain API
    /// Returns the most likely signature (first result)
    pub async fn lookup_selector(&self, selector: [u8; 4]) -> Result<Option<ResolvedSignature>> {
        let selector_hex = format!("0x{}", hex::encode(selector));

        // Check cache first
        {
            let cache = self.selector_cache.read().await;
            if let Some(sigs) = cache.get(&selector_hex) {
                return Ok(sigs.first().cloned());
            }
        }

        // Check if already pending
        {
            let pending = self.pending_selectors.read().await;
            if pending.contains_key(&selector_hex) {
                return Ok(None); // Request in progress
            }
        }

        // Mark as pending
        {
            let mut pending = self.pending_selectors.write().await;
            pending.insert(selector_hex.clone(), true);
        }

        // Query OpenChain API (used by cast 4byte)
        let url = format!(
            "https://api.openchain.xyz/signature-database/v1/lookup?function={}&filter=true",
            selector_hex
        );

        // Debug: Log the lookup request
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/poke-abi-debug.log")
        {
            use std::io::Write;
            let _ = writeln!(f, "[LOOKUP] Requesting selector: {}", selector_hex);
        }

        let result = self.fetch_selector_from_api(&url, &selector_hex, selector).await;

        // Debug: Log the result
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/poke-abi-debug.log")
        {
            use std::io::Write;
            match &result {
                Ok(Some(sig)) => {
                    let _ = writeln!(
                        f,
                        "[LOOKUP] Got result for {}: {} ({})",
                        selector_hex, sig.name, sig.signature
                    );
                }
                Ok(None) => {
                    let _ = writeln!(f, "[LOOKUP] No result for {}", selector_hex);
                }
                Err(e) => {
                    let _ = writeln!(f, "[LOOKUP] Error for {}: {:?}", selector_hex, e);
                }
            }
        }

        // Remove from pending
        {
            let mut pending = self.pending_selectors.write().await;
            pending.remove(&selector_hex);
        }

        result
    }

    async fn fetch_selector_from_api(
        &self,
        url: &str,
        selector_hex: &str,
        selector: [u8; 4],
    ) -> Result<Option<ResolvedSignature>> {
        let response = self
            .http
            .get(url)
            .send()
            .await
            .context("Failed to query OpenChain API")?;

        if !response.status().is_success() {
            // Don't cache failures - allow retry on next request
            // Just log and return None
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/poke-abi-debug.log")
            {
                use std::io::Write;
                let _ = writeln!(f, "[RESOLVER] API returned status {} for {}", response.status(), selector_hex);
            }
            return Ok(None);
        }

        let data: OpenChainResponse = response
            .json()
            .await
            .context("Failed to parse OpenChain response")?;

        if !data.ok {
            // Don't cache - API error, allow retry
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/poke-abi-debug.log")
            {
                use std::io::Write;
                let _ = writeln!(f, "[RESOLVER] API returned ok=false for {}", selector_hex);
            }
            return Ok(None);
        }

        let signatures: Vec<ResolvedSignature> = data
            .result
            .function
            .get(selector_hex)
            .map(|sigs| {
                sigs.iter()
                    .map(|s| {
                        let name = s.name.split('(').next().unwrap_or(&s.name).to_string();
                        ResolvedSignature {
                            selector,
                            name,
                            signature: s.name.clone(),
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Cache results
        {
            let mut cache = self.selector_cache.write().await;
            cache.insert(selector_hex.to_string(), signatures.clone());
        }

        Ok(signatures.first().cloned())
    }

    /// Lookup multiple selectors in batch
    pub async fn lookup_selectors_batch(
        &self,
        selectors: Vec<[u8; 4]>,
    ) -> HashMap<[u8; 4], Option<ResolvedSignature>> {
        let mut results = HashMap::new();

        // Filter out already cached
        let mut to_fetch = Vec::new();
        {
            let cache = self.selector_cache.read().await;
            for sel in selectors {
                let hex = format!("0x{}", hex::encode(sel));
                if let Some(sigs) = cache.get(&hex) {
                    results.insert(sel, sigs.first().cloned());
                } else {
                    to_fetch.push(sel);
                }
            }
        }

        // Fetch remaining (limit concurrent requests)
        for sel in to_fetch.into_iter().take(10) {
            if let Ok(sig) = self.lookup_selector(sel).await {
                results.insert(sel, sig);
            }
        }

        results
    }

    /// Lookup contract ABI from Sourcify
    pub async fn lookup_abi(&self, chain_id: u64, address: &str) -> Result<Option<ResolvedAbi>> {
        let addr = address.to_lowercase();
        let cache_key = (chain_id, addr.clone());

        // Check cache first
        {
            let cache = self.abi_cache.read().await;
            if let Some(abi) = cache.get(&cache_key) {
                return Ok(abi.clone());
            }
        }

        // Query Sourcify API
        let url = format!(
            "https://sourcify.dev/server/v2/contract/{}/{}?fields=abi,name",
            chain_id, addr
        );

        let response = self.http.get(&url).send().await;

        let abi = match response {
            Ok(resp) if resp.status().is_success() => {
                let data: SourcifyResponse = resp
                    .json()
                    .await
                    .context("Failed to parse Sourcify response")?;

                data.abi.map(|abi_json| ResolvedAbi {
                    address: addr.clone(),
                    chain_id,
                    abi_json: abi_json.to_string(),
                    contract_name: data.name,
                })
            }
            _ => None,
        };

        // Cache result (including None for not found)
        {
            let mut cache = self.abi_cache.write().await;
            cache.insert(cache_key, abi.clone());
        }

        Ok(abi)
    }

    /// Get cached selector resolution (non-blocking)
    pub async fn get_cached_selector(&self, selector: [u8; 4]) -> Option<ResolvedSignature> {
        let hex = format!("0x{}", hex::encode(selector));
        let cache = self.selector_cache.read().await;
        cache.get(&hex).and_then(|v| v.first().cloned())
    }

    /// Check if selector is in cache
    pub async fn is_selector_cached(&self, selector: [u8; 4]) -> bool {
        let hex = format!("0x{}", hex::encode(selector));
        let cache = self.selector_cache.read().await;
        cache.contains_key(&hex)
    }

    /// Get cache stats
    pub async fn cache_stats(&self) -> (usize, usize) {
        let sel_count = self.selector_cache.read().await.len();
        let abi_count = self.abi_cache.read().await.len();
        (sel_count, abi_count)
    }
}

impl Default for AbiResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_lookup_deposit_selector() {
        let resolver = AbiResolver::new();
        // deposit() = 0xd0e30db0
        let selector = [0xd0, 0xe3, 0x0d, 0xb0];

        let result = resolver.lookup_selector(selector).await;
        assert!(result.is_ok());

        if let Ok(Some(sig)) = result {
            assert_eq!(sig.name, "deposit");
            assert!(sig.signature.starts_with("deposit("));
        }
    }

    #[tokio::test]
    async fn test_lookup_transfer_selector() {
        let resolver = AbiResolver::new();
        // transfer(address,uint256) = 0xa9059cbb
        let selector = [0xa9, 0x05, 0x9c, 0xbb];

        let result = resolver.lookup_selector(selector).await;
        assert!(result.is_ok());

        if let Ok(Some(sig)) = result {
            assert_eq!(sig.name, "transfer");
        }
    }

    #[tokio::test]
    async fn test_cache_hit() {
        let resolver = AbiResolver::new();
        let selector = [0xd0, 0xe3, 0x0d, 0xb0];

        // First lookup (cache miss)
        let _ = resolver.lookup_selector(selector).await;

        // Second lookup (should be cached)
        assert!(resolver.is_selector_cached(selector).await);
        let cached = resolver.get_cached_selector(selector).await;
        assert!(cached.is_some());
    }
}
