# Phase 1: Toolkit Implementation

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement the first batch of toolkit commands that provide immediate value to developers

**Architecture:**
- Each tool is a function in `src/modules/toolkit/`
- Tools are invoked via `:command` and display results in a popup overlay
- Tools can read from context (current selection) for auto-fill

**Tech Stack:** Rust, alloy (for ABI encoding), hex crate

---

## Overview

Phase 1 focuses on the most commonly used toolkit commands:

| Priority | Command | Function |
|----------|---------|----------|
| P0 | `:convert` | wei/gwei/ether conversion |
| P0 | `:hex` | hex/dec/string conversion |
| P0 | `:hash` | keccak256 hash |
| P0 | `:selector` | function selector calculation |
| P0 | `:4byte` | reverse lookup selector |
| P1 | `:timestamp` | unix timestamp conversion |
| P1 | `:checksum` | address checksum |

---

## Task 1: Create Toolkit Module Structure

**Files:**
- Create: `src/modules/toolkit/mod.rs`
- Create: `src/modules/toolkit/convert.rs`
- Modify: `src/modules/mod.rs`

**Step 1: Create toolkit module**

Create `src/modules/toolkit/mod.rs`:
```rust
//! Toolkit commands for data processing and conversion

pub mod convert;

use crate::core::{Action, NotifyLevel};

/// Result of a toolkit operation
pub struct ToolResult {
    pub title: String,
    pub content: Vec<(String, String)>, // (label, value) pairs
}

impl ToolResult {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            content: Vec::new(),
        }
    }

    pub fn add(mut self, label: impl Into<String>, value: impl Into<String>) -> Self {
        self.content.push((label.into(), value.into()));
        self
    }

    pub fn into_action(self) -> Action {
        // For now, format as status message; later will be popup
        let msg = self.content
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<_>>()
            .join(" | ");
        Action::Notify(format!("{} - {}", self.title, msg), NotifyLevel::Info)
    }
}
```

**Step 2: Create convert.rs**

Create `src/modules/toolkit/convert.rs`:
```rust
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
        let parsed: f64 = value_str.parse()
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
        integer_part.parse()
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
        padded.parse()
            .map_err(|_| format!("Invalid decimal: {}", decimal_part))?
    };

    Ok(integer.checked_mul(multiplier)
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
        assert_eq!(parse_to_wei("1", "ether").unwrap(), 1_000_000_000_000_000_000);
        assert_eq!(parse_to_wei("1.5", "ether").unwrap(), 1_500_000_000_000_000_000);
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
```

**Step 3: Update modules/mod.rs**

Modify `src/modules/mod.rs`:
```rust
//! UI Modules

pub mod toolkit;
```

**Step 4: Verify compilation**

Run: `cargo check`

**Step 5: Run tests**

Run: `cargo test toolkit`

**Step 6: Commit**

```bash
git add src/modules/
git commit -m "feat(toolkit): add convert command for unit conversion"
```

---

## Task 2: Add hex conversion

**Files:**
- Create: `src/modules/toolkit/hex.rs`
- Modify: `src/modules/toolkit/mod.rs`

**Step 1: Create hex.rs**

Create `src/modules/toolkit/hex.rs`:
```rust
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
```

**Step 2: Update toolkit/mod.rs**

```rust
//! Toolkit commands for data processing and conversion

pub mod convert;
pub mod hex;

// ... rest of mod.rs
```

**Step 3: Verify and commit**

---

## Task 3: Add hash command

**Files:**
- Create: `src/modules/toolkit/hash.rs`
- Modify: `src/modules/toolkit/mod.rs`

**Step 1: Create hash.rs**

Create `src/modules/toolkit/hash.rs`:
```rust
//! Keccak256 hashing

use alloy::primitives::keccak256;
use super::ToolResult;
use crate::core::{Action, NotifyLevel};

/// Compute keccak256 hash of input
pub fn hash(input: Option<String>) -> Action {
    let Some(input) = input else {
        return Action::Notify("Usage: :hash <data>".into(), NotifyLevel::Warn);
    };

    let input = input.trim();
    if input.is_empty() {
        return Action::Notify("Usage: :hash <data>".into(), NotifyLevel::Warn);
    }

    // Check if input is hex
    let data = if input.starts_with("0x") || input.starts_with("0X") {
        match hex::decode(&input[2..]) {
            Ok(bytes) => bytes,
            Err(e) => return Action::Notify(format!("Invalid hex: {}", e), NotifyLevel::Error),
        }
    } else {
        // Treat as string
        input.as_bytes().to_vec()
    };

    let hash = keccak256(&data);
    let hash_hex = format!("0x{}", hex::encode(hash.as_slice()));

    let result = ToolResult::new("Keccak256")
        .add("input", if data.len() > 32 {
            format!("{}... ({} bytes)", &input[..32.min(input.len())], data.len())
        } else {
            input.to_string()
        })
        .add("hash", hash_hex);

    result.into_action()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_string() {
        // keccak256("") = 0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470
        let _ = hash(Some("".to_string()));
    }
}
```

---

## Task 4: Add selector command

**Files:**
- Create: `src/modules/toolkit/selector.rs`
- Modify: `src/modules/toolkit/mod.rs`

**Step 1: Create selector.rs**

```rust
//! Function selector calculation

use alloy::primitives::keccak256;
use super::ToolResult;
use crate::core::{Action, NotifyLevel};

/// Compute function selector from signature
pub fn selector(input: Option<String>) -> Action {
    let Some(input) = input else {
        return Action::Notify("Usage: :selector <function_signature>".into(), NotifyLevel::Warn);
    };

    let sig = input.trim();
    if sig.is_empty() {
        return Action::Notify("Usage: :selector transfer(address,uint256)".into(), NotifyLevel::Warn);
    }

    // Normalize: remove spaces, ensure no returns clause
    let normalized = normalize_signature(sig);

    let hash = keccak256(normalized.as_bytes());
    let selector = &hash[..4];
    let selector_hex = format!("0x{}", hex::encode(selector));

    let result = ToolResult::new("Selector")
        .add("signature", normalized)
        .add("selector", selector_hex);

    result.into_action()
}

fn normalize_signature(sig: &str) -> String {
    // Remove returns clause if present
    let sig = if let Some(pos) = sig.find("returns") {
        sig[..pos].trim()
    } else {
        sig
    };

    // Remove spaces around parentheses and commas
    sig.replace(" ", "")
        .replace(",(", "(")
        .replace(",)", ")")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_signature() {
        assert_eq!(normalize_signature("transfer(address, uint256)"), "transfer(address,uint256)");
        assert_eq!(normalize_signature("transfer(address,uint256) returns (bool)"), "transfer(address,uint256)");
    }
}
```

---

## Task 5: Add 4byte lookup

**Files:**
- Create: `src/modules/toolkit/fourbyte.rs`

**Step 1: Create fourbyte.rs**

```rust
//! 4byte.directory lookup (uses cached signatures from resolver)

use super::ToolResult;
use crate::core::{Action, NotifyLevel};

/// Lookup function selector in signature cache or return placeholder
pub fn fourbyte(input: Option<String>, signature_cache: &std::collections::BTreeMap<String, (String, String)>) -> Action {
    let Some(input) = input else {
        return Action::Notify("Usage: :4byte <selector>".into(), NotifyLevel::Warn);
    };

    let selector = input.trim().to_lowercase();
    let selector = if selector.starts_with("0x") {
        selector
    } else {
        format!("0x{}", selector)
    };

    if selector.len() != 10 {
        return Action::Notify("Selector must be 4 bytes (8 hex chars)".into(), NotifyLevel::Error);
    }

    // Check local cache first
    if let Some((name, sig)) = signature_cache.get(&selector) {
        let result = ToolResult::new("4byte Lookup")
            .add("selector", &selector)
            .add("name", name)
            .add("signature", sig);
        return result.into_action();
    }

    // Not in cache - would need async lookup
    Action::Notify(format!("Selector {} not in cache (async lookup not yet implemented)", selector), NotifyLevel::Info)
}
```

---

## Task 6: Add timestamp conversion

**Files:**
- Create: `src/modules/toolkit/timestamp.rs`

**Step 1: Create timestamp.rs**

```rust
//! Unix timestamp conversion

use super::ToolResult;
use crate::core::{Action, NotifyLevel};
use std::time::{SystemTime, UNIX_EPOCH};

/// Convert between unix timestamp and human readable date
pub fn timestamp(input: Option<String>) -> Action {
    let input = input.map(|s| s.trim().to_string());

    match input.as_deref() {
        None | Some("") | Some("now") => {
            // Show current time
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);

            let result = ToolResult::new("Timestamp")
                .add("unix", now.to_string())
                .add("human", format_timestamp(now));
            result.into_action()
        }
        Some(s) => {
            // Try to parse as unix timestamp
            if let Ok(ts) = s.parse::<u64>() {
                let result = ToolResult::new("Timestamp")
                    .add("unix", ts.to_string())
                    .add("human", format_timestamp(ts));
                return result.into_action();
            }

            Action::Notify(format!("Cannot parse timestamp: {}", s), NotifyLevel::Error)
        }
    }
}

fn format_timestamp(ts: u64) -> String {
    // Simple formatting without external crates
    let secs_per_minute = 60u64;
    let secs_per_hour = 3600u64;
    let secs_per_day = 86400u64;

    // Days since epoch
    let days = ts / secs_per_day;
    let remaining = ts % secs_per_day;
    let hours = remaining / secs_per_hour;
    let remaining = remaining % secs_per_hour;
    let minutes = remaining / secs_per_minute;
    let seconds = remaining % secs_per_minute;

    // Approximate date calculation (not accounting for leap years precisely)
    let mut year = 1970u64;
    let mut remaining_days = days;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    let (month, day) = days_to_month_day(remaining_days as u32, is_leap_year(year));

    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
            year, month, day, hours, minutes, seconds)
}

fn is_leap_year(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn days_to_month_day(mut days: u32, leap: bool) -> (u32, u32) {
    let month_days = if leap {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    for (i, &d) in month_days.iter().enumerate() {
        if days < d {
            return ((i + 1) as u32, days + 1);
        }
        days -= d;
    }
    (12, 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_timestamp() {
        assert_eq!(format_timestamp(0), "1970-01-01 00:00:00 UTC");
        assert_eq!(format_timestamp(1704067200), "2024-01-01 00:00:00 UTC");
    }
}
```

---

## Task 7: Add checksum command

**Files:**
- Create: `src/modules/toolkit/checksum.rs`

**Step 1: Create checksum.rs**

```rust
//! Ethereum address checksum (EIP-55)

use alloy::primitives::keccak256;
use super::ToolResult;
use crate::core::{Action, NotifyLevel};

/// Convert address to checksummed format
pub fn checksum(input: Option<String>) -> Action {
    let Some(input) = input else {
        return Action::Notify("Usage: :checksum <address>".into(), NotifyLevel::Warn);
    };

    let addr = input.trim().to_lowercase();
    let addr = addr.strip_prefix("0x").unwrap_or(&addr);

    if addr.len() != 40 || !addr.chars().all(|c| c.is_ascii_hexdigit()) {
        return Action::Notify("Invalid address (expected 40 hex chars)".into(), NotifyLevel::Error);
    }

    let checksummed = to_checksum_address(addr);

    let result = ToolResult::new("Checksum")
        .add("address", &checksummed)
        .add("valid", "✓");

    result.into_action()
}

fn to_checksum_address(addr: &str) -> String {
    let hash = keccak256(addr.as_bytes());
    let hash_hex = hex::encode(hash.as_slice());

    let mut result = String::with_capacity(42);
    result.push_str("0x");

    for (i, c) in addr.chars().enumerate() {
        let hash_char = hash_hex.chars().nth(i).unwrap_or('0');
        let hash_val = hash_char.to_digit(16).unwrap_or(0);

        if hash_val >= 8 {
            result.push(c.to_ascii_uppercase());
        } else {
            result.push(c);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checksum() {
        // Example addresses
        let addr = "0xfb6916095ca1df60bb79ce92ce3ea74c37c5d359";
        let expected = "0xfB6916095ca1df60bB79Ce92cE3Ea74c37c5d359";
        assert_eq!(to_checksum_address(&addr[2..]), expected);
    }
}
```

---

## Task 8: Wire up toolkit commands in App

**Files:**
- Modify: `src/app.rs`

**Step 1: Update execute_command to call toolkit functions**

In `src/app.rs`, update the `execute_command` method to call the actual toolkit functions:

```rust
// In execute_command method, replace the placeholder responses:

Command::Convert(args) => crate::modules::toolkit::convert::convert(args.clone()),
Command::Hex(args) => crate::modules::toolkit::hex::hex_convert(args.clone()),
Command::Hash(args) => crate::modules::toolkit::hash::hash(args.clone()),
Command::Selector(args) => crate::modules::toolkit::selector::selector(args.clone()),
Command::FourByte(args) => crate::modules::toolkit::fourbyte::fourbyte(args.clone(), &self.signature_cache),
Command::Timestamp(args) => crate::modules::toolkit::timestamp::timestamp(args.clone()),
Command::Checksum(args) => crate::modules::toolkit::checksum::checksum(args.clone()),
```

**Step 2: Verify and test**

Run: `cargo test toolkit`

**Step 3: Commit**

---

## Summary

After completing Phase 1, users can:

- `:convert 1.5 ether` → See wei/gwei/ether values
- `:hex 255` → See 0xff
- `:hex 0x48656c6c6f` → See "Hello"
- `:hash transfer(address,uint256)` → See keccak256 hash
- `:selector transfer(address,uint256)` → See 0xa9059cbb
- `:4byte 0xa9059cbb` → See transfer(address,uint256) (if cached)
- `:timestamp` → See current unix timestamp
- `:timestamp 1704067200` → See 2024-01-01 00:00:00 UTC
- `:checksum 0xfb6916...` → See EIP-55 checksummed address
