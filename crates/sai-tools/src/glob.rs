//! File search tool — finds files matching a glob pattern.

use async_trait::async_trait;
use ignore::WalkBuilder;
use sai_core::error::ToolError;
use sai_core::ports::tool::{ToolOutput, ToolPort};
use serde::Deserialize;

use crate::config::ToolConfig;
use crate::truncate::truncate_output;
use crate::validate::parse_input;

/// Input for the glob tool.
#[derive(Debug, Deserialize)]
pub struct GlobInput {
    /// Glob pattern (e.g., "**/*.rs").
    pub pattern: String,
    /// Directory to search in (default: project root).
    pub path: Option<String>,
}

/// Tool that finds files matching a glob pattern.
pub struct GlobTool {
    config: ToolConfig,
}

impl GlobTool {
    /// Create a new glob tool with the given config.
    pub fn new(config: ToolConfig) -> Self {
        Self { config }
    }

    fn resolve_path(&self, path: Option<&str>) -> std::path::PathBuf {
        match path {
            Some(p) => {
                let pb = std::path::Path::new(p);
                if pb.is_absolute() {
                    pb.to_path_buf()
                } else {
                    self.config.project_root.join(pb)
                }
            }
            None => self.config.project_root.clone(),
        }
    }
}

#[async_trait]
impl ToolPort for GlobTool {
    fn name(&self) -> &str {
        "glob"
    }

    fn description(&self) -> &str {
        "Find files matching a glob pattern. Returns matching file paths, one per line."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern (e.g., \"**/*.rs\")"
                },
                "path": {
                    "type": "string",
                    "description": "Directory to search in (default: project root)"
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, input: serde_json::Value) -> Result<ToolOutput, ToolError> {
        let input: GlobInput = parse_input(input)?;

        if input.pattern.is_empty() {
            return Err(ToolError::InvalidInput("pattern must not be empty".into()));
        }

        let glob = globset::GlobBuilder::new(&input.pattern)
            .literal_separator(false)
            .build()
            .map_err(|e| ToolError::InvalidInput(format!("invalid glob pattern: {e}")))?
            .compile_matcher();

        let search_path = self.resolve_path(input.path.as_deref());
        let max_results = self.config.max_search_results;

        let walker = WalkBuilder::new(&search_path)
            .hidden(false)
            .git_ignore(true)
            .build();

        let mut results = Vec::new();

        for entry in walker.flatten() {
            if results.len() >= max_results {
                break;
            }

            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            // Match against relative path from search root
            let relative = path.strip_prefix(&search_path).unwrap_or(path);

            if glob.is_match(relative) {
                results.push(path.display().to_string());
            }
        }

        if results.is_empty() {
            return Ok(ToolOutput::Success(String::new()));
        }

        let output = results.join("\n");
        Ok(ToolOutput::Success(truncate_output(
            &output,
            self.config.max_output_bytes,
        )))
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    fn is_read_only(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_config(dir: &TempDir) -> ToolConfig {
        ToolConfig::new(dir.path().to_path_buf())
    }

    #[tokio::test]
    async fn matches_files_by_extension() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("a.rs"), "rust").unwrap();
        std::fs::write(dir.path().join("b.txt"), "text").unwrap();
        std::fs::write(dir.path().join("c.rs"), "rust2").unwrap();
        let tool = GlobTool::new(test_config(&dir));

        let result = tool
            .execute(serde_json::json!({"pattern": "*.rs"}))
            .await
            .unwrap();

        match result {
            ToolOutput::Success(content) => {
                assert!(content.contains("a.rs"));
                assert!(content.contains("c.rs"));
                assert!(!content.contains("b.txt"));
            }
            ToolOutput::Error(e) => panic!("expected success, got error: {e}"),
        }
    }

    #[tokio::test]
    async fn no_matches_returns_empty() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("a.txt"), "text").unwrap();
        let tool = GlobTool::new(test_config(&dir));

        let result = tool
            .execute(serde_json::json!({"pattern": "*.xyz"}))
            .await
            .unwrap();

        match result {
            ToolOutput::Success(content) => assert!(content.is_empty()),
            ToolOutput::Error(e) => panic!("expected success, got error: {e}"),
        }
    }

    #[tokio::test]
    async fn recursive_glob() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("sub/deep")).unwrap();
        std::fs::write(dir.path().join("top.rs"), "").unwrap();
        std::fs::write(dir.path().join("sub/mid.rs"), "").unwrap();
        std::fs::write(dir.path().join("sub/deep/bot.rs"), "").unwrap();
        let tool = GlobTool::new(test_config(&dir));

        let result = tool
            .execute(serde_json::json!({"pattern": "**/*.rs"}))
            .await
            .unwrap();

        match result {
            ToolOutput::Success(content) => {
                assert!(content.contains("top.rs"));
                assert!(content.contains("mid.rs"));
                assert!(content.contains("bot.rs"));
            }
            ToolOutput::Error(e) => panic!("expected success, got error: {e}"),
        }
    }

    #[tokio::test]
    async fn invalid_glob_returns_error() {
        let dir = TempDir::new().unwrap();
        let tool = GlobTool::new(test_config(&dir));

        let result = tool
            .execute(serde_json::json!({"pattern": "[invalid"}))
            .await;

        assert!(result.is_err());
    }
}
