use std::sync::Arc;

use sai_core::domain::event::AgentEvent;
use tokio::sync::mpsc::UnboundedSender;

use crate::event::Event;

/// Adapter implementing `UiPort` that forwards `AgentEvent`s to the TUI event loop.
#[derive(Clone)]
pub struct TuiUiAdapter {
    event_tx: UnboundedSender<Event>,
}

impl TuiUiAdapter {
    pub fn new(event_tx: UnboundedSender<Event>) -> Self {
        Self { event_tx }
    }
}

#[async_trait::async_trait]
impl sai_core::ports::ui::UiPort for TuiUiAdapter {
    async fn emit_event(&self, event: AgentEvent) {
        let _ = self.event_tx.send(Event::Agent(Arc::new(event)));
    }
}
