//! UI port trait for delivering events to the user.

use async_trait::async_trait;

use crate::domain::event::AgentEvent;

/// Port trait for the user interface layer.
///
/// The agent loop emits events through this trait. The UI layer is
/// responsible for rendering them. This trait MUST NOT block the
/// agent loop.
#[async_trait]
#[cfg_attr(test, mockall::automock)]
pub trait UiPort: Send + Sync {
    /// Emit an event to the UI layer.
    async fn emit_event(&self, event: AgentEvent);
}
