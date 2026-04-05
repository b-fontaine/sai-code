//! LLM provider port trait.

use async_trait::async_trait;

use crate::domain::message::{ContentBlock, Message, StopReason};
use crate::domain::tool_call::ToolCall;
use crate::error::LlmError;

/// A request to send to the language model.
#[derive(Debug, Clone)]
pub struct ChatRequest {
    /// System prompt (prepended to the conversation).
    pub system_prompt: Option<String>,
    /// Conversation messages.
    pub messages: Vec<Message>,
    /// Tool definitions available to the model.
    pub tool_definitions: Vec<serde_json::Value>,
}

impl ChatRequest {
    /// Create a new chat request with the given messages.
    pub fn new(messages: Vec<Message>) -> Self {
        Self {
            system_prompt: None,
            messages,
            tool_definitions: Vec::new(),
        }
    }

    /// Set the system prompt.
    #[must_use]
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Set the tool definitions.
    #[must_use]
    pub fn with_tools(mut self, tools: Vec<serde_json::Value>) -> Self {
        self.tool_definitions = tools;
        self
    }
}

/// A streaming event from the language model.
#[derive(Debug, Clone)]
pub enum ChatStreamEvent {
    /// The stream has started.
    StreamStart,
    /// A text token delta.
    TextDelta(String),
    /// A complete tool call (assembled from stream chunks by the adapter).
    ToolCallComplete(ToolCall),
    /// The stream has ended.
    StreamEnd {
        /// Why the model stopped generating.
        stop_reason: StopReason,
    },
}

/// A collected (non-streaming) response from the model.
#[derive(Debug, Clone)]
pub struct ChatResponse {
    /// Content blocks in the response.
    pub content: Vec<ContentBlock>,
    /// Why the model stopped generating.
    pub stop_reason: StopReason,
}

/// A stream of chat events from the model.
///
/// This is a boxed async stream that yields `ChatStreamEvent` items.
pub type ChatStream =
    std::pin::Pin<Box<dyn futures_core::Stream<Item = Result<ChatStreamEvent, LlmError>> + Send>>;

/// Port trait for language model interaction.
///
/// The agent loop uses this trait to communicate with any LLM provider.
/// Adapter crates (e.g., `sai-llm`) implement this trait.
#[async_trait]
#[cfg_attr(test, mockall::automock)]
pub trait LlmPort: Send + Sync {
    /// Send a chat request and receive a streaming response.
    async fn chat_stream(&self, request: ChatRequest) -> Result<ChatStream, LlmError>;

    /// Return the identifier of the active model.
    fn model_name(&self) -> &str;

    /// Return the name of the active provider.
    fn provider_name(&self) -> &str;
}
