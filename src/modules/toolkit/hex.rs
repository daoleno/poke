//! Hex/decimal/string conversion

use super::ToolResult;
use crate::core::{Action, NotifyLevel};

/// Convert between hex, decimal, and string representations
pub fn hex_convert(input: Option<String>) -> Action {
    let Some(input) = input else {
        return Action::Notify("Usage: :hex <value>".into(), NotifyLevel::Warn);
    };

    let input = input.trim();
    if input.is_empty() {
        return Action::Notify("Usage: :hex <value>".into(), NotifyLevel::Warn);
    }

    // Detect input type and convert
    if input.starts_with("0x") || input.starts_with("0X") {
        // Hex input
        let hex_str = &input[2..];
        convert_from_hex(hex_str)
    } else if input.chars().all(|c| c.is_ascii_digit()) {
        // Decimal input
        convert_from_decimal(input)
    } else {
        // String input
        convert_from_string(input)
    }
}

fn convert_from_hex(hex_str: &str) -> Action {
    let bytes = match hex::decode(hex_str) {
        Ok(b) => b,
        Err(e) => return Action::Notify(format!("Invalid hex: {}", e), NotifyLevel::Error),
    };

    let mut result = ToolResult::new("Hex Convert")
        .add("hex", format!("0x{}", hex_str.to_lowercase()));

    // Try to parse as number (up to u128)
    if bytes.len() <= 16 {
        let mut arr = [0u8; 16];
        let start = 16 - bytes.len();
        arr[start..].copy_from_slice(&bytes);
        let num = u128::from_be_bytes(arr);
        result = result.add("dec", num.to_string());
    }

    // Try to decode as UTF-8 string
    if let Ok(s) = String::from_utf8(bytes.clone()) {
        if s.chars().all(|c| !c.is_control() || c == '\n' || c == '\t') {
            result = result.add("string", format!("\"{}\"", s));
        }
    }

    result = result.add("bytes", bytes.len().to_string());

    result.into_action()
}

fn convert_from_decimal(dec_str: &str) -> Action {
    let num: u128 = match dec_str.parse() {
        Ok(n) => n,
        Err(e) => return Action::Notify(format!("Invalid decimal: {}", e), NotifyLevel::Error),
    };

    let hex_str = format!("{:x}", num);
    let bytes = hex::decode(&hex_str).unwrap_or_default();

    let mut result = ToolResult::new("Hex Convert")
        .add("dec", num.to_string())
        .add("hex", format!("0x{}", hex_str));

    // If small enough, also show as bytes32 padded
    if num <= u64::MAX as u128 {
        result = result.add("bytes32", format!("0x{:064x}", num));
    }

    result.into_action()
}

fn convert_from_string(s: &str) -> Action {
    let bytes = s.as_bytes();
    let hex_str = hex::encode(bytes);

    let result = ToolResult::new("Hex Convert")
        .add("string", format!("\"{}\"", s))
        .add("hex", format!("0x{}", hex_str))
        .add("bytes", bytes.len().to_string());

    result.into_action()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_to_dec() {
        // This would need the Action to be inspectable for proper testing
        // For now just verify it doesn't panic
        let _ = hex_convert(Some("0xff".to_string()));
        let _ = hex_convert(Some("255".to_string()));
        let _ = hex_convert(Some("hello".to_string()));
    }
}
