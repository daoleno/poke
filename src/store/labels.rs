use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::{params, Connection};

#[derive(Debug)]
pub struct LabelStore {
    conn: Connection,
}

impl LabelStore {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path).with_context(|| format!("open db {}", path.display()))?;
        let store = Self { conn };
        store.init()?;
        Ok(store)
    }

    pub fn load_all(&self) -> Result<BTreeMap<String, String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT address, label FROM labels ORDER BY address")?;
        let mut rows = stmt.query([])?;
        let mut out = BTreeMap::new();
        while let Some(row) = rows.next()? {
            let address: String = row.get(0)?;
            let label: String = row.get(1)?;
            out.insert(address, label);
        }
        Ok(out)
    }

    pub fn set_label(&self, address: &str, label: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO labels(address, label) VALUES (?1, ?2)
             ON CONFLICT(address) DO UPDATE SET label=excluded.label",
            params![address, label],
        )?;
        Ok(())
    }

    pub fn remove_label(&self, address: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM labels WHERE address = ?1", params![address])?;
        Ok(())
    }

    fn init(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS labels (
                address TEXT PRIMARY KEY,
                label   TEXT NOT NULL
            );",
        )?;
        Ok(())
    }
}
