//! Gas estimation command

use super::ToolResult;
use crate::core::{Action, NotifyLevel};

/// Estimate gas for a transaction
/// Syntax: :gas <address>.<function>(<args>)
/// Example: :gas 0xRouter.swap(...)
pub fn estimate_gas(input: Option<String>) -> Action {
    let Some(input) = input else {
        return Action::Notify(
            "Usage: :gas <address>.<function>(<args>)".into(),
            NotifyLevel::Warn,
        );
    };

    // Parse the call syntax (reuse logic from call.rs)
    match parse_gas_syntax(&input) {
        Ok((address, function, args)) => {
            // For now, return a structured message
            // TODO: Integrate with async runtime for actual gas estimation
            ToolResult::new("Gas Estimation (Preview)")
                .add("address", &address)
                .add("function", &function)
                .add("args", &args)
                .add("status", "RPC integration pending")
                .into_action()
        }
        Err(e) => Action::Notify(format!("Parse error: {}", e), NotifyLevel::Error),
    }
}

fn parse_gas_syntax(input: &str) -> Result<(String, String, String), String> {
    // Same parsing logic as call
    let dot_pos = input.find('.').ok_or("Missing '.' between address and function")?;

    let address = input[..dot_pos].trim().to_string();
    let rest = &input[dot_pos + 1..];

    let paren_pos = rest.find('(').ok_or("Missing '(' in function call")?;
    let function = rest[..paren_pos].trim().to_string();

    if !rest.ends_with(')') {
        return Err("Missing closing ')' in function call".into());
    }
    let args = rest[paren_pos + 1..rest.len() - 1].trim().to_string();

    Ok((address, function, args))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_gas_syntax() {
        let (addr, func, args) = parse_gas_syntax("0xRouter.swap(0x123,1000)").unwrap();
        assert_eq!(addr, "0xRouter");
        assert_eq!(func, "swap");
        assert_eq!(args, "0x123,1000");
    }
}
