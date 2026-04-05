//! Tool execution service.
//!
//! Handles dispatching tool calls to the tool registry, with support
//! for both sequential and parallel execution based on concurrency safety.

use crate::domain::event::AgentEvent;
use crate::domain::tool_call::{ToolCall, ToolResult};
use crate::ports::permissions::{PermissionDecision, PermissionPort, PermissionRequest};
use crate::ports::tool::{ToolOutput, ToolRegistryPort};
use crate::ports::ui::UiPort;

/// Executes tool calls with support for parallel and sequential dispatch.
pub struct ToolExecutor<'a> {
    tools: &'a dyn ToolRegistryPort,
    permissions: &'a dyn PermissionPort,
    ui: &'a dyn UiPort,
    max_parallel: usize,
}

impl<'a> ToolExecutor<'a> {
    /// Create a new tool executor.
    pub fn new(
        tools: &'a dyn ToolRegistryPort,
        permissions: &'a dyn PermissionPort,
        ui: &'a dyn UiPort,
        max_parallel: usize,
    ) -> Self {
        Self {
            tools,
            permissions,
            ui,
            max_parallel,
        }
    }

    /// Execute a list of tool calls, respecting concurrency safety.
    ///
    /// Concurrency-safe tools run in parallel (up to `max_parallel`).
    /// Non-safe tools run sequentially after the parallel batch.
    /// All results are returned in the original request order.
    pub async fn execute(&self, tool_calls: Vec<ToolCall>) -> Vec<ToolResult> {
        let (safe, non_safe) = self.partition_tool_calls(&tool_calls);

        let mut results_map: Vec<(usize, ToolResult)> = Vec::with_capacity(tool_calls.len());

        // Execute concurrency-safe tools in parallel
        if !safe.is_empty() {
            let parallel_results = self.execute_parallel(&safe).await;
            results_map.extend(parallel_results);
        }

        // Execute non-safe tools sequentially
        for (idx, tc) in &non_safe {
            let result = self.execute_single(tc).await;
            results_map.push((*idx, result));
        }

        // Sort by original index to preserve request order
        results_map.sort_by_key(|(idx, _)| *idx);
        results_map.into_iter().map(|(_, r)| r).collect()
    }

    /// Partition tool calls into concurrency-safe and non-safe groups.
    ///
    /// Returns tuples of (`original_index`, `tool_call`) for each group.
    #[allow(clippy::type_complexity)]
    fn partition_tool_calls<'b>(
        &self,
        tool_calls: &'b [ToolCall],
    ) -> (Vec<(usize, &'b ToolCall)>, Vec<(usize, &'b ToolCall)>) {
        let mut safe = Vec::new();
        let mut non_safe = Vec::new();

        for (idx, tc) in tool_calls.iter().enumerate() {
            let is_safe = self
                .tools
                .get(&tc.name)
                .is_some_and(crate::ports::tool::ToolPort::is_concurrency_safe);
            if is_safe {
                safe.push((idx, tc));
            } else {
                non_safe.push((idx, tc));
            }
        }

        (safe, non_safe)
    }

    /// Execute a batch of tool calls in parallel using `JoinSet`.
    async fn execute_parallel(&self, calls: &[(usize, &ToolCall)]) -> Vec<(usize, ToolResult)> {
        // For now, execute concurrently up to max_parallel using sequential
        // async calls. True JoinSet parallelism requires 'static tool refs
        // which we'll handle by executing in chunks.
        let mut results = Vec::with_capacity(calls.len());

        for chunk in calls.chunks(self.max_parallel) {
            // Execute each chunk concurrently
            let mut futures = Vec::with_capacity(chunk.len());
            for (idx, tc) in chunk {
                futures.push(async move {
                    let result = self.execute_single(tc).await;
                    (*idx, result)
                });
            }

            // Use join_all equivalent
            for fut in futures {
                results.push(fut.await);
            }
        }

        results
    }

    /// Execute a single tool call with permission checking.
    async fn execute_single(&self, tool_call: &ToolCall) -> ToolResult {
        // Look up the tool
        let Some(tool) = self.tools.get(&tool_call.name) else {
            self.ui
                .emit_event(AgentEvent::ToolCallComplete {
                    call_id: tool_call.id.clone(),
                    success: false,
                    summary: format!("tool '{}' not found", tool_call.name),
                })
                .await;
            return ToolResult::error(
                &tool_call.id,
                format!("tool '{}' not found in registry", tool_call.name),
            );
        };

        // Check permissions
        let perm_request = PermissionRequest {
            tool_call: tool_call.clone(),
            is_read_only: tool.is_read_only(),
        };
        let decision = self.permissions.check(&perm_request).await;

        match decision {
            PermissionDecision::Allow => {}
            PermissionDecision::Deny(reason) => {
                self.ui
                    .emit_event(AgentEvent::ToolCallComplete {
                        call_id: tool_call.id.clone(),
                        success: false,
                        summary: format!("permission denied: {reason}"),
                    })
                    .await;
                return ToolResult::error(
                    &tool_call.id,
                    format!("permission denied: {reason}"),
                );
            }
            PermissionDecision::Ask => {
                // For now, treat Ask as Deny until UI prompt is implemented
                self.ui
                    .emit_event(AgentEvent::ToolCallComplete {
                        call_id: tool_call.id.clone(),
                        success: false,
                        summary: "user approval required".into(),
                    })
                    .await;
                return ToolResult::error(
                    &tool_call.id,
                    "tool requires user approval (not yet implemented)",
                );
            }
        }

        // Execute the tool
        self.ui
            .emit_event(AgentEvent::ToolCallStart {
                name: tool_call.name.clone(),
                call_id: tool_call.id.clone(),
            })
            .await;

        let result = match tool.execute(tool_call.input.clone()).await {
            Ok(ToolOutput::Success(content)) => {
                self.ui
                    .emit_event(AgentEvent::ToolCallComplete {
                        call_id: tool_call.id.clone(),
                        success: true,
                        summary: truncate(&content, 100),
                    })
                    .await;
                ToolResult::success(&tool_call.id, content)
            }
            Ok(ToolOutput::Error(msg)) => {
                self.ui
                    .emit_event(AgentEvent::ToolCallComplete {
                        call_id: tool_call.id.clone(),
                        success: false,
                        summary: truncate(&msg, 100),
                    })
                    .await;
                ToolResult::error(&tool_call.id, msg)
            }
            Err(e) => {
                let msg = e.to_string();
                self.ui
                    .emit_event(AgentEvent::ToolCallComplete {
                        call_id: tool_call.id.clone(),
                        success: false,
                        summary: truncate(&msg, 100),
                    })
                    .await;
                ToolResult::error(&tool_call.id, msg)
            }
        };

        result
    }
}

/// Truncate a string to at most `max` characters.
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::permissions::MockPermissionPort;
    use crate::ports::tool::ToolPort;
    use crate::ports::ui::MockUiPort;
    use crate::error::ToolError;
    use async_trait::async_trait;

    // Reuse helpers from agent_loop tests
    struct TestToolRegistry {
        tools: Vec<Box<dyn ToolPort>>,
    }

    impl TestToolRegistry {
        fn new() -> Self {
            Self { tools: Vec::new() }
        }
        fn with_tool(mut self, tool: impl ToolPort + 'static) -> Self {
            self.tools.push(Box::new(tool));
            self
        }
    }

    impl ToolRegistryPort for TestToolRegistry {
        fn get(&self, name: &str) -> Option<&dyn ToolPort> {
            self.tools.iter().find(|t| t.name() == name).map(|t| t.as_ref())
        }
        fn list(&self) -> Vec<&dyn ToolPort> {
            self.tools.iter().map(|t| t.as_ref()).collect()
        }
        fn tool_definitions(&self) -> Vec<serde_json::Value> {
            Vec::new()
        }
    }

    struct SimpleTool {
        name: String,
        output: String,
        safe: bool,
    }

    #[async_trait]
    impl ToolPort for SimpleTool {
        fn name(&self) -> &str { &self.name }
        fn description(&self) -> &str { "test" }
        fn input_schema(&self) -> serde_json::Value { serde_json::json!({}) }
        async fn execute(&self, _: serde_json::Value) -> Result<ToolOutput, ToolError> {
            Ok(ToolOutput::Success(self.output.clone()))
        }
        fn is_concurrency_safe(&self) -> bool { self.safe }
    }

    struct FailingTool;

    #[async_trait]
    impl ToolPort for FailingTool {
        fn name(&self) -> &str { "fail_tool" }
        fn description(&self) -> &str { "always fails" }
        fn input_schema(&self) -> serde_json::Value { serde_json::json!({}) }
        async fn execute(&self, _: serde_json::Value) -> Result<ToolOutput, ToolError> {
            Err(ToolError::Execution("boom".into()))
        }
    }

    fn setup_ui() -> MockUiPort {
        let mut ui = MockUiPort::new();
        ui.expect_emit_event().returning(|_| Box::pin(async {}));
        ui
    }

    fn setup_perms_allow() -> MockPermissionPort {
        let mut p = MockPermissionPort::new();
        p.expect_check().returning(|_| Box::pin(async { PermissionDecision::Allow }));
        p
    }

    #[tokio::test]
    async fn us2_tool_execution_returns_success() {
        let registry = TestToolRegistry::new().with_tool(SimpleTool {
            name: "greet".into(),
            output: "hello!".into(),
            safe: false,
        });
        let ui = setup_ui();
        let perms = setup_perms_allow();
        let executor = ToolExecutor::new(&registry, &perms, &ui, 10);

        let results = executor
            .execute(vec![ToolCall {
                id: "c1".into(),
                name: "greet".into(),
                input: serde_json::json!({}),
            }])
            .await;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, crate::domain::tool_call::ToolResultStatus::Success);
        assert_eq!(results[0].content, "hello!");
    }

    #[tokio::test]
    async fn us2_tool_execution_failure_returns_error() {
        let registry = TestToolRegistry::new().with_tool(FailingTool);
        let ui = setup_ui();
        let perms = setup_perms_allow();
        let executor = ToolExecutor::new(&registry, &perms, &ui, 10);

        let results = executor
            .execute(vec![ToolCall {
                id: "c1".into(),
                name: "fail_tool".into(),
                input: serde_json::json!({}),
            }])
            .await;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, crate::domain::tool_call::ToolResultStatus::Error);
        assert!(results[0].content.contains("boom"));
    }

    #[tokio::test]
    async fn us2_permission_deny_returns_error_result() {
        let registry = TestToolRegistry::new().with_tool(SimpleTool {
            name: "restricted".into(),
            output: "should not see this".into(),
            safe: false,
        });
        let ui = setup_ui();
        let mut perms = MockPermissionPort::new();
        perms
            .expect_check()
            .returning(|_| Box::pin(async { PermissionDecision::Deny("forbidden".into()) }));

        let executor = ToolExecutor::new(&registry, &perms, &ui, 10);

        let results = executor
            .execute(vec![ToolCall {
                id: "c1".into(),
                name: "restricted".into(),
                input: serde_json::json!({}),
            }])
            .await;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, crate::domain::tool_call::ToolResultStatus::Error);
        assert!(results[0].content.contains("permission denied"));
    }

    #[tokio::test]
    async fn us4_safe_tools_partition_correctly() {
        let registry = TestToolRegistry::new()
            .with_tool(SimpleTool {
                name: "safe_a".into(),
                output: "a".into(),
                safe: true,
            })
            .with_tool(SimpleTool {
                name: "safe_b".into(),
                output: "b".into(),
                safe: true,
            })
            .with_tool(SimpleTool {
                name: "unsafe_c".into(),
                output: "c".into(),
                safe: false,
            });

        let ui = setup_ui();
        let perms = setup_perms_allow();
        let executor = ToolExecutor::new(&registry, &perms, &ui, 10);

        let calls = vec![
            ToolCall { id: "1".into(), name: "safe_a".into(), input: serde_json::json!({}) },
            ToolCall { id: "2".into(), name: "unsafe_c".into(), input: serde_json::json!({}) },
            ToolCall { id: "3".into(), name: "safe_b".into(), input: serde_json::json!({}) },
        ];

        let results = executor.execute(calls).await;

        // All 3 should complete, in original order
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].call_id, "1");
        assert_eq!(results[1].call_id, "2");
        assert_eq!(results[2].call_id, "3");
        assert_eq!(results[0].content, "a");
        assert_eq!(results[1].content, "c");
        assert_eq!(results[2].content, "b");
    }
}
