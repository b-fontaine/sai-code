//! Tool execution port traits.

use async_trait::async_trait;

use crate::error::ToolError;

/// Output from a tool execution.
#[derive(Debug, Clone)]
pub enum ToolOutput {
    /// The tool executed successfully.
    Success(String),
    /// The tool encountered an error.
    Error(String),
}

/// Port trait for a single tool's execution.
///
/// Each registered tool implements this trait. The agent loop discovers
/// tools through the `ToolRegistryPort` and executes them via this trait.
#[async_trait]
pub trait ToolPort: Send + Sync {
    /// The unique name of this tool.
    fn name(&self) -> &str;

    /// A human-readable description (sent to the model).
    fn description(&self) -> &str;

    /// JSON Schema for the tool's input parameters.
    fn input_schema(&self) -> serde_json::Value;

    /// Execute the tool with the given input arguments.
    async fn execute(&self, input: serde_json::Value) -> Result<ToolOutput, ToolError>;

    /// Whether this tool can safely run in parallel with other tools.
    fn is_concurrency_safe(&self) -> bool {
        false
    }

    /// Whether this tool only reads state (does not modify files, etc.).
    fn is_read_only(&self) -> bool {
        false
    }
}

/// Port trait for looking up and listing registered tools.
pub trait ToolRegistryPort: Send + Sync {
    /// Look up a tool by name.
    fn get(&self, name: &str) -> Option<&dyn ToolPort>;

    /// Return all registered tools.
    fn list(&self) -> Vec<&dyn ToolPort>;

    /// Return tool definitions in the format expected by the LLM.
    ///
    /// Each definition includes the tool's name, description, and
    /// input schema as a JSON object.
    fn tool_definitions(&self) -> Vec<serde_json::Value>;
}
