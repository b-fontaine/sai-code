//! Tool call and result types.

use serde::{Deserialize, Serialize};

/// A structured request from the model to invoke a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique identifier for this call (must be echoed in the result).
    pub id: String,
    /// The name of the tool to look up in the registry.
    pub name: String,
    /// The arguments to pass to the tool, as JSON.
    pub input: serde_json::Value,
}

/// The output from executing a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// The ID of the tool call this result corresponds to.
    pub call_id: String,
    /// Whether the execution succeeded or failed.
    pub status: ToolResultStatus,
    /// The result content (output text or error message).
    pub content: String,
}

/// Whether a tool execution succeeded or failed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ToolResultStatus {
    /// The tool executed successfully.
    Success,
    /// The tool execution failed.
    Error,
}

impl ToolResult {
    /// Create a successful tool result.
    pub fn success(call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            call_id: call_id.into(),
            status: ToolResultStatus::Success,
            content: content.into(),
        }
    }

    /// Create an error tool result.
    pub fn error(call_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            call_id: call_id.into(),
            status: ToolResultStatus::Error,
            content: message.into(),
        }
    }
}
