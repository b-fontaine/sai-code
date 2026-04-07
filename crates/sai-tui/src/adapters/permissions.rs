use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use sai_core::ports::permissions::{PermissionDecision, PermissionPort, PermissionRequest};
use tokio::sync::oneshot;

use crate::app::{AgentStatus, AppState, PendingPermission};

/// Adapter implementing `PermissionPort` via an in-TUI permission prompt.
///
/// When `check()` is called:
/// - Read-only tools: immediately `Allow`
/// - Non-interactive mode: immediately `Deny`
/// - Interactive: set `pending_permission` in `AppState`, await user response via oneshot
#[derive(Clone)]
pub struct TuiPermissionsAdapter {
    state: Arc<Mutex<AppState>>,
    is_interactive: bool,
}

impl TuiPermissionsAdapter {
    pub fn new(state: Arc<Mutex<AppState>>, is_interactive: bool) -> Self {
        Self {
            state,
            is_interactive,
        }
    }
}

#[async_trait]
impl PermissionPort for TuiPermissionsAdapter {
    async fn check(&self, request: &PermissionRequest) -> PermissionDecision {
        if request.is_read_only {
            return PermissionDecision::Allow;
        }
        if !self.is_interactive {
            return PermissionDecision::Deny(
                "non-interactive mode: write operations require user confirmation".into(),
            );
        }

        // Build description from tool call input
        let tool_name = request.tool_call.name.clone();
        let action_description = serde_json::to_string_pretty(&request.tool_call.input)
            .unwrap_or_else(|_| "(unknown input)".into());

        // Create oneshot channel
        let (tx, rx) = oneshot::channel();

        // Write pending permission to shared state
        {
            let mut state = self.state.lock().unwrap();
            state.pending_permission = Some(PendingPermission {
                tool_name,
                action_description,
                response_tx: tx,
            });
            state.status = AgentStatus::AwaitingPermission;
        }

        // Await user's response (TUI event loop sends on this channel when user presses y/n)
        match rx.await {
            Ok(decision) => decision,
            Err(_) => PermissionDecision::Deny("permission channel closed".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{AppState, TuiConfig};
    use sai_core::domain::tool_call::ToolCall;
    use sai_core::ports::permissions::PermissionRequest;
    use std::sync::{Arc, Mutex};

    fn make_read_only_request() -> PermissionRequest {
        PermissionRequest {
            tool_call: ToolCall {
                id: "t1".into(),
                name: "read_file".into(),
                input: serde_json::json!({}),
            },
            is_read_only: true,
        }
    }

    fn make_write_request() -> PermissionRequest {
        PermissionRequest {
            tool_call: ToolCall {
                id: "t2".into(),
                name: "write_file".into(),
                input: serde_json::json!({"path": "foo.txt"}),
            },
            is_read_only: false,
        }
    }

    #[tokio::test]
    async fn read_only_always_allowed() {
        let state = Arc::new(Mutex::new(AppState::new(&TuiConfig::default())));
        let adapter = TuiPermissionsAdapter::new(state, true);
        let req = make_read_only_request();
        let result = adapter.check(&req).await;
        assert_eq!(result, PermissionDecision::Allow);
    }

    #[tokio::test]
    async fn non_interactive_write_denied() {
        let state = Arc::new(Mutex::new(AppState::new(&TuiConfig::default())));
        let adapter = TuiPermissionsAdapter::new(state, false);
        let req = make_write_request();
        let result = adapter.check(&req).await;
        assert!(matches!(result, PermissionDecision::Deny(_)));
    }
}
