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
