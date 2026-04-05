//! Agent configuration.

/// Runtime configuration for the agent loop.
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// System prompt prepended to every model request.
    pub system_prompt: String,
    /// The model identifier string (e.g., `"claude-sonnet-4"`).
    pub model_name: String,
    /// Maximum tool-call iterations per turn before stopping.
    pub max_iterations_per_turn: usize,
    /// Maximum concurrent tool executions.
    pub max_parallel_tool_calls: usize,
    /// Maximum retries for transient LLM errors.
    pub max_retries_on_error: usize,
    /// Maximum number of messages before emitting a history size warning.
    pub max_history_messages: usize,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            system_prompt: String::new(),
            model_name: String::from("claude-sonnet-4"),
            max_iterations_per_turn: 50,
            max_parallel_tool_calls: 10,
            max_retries_on_error: 3,
            max_history_messages: 200,
        }
    }
}
