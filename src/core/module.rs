//! Module trait for extensible UI components

use crossterm::event::KeyEvent;

use super::{Action, Context};

/// Trait for UI modules that can handle input
pub trait Module {
    /// Handle keyboard input
    /// Returns an Action describing what should happen
    fn handle_key(&mut self, key: KeyEvent, ctx: &mut Context) -> Action;
}
