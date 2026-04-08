//! `LlmPort` implementation using the genai crate.

use async_trait::async_trait;
use futures::StreamExt;

use genai::chat::ChatOptions;
use genai::Client;

use sai_core::error::LlmError;
use sai_core::ports::llm::{ChatRequest, ChatStream, ChatStreamEvent, LlmPort};

use crate::convert;
use crate::provider;

/// Multi-provider LLM adapter backed by the `genai` crate.
///
/// Implements `LlmPort` from `sai-core`, routing requests to the correct
/// provider based on the model identifier string. All provider-specific
/// details (message format, tool calling, error types) are normalized
/// internally.
///
/// # Example
///
/// ```no_run
/// use sai_llm::GenaiLlmAdapter;
/// use sai_core::ports::llm::LlmPort;
///
/// let adapter = GenaiLlmAdapter::new("claude-sonnet-4").unwrap();
/// assert_eq!(adapter.provider_name(), "anthropic");
/// ```
pub struct GenaiLlmAdapter {
    client: Client,
    model_name: String,
    provider: String,
}

impl GenaiLlmAdapter {
    /// Create a new adapter for the given model.
    ///
    /// The provider is auto-detected from the model name prefix.
    /// Returns an error if the model prefix is unrecognized.
    pub fn new(model_name: &str) -> Result<Self, LlmError> {
        let provider = provider::provider_for_model(model_name)?;
        Ok(Self {
            client: Client::default(),
            model_name: model_name.to_string(),
            provider: provider.to_string(),
        })
    }

    /// Change the active model at runtime.
    ///
    /// The provider is re-derived from the new model name.
    pub fn set_model(&mut self, model_name: &str) -> Result<(), LlmError> {
        let provider = provider::provider_for_model(model_name)?;
        self.model_name = model_name.to_string();
        self.provider = provider.to_string();
        Ok(())
    }
}

#[async_trait]
impl LlmPort for GenaiLlmAdapter {
    async fn chat_stream(&self, request: ChatRequest) -> Result<ChatStream, LlmError> {
        // Pre-flight: check API key
        provider::check_api_key(&self.model_name)?;

        // Convert sai-core request to genai request
        let genai_req = convert::request::to_genai_request(&request);

        // Configure options: capture tool calls from the stream
        let options = ChatOptions::default()
            .with_capture_content(true)
            .with_capture_tool_calls(true);

        // Call genai
        let stream_response = self
            .client
            .exec_chat_stream(&self.model_name, genai_req, Some(&options))
            .await
            .map_err(convert::errors::from_genai_error)?;

        // Wrap the genai stream into a sai-core ChatStream
        let genai_stream = stream_response.stream;
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<ChatStreamEvent, LlmError>>(64);

        tokio::spawn(async move {
            let mut stream = genai_stream;
            while let Some(event_result) = stream.next().await {
                match event_result {
                    Ok(genai_event) => {
                        let sai_events = convert::response::convert_stream_event(genai_event);
                        for sai_event in sai_events {
                            if tx.send(Ok(sai_event)).await.is_err() {
                                return; // receiver dropped
                            }
                        }
                    }
                    Err(genai_err) => {
                        let llm_err = convert::errors::from_genai_error(genai_err);
                        let _ = tx.send(Err(llm_err)).await;
                        return;
                    }
                }
            }
        });

        Ok(Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }

    fn provider_name(&self) -> &str {
        &self.provider
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_detects_anthropic() {
        let adapter = GenaiLlmAdapter::new("claude-sonnet-4").unwrap();
        assert_eq!(adapter.model_name(), "claude-sonnet-4");
        assert_eq!(adapter.provider_name(), "anthropic");
    }

    #[test]
    fn new_detects_openai() {
        let adapter = GenaiLlmAdapter::new("gpt-4o").unwrap();
        assert_eq!(adapter.provider_name(), "openai");
    }

    #[test]
    fn new_detects_gemini() {
        let adapter = GenaiLlmAdapter::new("gemini-2.0-flash").unwrap();
        assert_eq!(adapter.provider_name(), "gemini");
    }

    #[test]
    fn new_detects_ollama() {
        let adapter = GenaiLlmAdapter::new("ollama::llama3").unwrap();
        assert_eq!(adapter.provider_name(), "ollama");
    }

    #[test]
    fn new_rejects_unknown_model() {
        let result = GenaiLlmAdapter::new("xyz-unknown");
        assert!(result.is_err());
    }

    #[test]
    fn set_model_switches_provider() {
        let mut adapter = GenaiLlmAdapter::new("claude-sonnet-4").unwrap();
        assert_eq!(adapter.provider_name(), "anthropic");

        adapter.set_model("gpt-4o").unwrap();
        assert_eq!(adapter.model_name(), "gpt-4o");
        assert_eq!(adapter.provider_name(), "openai");
    }

    #[test]
    fn set_model_rejects_unknown() {
        let mut adapter = GenaiLlmAdapter::new("claude-sonnet-4").unwrap();
        let result = adapter.set_model("xyz-unknown");
        assert!(result.is_err());
        // Original model should be unchanged
        assert_eq!(adapter.model_name(), "claude-sonnet-4");
    }
}
