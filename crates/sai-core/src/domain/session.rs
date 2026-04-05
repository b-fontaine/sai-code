//! Agent session state.

use uuid::Uuid;

use super::config::AgentConfig;
use super::message::Message;

/// The top-level stateful context for an agent conversation.
#[derive(Debug)]
pub struct AgentSession {
    /// Unique session identifier.
    pub id: Uuid,
    /// Agent configuration for this session.
    pub config: AgentConfig,
    /// Full conversation history (all turns).
    pub messages: Vec<Message>,
}

impl AgentSession {
    /// Create a new session with the given configuration.
    pub fn new(config: AgentConfig) -> Self {
        Self {
            id: Uuid::new_v4(),
            config,
            messages: Vec::new(),
        }
    }
}
