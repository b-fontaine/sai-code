use color_eyre::Result;
use ratatui::{layout::Rect, Frame};
use tokio::sync::mpsc::UnboundedSender;

use crate::event::{Action, Event};

pub mod activity;
pub mod conversation;
pub mod input_area;
pub mod permission_prompt;
pub mod status_bar;

/// Trait for TUI components following the ratatui component pattern.
pub trait Component {
    /// Register the action sender so this component can dispatch actions.
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        let _ = tx;
        Ok(())
    }

    /// Handle a raw event; return an action to dispatch if needed.
    fn handle_events(&mut self, event: Option<&Event>) -> Result<Option<Action>> {
        let _ = event;
        Ok(None)
    }

    /// Update component state in response to an action; return a chained action if needed.
    fn update(&mut self, action: &Action) -> Result<Option<Action>> {
        let _ = action;
        Ok(None)
    }

    /// Render this component into the given frame area.
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()>;
}
