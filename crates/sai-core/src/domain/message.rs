//! Conversation message types.

use serde::{Deserialize, Serialize};

use super::tool_call::ToolResultStatus;

/// A single entry in the conversation history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    /// A message from the user.
    User {
        /// The text content of the user's message.
        content: String,
    },

    /// A response from the language model.
    Assistant {
        /// Content blocks (text and/or tool-use requests).
        content: Vec<ContentBlock>,
        /// Why the model stopped generating.
        stop_reason: StopReason,
    },

    /// The result of executing a tool.
    ToolResult {
        /// The ID of the tool call this result corresponds to.
        call_id: String,
        /// Whether the tool execution succeeded or failed.
        status: ToolResultStatus,
        /// The result content (output text or error message).
        content: String,
    },
}

impl Message {
    /// Create a new user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self::User {
            content: content.into(),
        }
    }

    /// Create a new assistant message with text-only content.
    pub fn assistant_text(text: impl Into<String>) -> Self {
        Self::Assistant {
            content: vec![ContentBlock::Text {
                text: text.into(),
            }],
            stop_reason: StopReason::EndTurn,
        }
    }
}

/// A single block within an assistant message.
///
/// The model response can contain interleaved text and tool-use blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentBlock {
    /// A text segment of the response.
    Text {
        /// The text content.
        text: String,
    },

    /// A request to invoke a tool.
    ToolUse {
        /// Unique identifier for this tool call.
        id: String,
        /// The name of the tool to invoke.
        name: String,
        /// The arguments to pass to the tool, as JSON.
        input: serde_json::Value,
    },
}

/// Why the model stopped generating its response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StopReason {
    /// The model finished naturally, no tool calls requested.
    EndTurn,
    /// The model wants one or more tools executed.
    ToolUse,
    /// The response was truncated at the token limit.
    MaxTokens,
    /// A provider-specific stop reason.
    Unknown(String),
}
