use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct TokenSpec {
    pub address: String,
    pub symbol: Option<String>,
    pub decimals: Option<u8>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EndpointConfig {
    pub name: Option<String>,
    pub rpc: Option<String>,
    pub ipc: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub tokens: Vec<TokenSpec>,

    #[serde(default)]
    pub endpoints: Vec<EndpointConfig>,

    #[serde(default)]
    pub abi_paths: Vec<String>,
}

impl TokenSpec {
    pub fn normalized_address(&self) -> String {
        normalize_address(&self.address)
    }

    pub fn display_symbol(&self) -> String {
        self.symbol
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| short_addr(&self.address))
    }
}

pub fn load() -> Config {
    let Some(path) = config_path() else {
        return Config::default();
    };
    let content = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(_) => return Config::default(),
    };
    toml::from_str::<Config>(&content).unwrap_or_default()
}

pub fn config_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("POKE_CONFIG").map(PathBuf::from) {
        return Some(path);
    }
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from) {
        return Some(xdg.join("poke").join("config.toml"));
    }
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        return Some(home.join(".config").join("poke").join("config.toml"));
    }

    directories::ProjectDirs::from("io", "poke", "poke")
        .map(|dirs| dirs.config_dir().join("config.toml"))
}

pub fn data_dir() -> Option<PathBuf> {
    if let Some(xdg) = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from) {
        return Some(xdg.join("poke"));
    }
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        return Some(home.join(".local").join("share").join("poke"));
    }
    directories::ProjectDirs::from("io", "poke", "poke").map(|dirs| dirs.data_dir().to_path_buf())
}

pub fn labels_db_path() -> Option<PathBuf> {
    data_dir().map(|dir| dir.join("labels.sqlite3"))
}

fn normalize_address(address: &str) -> String {
    let trimmed = address.trim();
    let payload = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);
    format!("0x{}", payload.to_lowercase())
}

fn short_addr(value: &str) -> String {
    let value = value.trim();
    if value.len() <= 10 {
        return value.to_string();
    }
    let start: String = value.chars().take(6).collect();
    let end: String = value
        .chars()
        .rev()
        .take(4)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    format!("{}..{}", start, end)
}
