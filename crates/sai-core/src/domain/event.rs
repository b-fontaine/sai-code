//! Agent events emitted by the agent loop for the UI layer.

use crate::error::AgentError;

/// Events emitted during agent loop execution.
///
/// These events are sent through a channel to the UI layer for
/// real-time display of the agent's activity.
#[derive(Debug)]
pub enum AgentEvent {
    /// A new streaming response has started.
    StreamStart,

    /// A text token delta arrived from the model.
    TextDelta(String),

    /// A tool call is about to be executed.
    ToolCallStart {
        /// The name of the tool being called.
        name: String,
        /// The unique identifier for this call.
        call_id: String,
    },

    /// A tool call has completed.
    ToolCallComplete {
        /// The unique identifier for this call.
        call_id: String,
        /// Whether the tool execution succeeded.
        success: bool,
        /// A brief summary of the result.
        summary: String,
    },

    /// The current turn has completed (final response delivered).
    TurnComplete,

    /// An error occurred during the turn.
    Error(AgentError),

    /// The conversation history is large and may need compression.
    HistorySizeWarning {
        /// Current number of messages in history.
        message_count: usize,
    },
}
