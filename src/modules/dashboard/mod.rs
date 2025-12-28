//! Dashboard module - minimal state for tab-based UI

use crate::core::{Action, Context, Module};

/// Dashboard state (minimal for new tab-based UI)
#[derive(Clone, Debug)]
pub struct Dashboard {
    // Reserved for future use
    _placeholder: (),
}

impl Dashboard {
    pub fn new() -> Self {
        Self { _placeholder: () }
    }
}

impl Module for Dashboard {
    fn handle_key(&mut self, _key: crossterm::event::KeyEvent, _ctx: &mut Context) -> Action {
        // Tab-based UI handles keys in main.rs
        Action::None
    }
}
