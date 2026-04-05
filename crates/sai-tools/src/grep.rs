//! Content search tool — searches file contents for regex pattern matches.

use async_trait::async_trait;
use grep_regex::RegexMatcher;
use grep_searcher::sinks::UTF8;
use grep_searcher::Searcher;
use ignore::WalkBuilder;
use sai_core::error::ToolError;
use sai_core::ports::tool::{ToolOutput, ToolPort};
use serde::Deserialize;

use crate::config::ToolConfig;
use crate::truncate::truncate_output;
use crate::validate::parse_input;

/// Input for the grep tool.
#[derive(Debug, Deserialize)]
pub struct GrepInput {
    /// Regex pattern to search for.
    pub pattern: String,
    /// Directory or file to search in (default: project root).
    pub path: Option<String>,
    /// File name filter (e.g., "*.rs").
    pub glob: Option<String>,
    /// Maximum number of matches returned (default: from config).
    pub max_results: Option<usize>,
}

/// Tool that searches file contents for regex pattern matches.
pub struct GrepTool {
    config: ToolConfig,
}

impl GrepTool {
    /// Create a new grep tool with the given config.
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
impl ToolPort for GrepTool {
    fn name(&self) -> &str {
        "grep"
    }

    fn description(&self) -> &str {
        "Search file contents for lines matching a regex pattern. Returns matching lines with file paths and line numbers."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Regex pattern to search for"
                },
                "path": {
                    "type": "string",
                    "description": "Directory or file to search in (default: project root)"
                },
                "glob": {
                    "type": "string",
                    "description": "File name filter (e.g., \"*.rs\")"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of matches to return",
                    "minimum": 1
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, input: serde_json::Value) -> Result<ToolOutput, ToolError> {
        let input: GrepInput = parse_input(input)?;

        if input.pattern.is_empty() {
            return Err(ToolError::InvalidInput("pattern must not be empty".into()));
        }

        let matcher = RegexMatcher::new(&input.pattern)
            .map_err(|e| ToolError::InvalidInput(format!("invalid regex pattern: {e}")))?;

        let search_path = self.resolve_path(input.path.as_deref());
        let max_results = input.max_results.unwrap_or(self.config.max_search_results);

        // Build the walker (respects .gitignore)
        let mut walker_builder = WalkBuilder::new(&search_path);
        walker_builder.hidden(false).git_ignore(true);

        if let Some(ref glob_pattern) = input.glob {
            let mut types_builder = ignore::types::TypesBuilder::new();
            types_builder
                .add("custom", glob_pattern)
                .map_err(|e| ToolError::InvalidInput(format!("invalid glob pattern: {e}")))?;
            types_builder.select("custom");
            walker_builder.types(types_builder.build().map_err(|e| {
                ToolError::InvalidInput(format!("failed to build glob filter: {e}"))
            })?);
        }

        let mut results = Vec::new();
        let mut searcher = Searcher::new();

        for entry in walker_builder.build().flatten() {
            if results.len() >= max_results {
                break;
            }

            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let path_display = path.display().to_string();
            let _ = searcher.search_path(
                &matcher,
                path,
                UTF8(|line_num, line| {
                    if results.len() < max_results {
                        results.push(format!("{}:{}:{}", path_display, line_num, line.trim_end()));
                    }
                    Ok(results.len() < max_results)
                }),
            );
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
    async fn pattern_matches_return_results() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("a.txt"),
            "hello world\nfoo bar\nhello again",
        )
        .unwrap();
        let tool = GrepTool::new(test_config(&dir));

        let result = tool
            .execute(serde_json::json!({"pattern": "hello"}))
            .await
            .unwrap();

        match result {
            ToolOutput::Success(content) => {
                assert!(content.contains("hello world"));
                assert!(content.contains("hello again"));
                // Should have file path and line numbers
                assert!(content.contains(":1:"));
                assert!(content.contains(":3:"));
            }
            ToolOutput::Error(e) => panic!("expected success, got error: {e}"),
        }
    }

    #[tokio::test]
    async fn no_matches_returns_empty() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("a.txt"), "hello world").unwrap();
        let tool = GrepTool::new(test_config(&dir));

        let result = tool
            .execute(serde_json::json!({"pattern": "zzz_not_found"}))
            .await
            .unwrap();

        match result {
            ToolOutput::Success(content) => assert!(content.is_empty()),
            ToolOutput::Error(e) => panic!("expected success, got error: {e}"),
        }
    }

    #[tokio::test]
    async fn invalid_regex_returns_error() {
        let dir = TempDir::new().unwrap();
        let tool = GrepTool::new(test_config(&dir));

        let result = tool
            .execute(serde_json::json!({"pattern": "[invalid"}))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn max_results_caps_output() {
        let dir = TempDir::new().unwrap();
        let content = (0..50)
            .map(|i| format!("match line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(dir.path().join("many.txt"), content).unwrap();
        let tool = GrepTool::new(test_config(&dir));

        let result = tool
            .execute(serde_json::json!({"pattern": "match", "max_results": 5}))
            .await
            .unwrap();

        match result {
            ToolOutput::Success(content) => {
                let lines: Vec<&str> = content.lines().collect();
                assert_eq!(lines.len(), 5);
            }
            ToolOutput::Error(e) => panic!("expected success, got error: {e}"),
        }
    }
}
