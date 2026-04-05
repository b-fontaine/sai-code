//! Permission port trait for tool execution authorization.

use async_trait::async_trait;

use crate::domain::tool_call::ToolCall;

/// A request to check whether a tool call is permitted.
#[derive(Debug, Clone)]
pub struct PermissionRequest {
    /// The tool call to check.
    pub tool_call: ToolCall,
    /// Whether the tool is read-only.
    pub is_read_only: bool,
}

/// The decision from a permission check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionDecision {
    /// The tool call is permitted.
    Allow,
    /// The tool call is denied.
    Deny(String),
    /// The user should be asked for approval.
    Ask,
}

/// Port trait for checking tool execution permissions.
///
/// This trait MUST be called before every tool execution. The agent
/// loop MUST NOT execute a tool without a permission check.
#[async_trait]
#[cfg_attr(test, mockall::automock)]
pub trait PermissionPort: Send + Sync {
    /// Check whether the given tool call is permitted.
    async fn check(&self, request: &PermissionRequest) -> PermissionDecision;
}
