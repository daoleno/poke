//! Contract call command (read-only)

use super::ToolResult;
use crate::core::{Action, NotifyLevel};

/// Parse and execute contract call
/// Syntax: :call <address>.<function>(<args>)
/// Example: :call 0xUSDC.balanceOf(0x123...)
pub fn call(input: Option<String>) -> Action {
    let Some(input) = input else {
        return Action::Notify(
            "Usage: :call <address>.<function>(<args>)".into(),
            NotifyLevel::Warn,
        );
    };

    // Parse the call syntax
    match parse_call_syntax(&input) {
        Ok((address, function, args)) => {
            // For now, return a structured message showing what would be called
            // TODO: Integrate with async runtime for actual RPC call
            ToolResult::new("Contract Call (Preview)")
                .add("address", &address)
                .add("function", &function)
                .add("args", &args)
                .add("status", "RPC integration pending")
                .into_action()
        }
        Err(e) => Action::Notify(format!("Parse error: {}", e), NotifyLevel::Error),
    }
}

fn parse_call_syntax(input: &str) -> Result<(String, String, String), String> {
    // Find the dot separating address and function
    let dot_pos = input.find('.').ok_or("Missing '.' between address and function")?;

    let address = input[..dot_pos].trim().to_string();
    let rest = &input[dot_pos + 1..];

    // Find the opening parenthesis
    let paren_pos = rest.find('(').ok_or("Missing '(' in function call")?;
    let function = rest[..paren_pos].trim().to_string();

    // Extract args (everything between parentheses)
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
    fn test_parse_call_syntax() {
        let (addr, func, args) = parse_call_syntax("0xUSDC.balanceOf(0x123)").unwrap();
        assert_eq!(addr, "0xUSDC");
        assert_eq!(func, "balanceOf");
        assert_eq!(args, "0x123");
    }

    #[test]
    fn test_parse_call_no_args() {
        let (addr, func, args) = parse_call_syntax("0xToken.totalSupply()").unwrap();
        assert_eq!(addr, "0xToken");
        assert_eq!(func, "totalSupply");
        assert_eq!(args, "");
    }
}
