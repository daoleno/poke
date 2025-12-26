//! Real-time logs command

use super::{OpsResult, OpsStatus};
use crate::core::{Action, NotifyLevel};

/// Log level filter
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
    All,
}

impl LogLevel {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "debug" | "d" => Some(LogLevel::Debug),
            "info" | "i" => Some(LogLevel::Info),
            "warn" | "warning" | "w" => Some(LogLevel::Warn),
            "error" | "err" | "e" => Some(LogLevel::Error),
            "all" | "a" => Some(LogLevel::All),
            _ => None,
        }
    }
}

/// Parse and handle :logs command
pub fn logs(input: Option<String>) -> Action {
    let (level, tail_lines) = if let Some(input_str) = input {
        parse_logs_args(&input_str)
    } else {
        (LogLevel::All, 50)
    };

    OpsResult::new("Logs")
        .add("level", format!("{:?}", level), OpsStatus::Ok)
        .add("tail", tail_lines.to_string(), OpsStatus::Ok)
        .add("status", "Log streaming requires async runtime", OpsStatus::Unknown)
        .into_action()
}

fn parse_logs_args(input: &str) -> (LogLevel, usize) {
    let mut level = LogLevel::All;
    let mut tail = 50;

    let parts: Vec<&str> = input.split_whitespace().collect();
    let mut i = 0;

    while i < parts.len() {
        match parts[i] {
            "--level" | "-l" => {
                if i + 1 < parts.len() {
                    if let Some(parsed_level) = LogLevel::from_str(parts[i + 1]) {
                        level = parsed_level;
                    }
                    i += 1;
                }
            }
            "--tail" | "-n" => {
                if i + 1 < parts.len() {
                    if let Ok(n) = parts[i + 1].parse() {
                        tail = n;
                    }
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }

    (level, tail)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_logs_args() {
        let (level, tail) = parse_logs_args("--level error --tail 100");
        assert_eq!(level, LogLevel::Error);
        assert_eq!(tail, 100);
    }

    #[test]
    fn test_log_level_from_str() {
        assert_eq!(LogLevel::from_str("error"), Some(LogLevel::Error));
        assert_eq!(LogLevel::from_str("info"), Some(LogLevel::Info));
        assert_eq!(LogLevel::from_str("warn"), Some(LogLevel::Warn));
    }
}
