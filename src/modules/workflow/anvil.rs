//! Anvil local node management

use super::WorkflowResult;
use crate::core::{Action, NotifyLevel};
use std::process::Command;

/// Anvil configuration
#[derive(Clone, Debug)]
pub struct AnvilConfig {
    pub port: u16,
    pub chain_id: Option<u64>,
    pub block_time: Option<u64>,
    pub fork_url: Option<String>,
}

impl Default for AnvilConfig {
    fn default() -> Self {
        Self {
            port: 8545,
            chain_id: None,
            block_time: None,
            fork_url: None,
        }
    }
}

/// Parse and handle :anvil command
pub fn anvil(input: Option<String>) -> Action {
    let input_str = input.as_deref().unwrap_or("").trim();

    match input_str {
        "kill" | "stop" => {
            Action::Notify(
                "Anvil: stop requested (process management in app state)".into(),
                NotifyLevel::Info,
            )
        }
        "status" => {
            Action::Notify(
                "Anvil: status check requested (check app state)".into(),
                NotifyLevel::Info,
            )
        }
        "" => {
            start_anvil(AnvilConfig::default())
        }
        args => {
            match parse_anvil_args(args) {
                Ok(config) => start_anvil(config),
                Err(e) => Action::Notify(format!("Anvil: {}", e), NotifyLevel::Error),
            }
        }
    }
}

fn start_anvil(config: AnvilConfig) -> Action {
    if !is_anvil_available() {
        return Action::Notify(
            "Anvil: not found in PATH. Install: cargo install --git https://github.com/foundry-rs/foundry --locked anvil".into(),
            NotifyLevel::Error,
        );
    }

    WorkflowResult::new("Anvil Start Request")
        .add("port", config.port.to_string())
        .add(
            "chain_id",
            config.chain_id.map(|c| c.to_string()).unwrap_or_else(|| "31337".to_string()),
        )
        .add("status", "Process management pending")
        .into_action()
}

fn parse_anvil_args(args: &str) -> Result<AnvilConfig, String> {
    let mut config = AnvilConfig::default();
    let parts: Vec<&str> = args.split_whitespace().collect();

    let mut i = 0;
    while i < parts.len() {
        match parts[i] {
            "--port" | "-p" => {
                i += 1;
                if i >= parts.len() {
                    return Err("Missing value for --port".into());
                }
                config.port = parts[i]
                    .parse()
                    .map_err(|_| format!("Invalid port: {}", parts[i]))?;
            }
            "--chain-id" => {
                i += 1;
                if i >= parts.len() {
                    return Err("Missing value for --chain-id".into());
                }
                config.chain_id = Some(
                    parts[i]
                        .parse()
                        .map_err(|_| format!("Invalid chain-id: {}", parts[i]))?,
                );
            }
            "--block-time" | "-b" => {
                i += 1;
                if i >= parts.len() {
                    return Err("Missing value for --block-time".into());
                }
                config.block_time = Some(
                    parts[i]
                        .parse()
                        .map_err(|_| format!("Invalid block-time: {}", parts[i]))?,
                );
            }
            "--fork" | "-f" => {
                i += 1;
                if i >= parts.len() {
                    return Err("Missing value for --fork".into());
                }
                config.fork_url = Some(parts[i].to_string());
            }
            unknown => {
                return Err(format!("Unknown argument: {}", unknown));
            }
        }
        i += 1;
    }

    Ok(config)
}

fn is_anvil_available() -> bool {
    Command::new("anvil")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_anvil_args() {
        let config = parse_anvil_args("--port 8546").unwrap();
        assert_eq!(config.port, 8546);
    }

    #[test]
    fn test_parse_anvil_fork() {
        let config = parse_anvil_args("--fork https://mainnet.example.com").unwrap();
        assert_eq!(config.fork_url, Some("https://mainnet.example.com".to_string()));
    }

    #[test]
    fn test_parse_multiple_args() {
        let config = parse_anvil_args("--port 8546 --chain-id 1 --block-time 12").unwrap();
        assert_eq!(config.port, 8546);
        assert_eq!(config.chain_id, Some(1));
        assert_eq!(config.block_time, Some(12));
    }
}
