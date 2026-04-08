//! Agent session state and persistence types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
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

    /// Create a session pre-loaded with conversation history from a prior persisted session.
    pub fn resume(config: AgentConfig, id: Uuid, messages: Vec<Message>) -> Self {
        Self {
            id,
            config,
            messages,
        }
    }
}

// ── Persistence types ─────────────────────────────────────────────────────────

/// Lightweight summary of a session, used for listing without loading full turn data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    /// Unique session identifier (UUID v4).
    pub id: Uuid,
    /// User-provided display name, or `None` if unnamed.
    pub name: Option<String>,
    /// LLM model active when the session was created.
    pub model_name: String,
    /// Absolute path of the working directory at session start.
    pub working_dir: std::path::PathBuf,
    /// When the session was first created.
    pub created_at: DateTime<Utc>,
    /// When the last turn was persisted.
    pub last_active_at: DateTime<Utc>,
    /// Number of completed turns saved.
    pub turn_count: usize,
}

impl SessionMeta {
    /// Create new metadata for a fresh session.
    pub fn new(id: Uuid, model_name: String, working_dir: std::path::PathBuf) -> Self {
        let now = Utc::now();
        Self {
            id,
            name: None,
            model_name,
            working_dir,
            created_at: now,
            last_active_at: now,
            turn_count: 0,
        }
    }
}

/// A single completed conversation exchange, stored in the turns file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    /// Zero-based index of this turn in the session.
    pub turn_index: usize,
    /// Verbatim text the user submitted.
    pub user_message: String,
    /// All messages generated during this turn (`User` + `Assistant` + `ToolResult`s).
    pub messages: Vec<Message>,
    /// When `run_turn` returned successfully for this turn.
    pub completed_at: DateTime<Utc>,
}

/// The full, loadable representation of a saved session.
#[derive(Debug, Clone)]
pub struct PersistedSession {
    /// Session metadata.
    pub meta: SessionMeta,
    /// All completed turns in order.
    pub turns: Vec<ConversationTurn>,
}

impl PersistedSession {
    /// Reconstruct the flat message history by flattening all turns.
    ///
    /// This produces the `Vec<Message>` to inject into `AgentSession::messages`
    /// when resuming a session.
    pub fn into_messages(self) -> Vec<Message> {
        self.turns.into_iter().flat_map(|t| t.messages).collect()
    }
}
