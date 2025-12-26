//! Unit conversion: wei/gwei/ether

use super::ToolResult;
use crate::core::{Action, NotifyLevel};

/// Parse a value with optional unit and convert to all units
pub fn convert(input: Option<String>) -> Action {
    let Some(input) = input else {
        return Action::Notify("Usage: :convert <value> [unit]".into(), NotifyLevel::Warn);
    };

    let input = input.trim();
    if input.is_empty() {
        return Action::Notify("Usage: :convert <value> [unit]".into(), NotifyLevel::Warn);
    }

    // Parse input: "1.5 ether" or "1500000000000000000" or "1.5e18"
    let (value_str, unit) = parse_value_and_unit(input);

    let wei = match parse_to_wei(&value_str, &unit) {
        Ok(w) => w,
        Err(e) => return Action::Notify(format!("Parse error: {}", e), NotifyLevel::Error),
    };

    let result = ToolResult::new("Convert")
        .add("wei", format_wei(wei))
        .add("gwei", format_gwei(wei))
        .add("ether", format_ether(wei));

    result.into_action()
}

fn parse_value_and_unit(input: &str) -> (String, String) {
    let parts: Vec<&str> = input.split_whitespace().collect();
    match parts.len() {
        1 => (parts[0].to_string(), "wei".to_string()),
        2 => (parts[0].to_string(), parts[1].to_lowercase()),
        _ => (input.to_string(), "wei".to_string()),
    }
}

fn parse_to_wei(value_str: &str, unit: &str) -> Result<u128, String> {
    // Handle scientific notation
    let value_str = if value_str.contains('e') || value_str.contains('E') {
        let parsed: f64 = value_str
            .parse()
            .map_err(|_| format!("Invalid number: {}", value_str))?;
        format!("{:.0}", parsed)
    } else {
        value_str.to_string()
    };

    // Parse as decimal
    let (integer_part, decimal_part) = if let Some(pos) = value_str.find('.') {
        (&value_str[..pos], &value_str[pos + 1..])
    } else {
        (value_str.as_str(), "")
    };

    let multiplier: u128 = match unit {
        "wei" => 1,
        "kwei" | "babbage" => 1_000,
        "mwei" | "lovelace" => 1_000_000,
        "gwei" | "shannon" => 1_000_000_000,
        "szabo" | "microether" => 1_000_000_000_000,
        "finney" | "milliether" => 1_000_000_000_000_000,
        "ether" | "eth" => 1_000_000_000_000_000_000,
        _ => return Err(format!("Unknown unit: {}", unit)),
    };

    let integer: u128 = if integer_part.is_empty() {
        0
    } else {
        integer_part
            .parse()
            .map_err(|_| format!("Invalid integer: {}", integer_part))?
    };

    let decimal_wei = if decimal_part.is_empty() {
        0u128
    } else {
        let decimals = decimal_part.len();
        let multiplier_decimals = multiplier.to_string().len() - 1; // number of zeros
        if decimals > multiplier_decimals {
            return Err("Too many decimal places for unit".to_string());
        }
        let padding = multiplier_decimals - decimals;
        let padded = format!("{}{}", decimal_part, "0".repeat(padding));
        padded
            .parse()
            .map_err(|_| format!("Invalid decimal: {}", decimal_part))?
    };

    Ok(integer
        .checked_mul(multiplier)
        .and_then(|v| v.checked_add(decimal_wei))
        .ok_or("Overflow")?)
}

fn format_wei(wei: u128) -> String {
    // Add thousand separators
    let s = wei.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

fn format_gwei(wei: u128) -> String {
    let gwei = wei / 1_000_000_000;
    let remainder = wei % 1_000_000_000;
    if remainder == 0 {
        format_wei(gwei)
    } else {
        let decimal = format!("{:09}", remainder).trim_end_matches('0').to_string();
        format!("{}.{}", format_wei(gwei), decimal)
    }
}

fn format_ether(wei: u128) -> String {
    let ether = wei / 1_000_000_000_000_000_000;
    let remainder = wei % 1_000_000_000_000_000_000;
    if remainder == 0 {
        format_wei(ether)
    } else {
        let decimal = format!("{:018}", remainder).trim_end_matches('0').to_string();
        format!("{}.{}", format_wei(ether), decimal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_to_wei() {
        assert_eq!(
            parse_to_wei("1", "ether").unwrap(),
            1_000_000_000_000_000_000
        );
        assert_eq!(
            parse_to_wei("1.5", "ether").unwrap(),
            1_500_000_000_000_000_000
        );
        assert_eq!(parse_to_wei("1", "gwei").unwrap(), 1_000_000_000);
        assert_eq!(parse_to_wei("100", "wei").unwrap(), 100);
    }

    #[test]
    fn test_format_ether() {
        assert_eq!(format_ether(1_000_000_000_000_000_000), "1");
        assert_eq!(format_ether(1_500_000_000_000_000_000), "1.5");
        assert_eq!(format_ether(100_000_000_000_000_000), "0.1");
    }
}
