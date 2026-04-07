use std::sync::Arc;

use crossterm::event::{KeyEvent, MouseEvent};
use sai_core::domain::event::AgentEvent;

/// Raw events from crossterm or the agent adapter channel.
///
/// `AgentEvent` does not implement `Clone`, so it is wrapped in an `Arc`
/// to allow this enum to remain `Clone`-compatible.
#[derive(Debug, Clone)]
pub enum Event {
    Tick,
    Render,
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
    Agent(Arc<AgentEvent>),
    Error,
}

/// Semantic actions derived from events.
#[derive(Debug, Clone)]
pub enum Action {
    Quit,
    Render,
    SubmitInput,
    AppendInputChar(char),
    DeleteInputChar,
    ClearInput,
    ScrollUp(u16),
    ScrollDown(u16),
    ScrollToBottom,
    ToggleHelp,
    ClearConversation,
    ApprovePermission,
    DenyPermission,
    AgentEvent(Arc<AgentEvent>),
    Error(String),
}
