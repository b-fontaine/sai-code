//! Core agent loop service.
//!
//! Orchestrates the LLM conversation cycle: assemble messages, call the
//! model, inspect the response for tool-use requests, dispatch tool calls,
//! append results, and loop until end-of-turn.

use crate::domain::config::AgentConfig;
use crate::domain::event::AgentEvent;
use crate::domain::message::{ContentBlock, Message, StopReason};
use crate::domain::session::{AgentSession, ConversationTurn, SessionMeta};
use crate::domain::tool_call::ToolCall;
use crate::error::{AgentError, LlmError};
use crate::ports::llm::{ChatRequest, ChatStreamEvent, LlmPort};
use crate::ports::permissions::PermissionPort;
use crate::ports::session::SessionPort;
use crate::ports::tool::ToolRegistryPort;
use crate::ports::ui::UiPort;
use crate::services::tool_executor::ToolExecutor;

use futures_core::Stream;
use std::pin::Pin;

/// The core agent loop.
///
/// Holds references to all port trait objects and drives the
/// conversation cycle.
pub struct AgentLoop<'a> {
    session: AgentSession,
    llm: &'a dyn LlmPort,
    tools: &'a dyn ToolRegistryPort,
    ui: &'a dyn UiPort,
    permissions: &'a dyn PermissionPort,
    session_port: &'a dyn SessionPort,
    /// Set to `true` after the first turn to avoid redundant `create_session` calls.
    session_created: bool,
    /// Track session metadata for persistence (model name, working dir).
    session_meta: Option<SessionMeta>,
}

/// The outcome of a single conversation turn.
#[derive(Debug)]
pub struct TurnResult {
    /// The final text response from the model, if any.
    pub text: Option<String>,
}

impl<'a> AgentLoop<'a> {
    /// Create a new agent loop with the given ports and configuration.
    pub fn new(
        config: AgentConfig,
        llm: &'a dyn LlmPort,
        tools: &'a dyn ToolRegistryPort,
        ui: &'a dyn UiPort,
        permissions: &'a dyn PermissionPort,
        session_port: &'a dyn SessionPort,
    ) -> Self {
        Self {
            session: AgentSession::new(config),
            llm,
            tools,
            ui,
            permissions,
            session_port,
            session_created: false,
            session_meta: None,
        }
    }

    /// Create an agent loop that resumes a prior persisted session.
    #[allow(clippy::too_many_arguments)]
    pub fn resume(
        config: AgentConfig,
        session_id: uuid::Uuid,
        prior_messages: Vec<Message>,
        llm: &'a dyn LlmPort,
        tools: &'a dyn ToolRegistryPort,
        ui: &'a dyn UiPort,
        permissions: &'a dyn PermissionPort,
        session_port: &'a dyn SessionPort,
    ) -> Self {
        Self {
            session: AgentSession::resume(config, session_id, prior_messages),
            llm,
            tools,
            ui,
            permissions,
            session_port,
            // Mark as already created so save_turn can run without create_session
            session_created: true,
            session_meta: None,
        }
    }

    /// Attach session metadata used when persisting sessions.
    #[must_use]
    pub fn with_session_meta(mut self, meta: SessionMeta) -> Self {
        self.session_meta = Some(meta);
        self
    }

    /// Run a single conversation turn.
    ///
    /// Sends the user message to the model, handles tool calls in a loop,
    /// and returns the final text response. On success, the completed turn
    /// is persisted via the `SessionPort`.
    pub async fn run_turn(&mut self, user_message: &str) -> Result<TurnResult, AgentError> {
        // Ensure the session is registered with the persistence layer
        if !self.session_created {
            if let Some(ref meta) = self.session_meta.clone() {
                let _ = self.session_port.create_session(meta.clone()).await;
            }
            self.session_created = true;
        }

        let turn_index = self.session.messages.len() / 2; // approximate
        let messages_before = self.session.messages.len();

        self.session.messages.push(Message::user(user_message));

        let mut iteration = 0;

        loop {
            let request = self.build_request();
            let stream = self.llm.chat_stream(request).await?;

            self.ui.emit_event(AgentEvent::StreamStart).await;

            let (text, tool_calls, stop_reason) = self.collect_stream(stream).await?;

            // Build the assistant message
            let mut content_blocks = Vec::new();
            if let Some(ref t) = text {
                content_blocks.push(ContentBlock::Text { text: t.clone() });
            }
            for tc in &tool_calls {
                content_blocks.push(ContentBlock::ToolUse {
                    id: tc.id.clone(),
                    name: tc.name.clone(),
                    input: tc.input.clone(),
                });
            }

            // Handle empty response (no text, no tools) as end-of-turn
            if content_blocks.is_empty() {
                self.ui.emit_event(AgentEvent::TurnComplete).await;
                self.check_history_size().await;
                self.persist_turn(turn_index, user_message, messages_before)
                    .await;
                return Ok(TurnResult { text: None });
            }

            self.session.messages.push(Message::Assistant {
                content: content_blocks,
                stop_reason: stop_reason.clone(),
            });

            if tool_calls.is_empty() {
                // No tool calls — end of turn
                self.ui.emit_event(AgentEvent::TurnComplete).await;
                self.check_history_size().await;
                self.persist_turn(turn_index, user_message, messages_before)
                    .await;
                return Ok(TurnResult { text });
            }

            // Execute tools
            iteration += 1;
            if iteration > self.session.config.max_iterations_per_turn {
                let limit = self.session.config.max_iterations_per_turn;
                self.ui
                    .emit_event(AgentEvent::Error(AgentError::IterationLimitExceeded {
                        limit,
                    }))
                    .await;
                return Err(AgentError::IterationLimitExceeded { limit });
            }

            let executor = ToolExecutor::new(
                self.tools,
                self.permissions,
                self.ui,
                self.session.config.max_parallel_tool_calls,
            );
            let results = executor.execute(tool_calls).await;

            // Append tool results to history
            for result in &results {
                self.session.messages.push(Message::ToolResult {
                    call_id: result.call_id.clone(),
                    status: result.status.clone(),
                    content: result.content.clone(),
                });
            }

            // Loop back to call the model again with tool results
        }
    }

    /// Persist a completed turn to the session storage layer.
    ///
    /// Failures are logged and swallowed — a persistence error MUST NOT
    /// interrupt the interactive session.
    async fn persist_turn(&self, turn_index: usize, user_message: &str, messages_before: usize) {
        let turn_messages = self.session.messages[messages_before..].to_vec();
        let turn = ConversationTurn {
            turn_index,
            user_message: user_message.to_string(),
            messages: turn_messages,
            completed_at: chrono::Utc::now(),
        };
        if let Err(e) = self.session_port.save_turn(self.session.id, turn).await {
            tracing::warn!(error = %e, "Failed to persist conversation turn");
        }
    }

    /// Build a `ChatRequest` from the current session state.
    fn build_request(&self) -> ChatRequest {
        let mut request = ChatRequest::new(self.session.messages.clone());
        let prompt = &self.session.config.system_prompt;
        if !prompt.is_empty() {
            request = request.with_system_prompt(prompt.clone());
        }
        request = request.with_tools(self.tools.tool_definitions());
        request
    }

    /// Collect a streaming response, emitting events and extracting
    /// text content and tool calls.
    async fn collect_stream(
        &self,
        mut stream: Pin<Box<dyn Stream<Item = Result<ChatStreamEvent, LlmError>> + Send>>,
    ) -> Result<(Option<String>, Vec<ToolCall>, StopReason), AgentError> {
        let mut text = String::new();
        let mut tool_calls = Vec::new();
        let mut stop_reason = StopReason::EndTurn;

        loop {
            let event = std::future::poll_fn(|cx| stream.as_mut().poll_next(cx)).await;

            match event {
                Some(Ok(ChatStreamEvent::StreamStart)) => {}
                Some(Ok(ChatStreamEvent::TextDelta(delta))) => {
                    self.ui
                        .emit_event(AgentEvent::TextDelta(delta.clone()))
                        .await;
                    text.push_str(&delta);
                }
                Some(Ok(ChatStreamEvent::ToolCallComplete(tc))) => {
                    self.ui
                        .emit_event(AgentEvent::ToolCallStart {
                            name: tc.name.clone(),
                            call_id: tc.id.clone(),
                        })
                        .await;
                    tool_calls.push(tc);
                }
                Some(Ok(ChatStreamEvent::StreamEnd { stop_reason: sr })) => {
                    stop_reason = sr;
                    break;
                }
                Some(Err(e)) => return Err(AgentError::Llm(e)),
                None => break,
            }
        }

        let text_opt = if text.is_empty() { None } else { Some(text) };
        Ok((text_opt, tool_calls, stop_reason))
    }

    /// Check history size and emit a warning if it exceeds the threshold.
    async fn check_history_size(&self) {
        let count = self.session.messages.len();
        if count > self.session.config.max_history_messages {
            self.ui
                .emit_event(AgentEvent::HistorySizeWarning {
                    message_count: count,
                })
                .await;
        }
    }

    /// Access the current session (for testing and inspection).
    pub fn session(&self) -> &AgentSession {
        &self.session
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ToolError;
    use crate::ports::llm::MockLlmPort;
    use crate::ports::permissions::{MockPermissionPort, PermissionDecision};
    use crate::ports::session::{MockSessionPort, NoOpSessionPort};
    use crate::ports::tool::{ToolOutput, ToolPort};
    use crate::ports::ui::MockUiPort;
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};

    // --- Mock Tool Registry ---

    struct MockToolRegistry {
        tools: Vec<Box<dyn ToolPort>>,
    }

    impl MockToolRegistry {
        fn new() -> Self {
            Self { tools: Vec::new() }
        }

        fn with_tool(mut self, tool: impl ToolPort + 'static) -> Self {
            self.tools.push(Box::new(tool));
            self
        }
    }

    impl ToolRegistryPort for MockToolRegistry {
        fn get(&self, name: &str) -> Option<&dyn ToolPort> {
            self.tools
                .iter()
                .find(|t| t.name() == name)
                .map(|t| t.as_ref())
        }

        fn list(&self) -> Vec<&dyn ToolPort> {
            self.tools.iter().map(|t| t.as_ref()).collect()
        }

        fn tool_definitions(&self) -> Vec<serde_json::Value> {
            self.tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "name": t.name(),
                        "description": t.description(),
                        "input_schema": t.input_schema(),
                    })
                })
                .collect()
        }
    }

    // --- Dummy Tool ---

    struct DummyTool {
        name: String,
        result: String,
        concurrency_safe: bool,
    }

    impl DummyTool {
        fn new(name: &str, result: &str) -> Self {
            Self {
                name: name.to_string(),
                result: result.to_string(),
                concurrency_safe: false,
            }
        }

        fn concurrency_safe(mut self) -> Self {
            self.concurrency_safe = true;
            self
        }
    }

    #[async_trait]
    impl ToolPort for DummyTool {
        fn name(&self) -> &str {
            &self.name
        }
        fn description(&self) -> &str {
            "A dummy tool for testing"
        }
        fn input_schema(&self) -> serde_json::Value {
            serde_json::json!({"type": "object"})
        }
        async fn execute(&self, _input: serde_json::Value) -> Result<ToolOutput, ToolError> {
            Ok(ToolOutput::Success(self.result.clone()))
        }
        fn is_concurrency_safe(&self) -> bool {
            self.concurrency_safe
        }
    }

    // --- Helper to create a stream from events ---

    fn make_stream(
        events: Vec<ChatStreamEvent>,
    ) -> Pin<Box<dyn Stream<Item = Result<ChatStreamEvent, LlmError>> + Send>> {
        let (tx, rx) = tokio::sync::mpsc::channel(events.len() + 1);
        tokio::spawn(async move {
            for event in events {
                let _ = tx.send(Ok(event)).await;
            }
        });
        Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx))
    }

    fn setup_ui() -> MockUiPort {
        let mut ui = MockUiPort::new();
        ui.expect_emit_event().returning(|_| Box::pin(async {}));
        ui
    }

    fn setup_permissions_allow() -> MockPermissionPort {
        let mut perms = MockPermissionPort::new();
        perms
            .expect_check()
            .returning(|_| Box::pin(async { PermissionDecision::Allow }));
        perms
    }

    fn setup_session_port() -> NoOpSessionPort {
        NoOpSessionPort
    }

    // ===== US1 Tests =====

    #[tokio::test]
    async fn us1_text_only_response_returns_text() {
        let mut llm = MockLlmPort::new();
        llm.expect_chat_stream().returning(|_| {
            Box::pin(async {
                Ok(make_stream(vec![
                    ChatStreamEvent::StreamStart,
                    ChatStreamEvent::TextDelta("Hello ".into()),
                    ChatStreamEvent::TextDelta("world!".into()),
                    ChatStreamEvent::StreamEnd {
                        stop_reason: StopReason::EndTurn,
                    },
                ]))
            })
        });
        llm.expect_model_name()
            .return_const("test-model".to_string());
        llm.expect_provider_name().return_const("test".to_string());

        let registry = MockToolRegistry::new();
        let ui = setup_ui();
        let perms = setup_permissions_allow();

        let session_port = setup_session_port();
        let mut agent = AgentLoop::new(
            AgentConfig::default(),
            &llm,
            &registry,
            &ui,
            &perms,
            &session_port,
        );
        let result = agent.run_turn("Hi").await.unwrap();

        assert_eq!(result.text, Some("Hello world!".to_string()));
    }

    #[tokio::test]
    async fn us1_streaming_emits_text_delta_events() {
        let events_received = Arc::new(Mutex::new(Vec::<String>::new()));
        let events_clone = events_received.clone();

        let mut llm = MockLlmPort::new();
        llm.expect_chat_stream().returning(|_| {
            Box::pin(async {
                Ok(make_stream(vec![
                    ChatStreamEvent::StreamStart,
                    ChatStreamEvent::TextDelta("one".into()),
                    ChatStreamEvent::TextDelta("two".into()),
                    ChatStreamEvent::TextDelta("three".into()),
                    ChatStreamEvent::StreamEnd {
                        stop_reason: StopReason::EndTurn,
                    },
                ]))
            })
        });
        llm.expect_model_name()
            .return_const("test-model".to_string());
        llm.expect_provider_name().return_const("test".to_string());

        let mut ui = MockUiPort::new();
        ui.expect_emit_event().returning(move |event| {
            let events_clone = events_clone.clone();
            Box::pin(async move {
                if let AgentEvent::TextDelta(text) = event {
                    events_clone.lock().unwrap().push(text);
                }
            })
        });

        let registry = MockToolRegistry::new();
        let perms = setup_permissions_allow();

        let session_port = setup_session_port();
        let mut agent = AgentLoop::new(
            AgentConfig::default(),
            &llm,
            &registry,
            &ui,
            &perms,
            &session_port,
        );
        agent.run_turn("Hi").await.unwrap();

        let deltas = events_received.lock().unwrap();
        assert_eq!(*deltas, vec!["one", "two", "three"]);
    }

    #[tokio::test]
    async fn us1_empty_response_returns_none() {
        let mut llm = MockLlmPort::new();
        llm.expect_chat_stream().returning(|_| {
            Box::pin(async {
                Ok(make_stream(vec![
                    ChatStreamEvent::StreamStart,
                    ChatStreamEvent::StreamEnd {
                        stop_reason: StopReason::EndTurn,
                    },
                ]))
            })
        });
        llm.expect_model_name()
            .return_const("test-model".to_string());
        llm.expect_provider_name().return_const("test".to_string());

        let registry = MockToolRegistry::new();
        let ui = setup_ui();
        let perms = setup_permissions_allow();

        let session_port = setup_session_port();
        let mut agent = AgentLoop::new(
            AgentConfig::default(),
            &llm,
            &registry,
            &ui,
            &perms,
            &session_port,
        );
        let result = agent.run_turn("Hi").await.unwrap();

        assert!(result.text.is_none());
    }

    // ===== US2 Tests =====

    #[tokio::test]
    async fn us2_tool_call_executes_and_returns_result() {
        let call_count = Arc::new(Mutex::new(0u32));
        let cc = call_count.clone();

        let mut llm = MockLlmPort::new();
        llm.expect_chat_stream().returning(move |_| {
            let cc = cc.clone();
            Box::pin(async move {
                let mut count = cc.lock().unwrap();
                *count += 1;
                if *count == 1 {
                    Ok(make_stream(vec![
                        ChatStreamEvent::StreamStart,
                        ChatStreamEvent::ToolCallComplete(ToolCall {
                            id: "call_1".into(),
                            name: "read_file".into(),
                            input: serde_json::json!({"path": "test.txt"}),
                        }),
                        ChatStreamEvent::StreamEnd {
                            stop_reason: StopReason::ToolUse,
                        },
                    ]))
                } else {
                    Ok(make_stream(vec![
                        ChatStreamEvent::StreamStart,
                        ChatStreamEvent::TextDelta("File content: hello".into()),
                        ChatStreamEvent::StreamEnd {
                            stop_reason: StopReason::EndTurn,
                        },
                    ]))
                }
            })
        });
        llm.expect_model_name()
            .return_const("test-model".to_string());
        llm.expect_provider_name().return_const("test".to_string());

        let registry = MockToolRegistry::new().with_tool(DummyTool::new("read_file", "hello"));
        let ui = setup_ui();
        let perms = setup_permissions_allow();

        let session_port = setup_session_port();
        let mut agent = AgentLoop::new(
            AgentConfig::default(),
            &llm,
            &registry,
            &ui,
            &perms,
            &session_port,
        );
        let result = agent.run_turn("Read test.txt").await.unwrap();

        assert_eq!(result.text, Some("File content: hello".to_string()));
    }

    #[tokio::test]
    async fn us2_unknown_tool_returns_error_result() {
        let call_count = Arc::new(Mutex::new(0u32));
        let cc = call_count.clone();

        let mut llm = MockLlmPort::new();
        llm.expect_chat_stream().returning(move |_| {
            let cc = cc.clone();
            Box::pin(async move {
                let mut count = cc.lock().unwrap();
                *count += 1;
                if *count == 1 {
                    Ok(make_stream(vec![
                        ChatStreamEvent::StreamStart,
                        ChatStreamEvent::ToolCallComplete(ToolCall {
                            id: "call_1".into(),
                            name: "nonexistent".into(),
                            input: serde_json::json!({}),
                        }),
                        ChatStreamEvent::StreamEnd {
                            stop_reason: StopReason::ToolUse,
                        },
                    ]))
                } else {
                    Ok(make_stream(vec![
                        ChatStreamEvent::StreamStart,
                        ChatStreamEvent::TextDelta("I see that tool failed.".into()),
                        ChatStreamEvent::StreamEnd {
                            stop_reason: StopReason::EndTurn,
                        },
                    ]))
                }
            })
        });
        llm.expect_model_name()
            .return_const("test-model".to_string());
        llm.expect_provider_name().return_const("test".to_string());

        let registry = MockToolRegistry::new(); // no tools registered
        let ui = setup_ui();
        let perms = setup_permissions_allow();

        let session_port = setup_session_port();
        let mut agent = AgentLoop::new(
            AgentConfig::default(),
            &llm,
            &registry,
            &ui,
            &perms,
            &session_port,
        );
        // Should NOT panic — error is sent back to model
        let result = agent.run_turn("Use nonexistent").await.unwrap();
        assert!(result.text.is_some());
    }

    #[tokio::test]
    async fn us2_permission_deny_sends_error_to_model() {
        let call_count = Arc::new(Mutex::new(0u32));
        let cc = call_count.clone();

        let mut llm = MockLlmPort::new();
        llm.expect_chat_stream().returning(move |_| {
            let cc = cc.clone();
            Box::pin(async move {
                let mut count = cc.lock().unwrap();
                *count += 1;
                if *count == 1 {
                    Ok(make_stream(vec![
                        ChatStreamEvent::StreamStart,
                        ChatStreamEvent::ToolCallComplete(ToolCall {
                            id: "call_1".into(),
                            name: "dangerous_tool".into(),
                            input: serde_json::json!({}),
                        }),
                        ChatStreamEvent::StreamEnd {
                            stop_reason: StopReason::ToolUse,
                        },
                    ]))
                } else {
                    Ok(make_stream(vec![
                        ChatStreamEvent::StreamStart,
                        ChatStreamEvent::TextDelta("Permission denied.".into()),
                        ChatStreamEvent::StreamEnd {
                            stop_reason: StopReason::EndTurn,
                        },
                    ]))
                }
            })
        });
        llm.expect_model_name()
            .return_const("test-model".to_string());
        llm.expect_provider_name().return_const("test".to_string());

        let registry =
            MockToolRegistry::new().with_tool(DummyTool::new("dangerous_tool", "should not run"));
        let ui = setup_ui();

        let mut perms = MockPermissionPort::new();
        perms
            .expect_check()
            .returning(|_| Box::pin(async { PermissionDecision::Deny("not allowed".into()) }));

        let session_port = setup_session_port();
        let mut agent = AgentLoop::new(
            AgentConfig::default(),
            &llm,
            &registry,
            &ui,
            &perms,
            &session_port,
        );
        let result = agent.run_turn("Do dangerous thing").await.unwrap();
        assert!(result.text.is_some());
    }

    // ===== US3 Tests =====

    #[tokio::test]
    async fn us3_multi_tool_chain_completes() {
        let call_count = Arc::new(Mutex::new(0u32));
        let cc = call_count.clone();

        let mut llm = MockLlmPort::new();
        llm.expect_chat_stream().returning(move |_| {
            let cc = cc.clone();
            Box::pin(async move {
                let mut count = cc.lock().unwrap();
                *count += 1;
                if *count <= 3 {
                    Ok(make_stream(vec![
                        ChatStreamEvent::StreamStart,
                        ChatStreamEvent::ToolCallComplete(ToolCall {
                            id: format!("call_{count}"),
                            name: "read_file".into(),
                            input: serde_json::json!({}),
                        }),
                        ChatStreamEvent::StreamEnd {
                            stop_reason: StopReason::ToolUse,
                        },
                    ]))
                } else {
                    Ok(make_stream(vec![
                        ChatStreamEvent::StreamStart,
                        ChatStreamEvent::TextDelta("Done after 3 tools.".into()),
                        ChatStreamEvent::StreamEnd {
                            stop_reason: StopReason::EndTurn,
                        },
                    ]))
                }
            })
        });
        llm.expect_model_name()
            .return_const("test-model".to_string());
        llm.expect_provider_name().return_const("test".to_string());

        let registry = MockToolRegistry::new().with_tool(DummyTool::new("read_file", "content"));
        let ui = setup_ui();
        let perms = setup_permissions_allow();

        let session_port = setup_session_port();
        let mut agent = AgentLoop::new(
            AgentConfig::default(),
            &llm,
            &registry,
            &ui,
            &perms,
            &session_port,
        );
        let result = agent.run_turn("Do 3 things").await.unwrap();

        assert_eq!(result.text, Some("Done after 3 tools.".to_string()));
    }

    #[tokio::test]
    async fn us3_iteration_limit_exceeded() {
        let mut llm = MockLlmPort::new();
        llm.expect_chat_stream().returning(|_| {
            Box::pin(async {
                Ok(make_stream(vec![
                    ChatStreamEvent::StreamStart,
                    ChatStreamEvent::ToolCallComplete(ToolCall {
                        id: "call_x".into(),
                        name: "looper".into(),
                        input: serde_json::json!({}),
                    }),
                    ChatStreamEvent::StreamEnd {
                        stop_reason: StopReason::ToolUse,
                    },
                ]))
            })
        });
        llm.expect_model_name()
            .return_const("test-model".to_string());
        llm.expect_provider_name().return_const("test".to_string());

        let registry = MockToolRegistry::new().with_tool(DummyTool::new("looper", "looped"));
        let ui = setup_ui();
        let perms = setup_permissions_allow();

        let mut config = AgentConfig::default();
        config.max_iterations_per_turn = 3;
        let session_port = setup_session_port();
        let mut agent = AgentLoop::new(config, &llm, &registry, &ui, &perms, &session_port);
        let result = agent.run_turn("Loop forever").await;

        assert!(matches!(
            result,
            Err(AgentError::IterationLimitExceeded { limit: 3 })
        ));
    }

    // ===== US5 Tests =====

    #[tokio::test]
    async fn us5_history_preserved_across_turns() {
        let call_count = Arc::new(Mutex::new(0u32));
        let cc = call_count.clone();

        let mut llm = MockLlmPort::new();
        llm.expect_chat_stream().returning(move |req: ChatRequest| {
            let cc = cc.clone();
            Box::pin(async move {
                let mut count = cc.lock().unwrap();
                *count += 1;
                // On second call, verify history includes first turn
                if *count == 2 {
                    assert!(
                        req.messages.len() >= 2,
                        "Second turn should include messages from first turn, got {}",
                        req.messages.len()
                    );
                }
                Ok(make_stream(vec![
                    ChatStreamEvent::StreamStart,
                    ChatStreamEvent::TextDelta(format!("Response {count}")),
                    ChatStreamEvent::StreamEnd {
                        stop_reason: StopReason::EndTurn,
                    },
                ]))
            })
        });
        llm.expect_model_name()
            .return_const("test-model".to_string());
        llm.expect_provider_name().return_const("test".to_string());

        let registry = MockToolRegistry::new();
        let ui = setup_ui();
        let perms = setup_permissions_allow();

        let session_port = setup_session_port();
        let mut agent = AgentLoop::new(
            AgentConfig::default(),
            &llm,
            &registry,
            &ui,
            &perms,
            &session_port,
        );

        agent.run_turn("First message").await.unwrap();
        agent.run_turn("Second message").await.unwrap();

        assert!(agent.session().messages.len() >= 4); // 2 user + 2 assistant
    }

    #[tokio::test]
    async fn us5_history_size_warning_emitted() {
        let warning_received = Arc::new(Mutex::new(false));
        let wr = warning_received.clone();

        let mut llm = MockLlmPort::new();
        llm.expect_chat_stream().returning(|_| {
            Box::pin(async {
                Ok(make_stream(vec![
                    ChatStreamEvent::StreamStart,
                    ChatStreamEvent::TextDelta("ok".into()),
                    ChatStreamEvent::StreamEnd {
                        stop_reason: StopReason::EndTurn,
                    },
                ]))
            })
        });
        llm.expect_model_name()
            .return_const("test-model".to_string());
        llm.expect_provider_name().return_const("test".to_string());

        let mut ui = MockUiPort::new();
        ui.expect_emit_event().returning(move |event| {
            let wr = wr.clone();
            Box::pin(async move {
                if matches!(event, AgentEvent::HistorySizeWarning { .. }) {
                    *wr.lock().unwrap() = true;
                }
            })
        });

        let registry = MockToolRegistry::new();
        let perms = setup_permissions_allow();

        let mut config = AgentConfig::default();
        config.max_history_messages = 1; // trigger warning after 1 message

        let session_port = setup_session_port();
        let mut agent = AgentLoop::new(config, &llm, &registry, &ui, &perms, &session_port);
        agent.run_turn("Fill history").await.unwrap();

        assert!(*warning_received.lock().unwrap());
    }
}
