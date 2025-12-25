//! Persistent cache for resolved ABI signatures

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::Path;

/// Cached function signature from 4byte lookup
#[derive(Debug, Clone)]
pub struct CachedSignature {
    pub selector: String,   // hex with 0x prefix
    pub name: String,       // e.g., "transfer"
    pub signature: String,  // e.g., "transfer(address,uint256)"
}

/// Cached contract ABI from Sourcify
#[derive(Debug, Clone)]
pub struct CachedAbi {
    pub chain_id: u64,
    pub address: String,
    pub abi_json: String,
    pub contract_name: Option<String>,
}

/// SQLite-backed ABI cache
#[derive(Debug)]
pub struct AbiCache {
    conn: Connection,
}

impl AbiCache {
    /// Open or create the cache database
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path).with_context(|| format!("open db {}", path.display()))?;
        let cache = Self { conn };
        cache.init()?;
        Ok(cache)
    }

    /// Initialize database schema
    fn init(&self) -> Result<()> {
        self.conn.execute_batch(
            "
            -- Function selector cache (4byte lookups)
            CREATE TABLE IF NOT EXISTS selectors (
                selector    TEXT PRIMARY KEY,
                name        TEXT NOT NULL,
                signature   TEXT NOT NULL,
                created_at  INTEGER DEFAULT (strftime('%s', 'now'))
            );

            -- Contract ABI cache (Sourcify lookups)
            CREATE TABLE IF NOT EXISTS abis (
                chain_id      INTEGER NOT NULL,
                address       TEXT NOT NULL,
                abi_json      TEXT NOT NULL,
                contract_name TEXT,
                created_at    INTEGER DEFAULT (strftime('%s', 'now')),
                PRIMARY KEY (chain_id, address)
            );

            -- Index for faster lookups
            CREATE INDEX IF NOT EXISTS idx_selectors_created ON selectors(created_at);
            CREATE INDEX IF NOT EXISTS idx_abis_created ON abis(created_at);
            ",
        )?;
        Ok(())
    }

    /// Save a resolved function signature
    pub fn save_signature(&self, selector: &str, name: &str, signature: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO selectors(selector, name, signature) VALUES (?1, ?2, ?3)
             ON CONFLICT(selector) DO UPDATE SET name=excluded.name, signature=excluded.signature",
            params![selector, name, signature],
        )?;
        Ok(())
    }

    /// Get a cached function signature by selector
    pub fn get_signature(&self, selector: &str) -> Result<Option<CachedSignature>> {
        let mut stmt = self
            .conn
            .prepare("SELECT selector, name, signature FROM selectors WHERE selector = ?1")?;

        let mut rows = stmt.query(params![selector])?;
        if let Some(row) = rows.next()? {
            Ok(Some(CachedSignature {
                selector: row.get(0)?,
                name: row.get(1)?,
                signature: row.get(2)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Check if a selector is cached
    pub fn has_signature(&self, selector: &str) -> Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM selectors WHERE selector = ?1",
            params![selector],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// Save a resolved contract ABI
    pub fn save_abi(
        &self,
        chain_id: u64,
        address: &str,
        abi_json: &str,
        contract_name: Option<&str>,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO abis(chain_id, address, abi_json, contract_name) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(chain_id, address) DO UPDATE SET
                abi_json=excluded.abi_json,
                contract_name=excluded.contract_name",
            params![chain_id, address.to_lowercase(), abi_json, contract_name],
        )?;
        Ok(())
    }

    /// Get a cached contract ABI
    pub fn get_abi(&self, chain_id: u64, address: &str) -> Result<Option<CachedAbi>> {
        let mut stmt = self.conn.prepare(
            "SELECT chain_id, address, abi_json, contract_name FROM abis
             WHERE chain_id = ?1 AND address = ?2",
        )?;

        let mut rows = stmt.query(params![chain_id, address.to_lowercase()])?;
        if let Some(row) = rows.next()? {
            Ok(Some(CachedAbi {
                chain_id: row.get(0)?,
                address: row.get(1)?,
                abi_json: row.get(2)?,
                contract_name: row.get(3)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Get all cached signatures (for preloading)
    pub fn get_all_signatures(&self) -> Result<Vec<CachedSignature>> {
        let mut stmt = self
            .conn
            .prepare("SELECT selector, name, signature FROM selectors ORDER BY selector")?;

        let mut rows = stmt.query([])?;
        let mut results = Vec::new();
        while let Some(row) = rows.next()? {
            results.push(CachedSignature {
                selector: row.get(0)?,
                name: row.get(1)?,
                signature: row.get(2)?,
            });
        }
        Ok(results)
    }

    /// Get cache statistics
    pub fn stats(&self) -> Result<(usize, usize)> {
        let sig_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM selectors", [], |row| row.get(0))?;
        let abi_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM abis", [], |row| row.get(0))?;
        Ok((sig_count as usize, abi_count as usize))
    }

    /// Clean old entries (older than 30 days)
    pub fn cleanup_old_entries(&self, max_age_days: u32) -> Result<usize> {
        let cutoff = max_age_days as i64 * 24 * 60 * 60;
        let deleted: usize = self.conn.execute(
            "DELETE FROM selectors WHERE created_at < (strftime('%s', 'now') - ?1)",
            params![cutoff],
        )?;
        let deleted_abis: usize = self.conn.execute(
            "DELETE FROM abis WHERE created_at < (strftime('%s', 'now') - ?1)",
            params![cutoff],
        )?;
        Ok(deleted + deleted_abis)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_db() -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("poke_test_{}.db", std::process::id()));
        path
    }

    #[test]
    fn test_signature_cache() {
        let path = temp_db();
        let cache = AbiCache::open(&path).unwrap();

        cache
            .save_signature("0xd0e30db0", "deposit", "deposit()")
            .unwrap();

        let sig = cache.get_signature("0xd0e30db0").unwrap();
        assert!(sig.is_some());
        let sig = sig.unwrap();
        assert_eq!(sig.name, "deposit");
        assert_eq!(sig.signature, "deposit()");

        std::fs::remove_file(path).ok();
    }

    #[test]
    fn test_abi_cache() {
        let path = temp_db();
        let cache = AbiCache::open(&path).unwrap();

        cache
            .save_abi(
                1,
                "0x1234567890abcdef",
                r#"[{"type":"function","name":"test"}]"#,
                Some("TestContract"),
            )
            .unwrap();

        let abi = cache.get_abi(1, "0x1234567890abcdef").unwrap();
        assert!(abi.is_some());
        let abi = abi.unwrap();
        assert_eq!(abi.contract_name, Some("TestContract".to_string()));

        std::fs::remove_file(path).ok();
    }
}
