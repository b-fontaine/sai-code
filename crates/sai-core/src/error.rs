//! Error types for the agent loop domain.

/// Errors that can occur during agent loop execution.
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    /// The requested tool was not found in the registry.
    #[error("tool '{name}' not found in registry")]
    ToolNotFound {
        /// Name of the tool that was requested.
        name: String,
    },

    /// Tool execution failed.
    #[error("tool '{name}' execution failed: {reason}")]
    ToolExecutionFailed {
        /// Name of the tool that failed.
        name: String,
        /// Description of the failure.
        reason: String,
    },

    /// An LLM provider error occurred.
    #[error("LLM error: {0}")]
    Llm(#[from] LlmError),

    /// The iteration limit for a single turn was exceeded.
    #[error("iteration limit exceeded ({limit} iterations)")]
    IterationLimitExceeded {
        /// The configured maximum iterations.
        limit: usize,
    },

    /// The event channel was closed unexpectedly.
    #[error("event channel closed")]
    ChannelClosed,
}

/// Errors originating from the LLM provider layer.
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    /// Connection to the provider failed.
    #[error("connection failed: {0}")]
    Connection(String),

    /// The provider rate-limited the request.
    #[error("rate limited, retry after {retry_after_secs}s")]
    RateLimited {
        /// Seconds to wait before retrying.
        retry_after_secs: u64,
    },

    /// The request exceeded the provider's token limit.
    #[error("token limit exceeded")]
    TokenLimitExceeded,

    /// A provider-specific error.
    #[error("provider error: {0}")]
    Provider(String),
}

/// Errors from tool execution.
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    /// The tool encountered an error during execution.
    #[error("{0}")]
    Execution(String),

    /// The tool's input was invalid.
    #[error("invalid input: {0}")]
    InvalidInput(String),
}

/// Errors from the session persistence layer.
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    /// No session exists with the given ID.
    #[error("session not found: {id}")]
    NotFound {
        /// The session UUID that was not found.
        id: uuid::Uuid,
    },

    /// A session with the requested name already exists.
    #[error("session name conflict: '{name}' is already in use")]
    NameConflict {
        /// The conflicting name.
        name: String,
    },

    /// The session data on disk is unreadable or structurally invalid.
    #[error("session {id} is corrupted: {reason}")]
    Corrupted {
        /// The session UUID whose data is corrupted.
        id: uuid::Uuid,
        /// Human-readable description of what is wrong.
        reason: String,
    },

    /// An underlying I/O error occurred.
    #[error("session I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// A JSON serialization or deserialization error occurred.
    #[error("session serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl LlmError {
    /// Returns true if this error is transient and the request can be retried.
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Connection(_) | Self::RateLimited { .. })
    }

    /// Returns the suggested wait time before retrying, if applicable.
    pub fn retry_after(&self) -> Option<std::time::Duration> {
        match self {
            Self::RateLimited { retry_after_secs } => {
                Some(std::time::Duration::from_secs(*retry_after_secs))
            }
            _ => None,
        }
    }
}
