//! Shared configuration for tool behavior.

use std::path::PathBuf;

/// Configuration shared across all tools.
#[derive(Debug, Clone)]
pub struct ToolConfig {
    /// Root directory of the project.
    pub project_root: PathBuf,
    /// Maximum output size in bytes before truncation (default: 100 KB).
    pub max_output_bytes: usize,
    /// Default shell command timeout in milliseconds (default: 120 000).
    pub shell_timeout_ms: u64,
    /// Default maximum search results for grep/glob (default: 100).
    pub max_search_results: usize,
}

impl ToolConfig {
    /// Create a new config with the given project root and sensible defaults.
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            project_root,
            max_output_bytes: 100 * 1024,
            shell_timeout_ms: 120_000,
            max_search_results: 100,
        }
    }
}
