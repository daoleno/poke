//! Module trait for extensible UI components

use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;

use super::{Action, Command, Context};

/// Trait for UI modules that can handle input and render themselves
pub trait Module {
    /// Module identifier
    fn id(&self) -> &'static str;

    /// Handle keyboard input
    /// Returns an Action describing what should happen
    fn handle_key(&mut self, key: KeyEvent, ctx: &mut Context) -> Action;

    /// Handle a parsed command
    /// Returns an Action describing what should happen
    fn handle_command(&mut self, cmd: &Command, ctx: &mut Context) -> Action;

    /// Render the module
    fn render(&self, frame: &mut Frame, area: Rect, ctx: &Context);

    /// Whether this module wants focus
    fn focusable(&self) -> bool {
        true
    }

    /// Called when module gains focus
    fn on_focus(&mut self, _ctx: &mut Context) {}

    /// Called when module loses focus
    fn on_blur(&mut self, _ctx: &mut Context) {}

    /// Called on each tick for async updates
    fn tick(&mut self, _ctx: &mut Context) {}
}

/// A boxed module for dynamic dispatch
pub type BoxedModule = Box<dyn Module + Send>;
