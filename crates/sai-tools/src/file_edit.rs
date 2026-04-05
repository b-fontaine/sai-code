//! File edit tool — performs targeted string replacement within a file.

use async_trait::async_trait;
use sai_core::error::ToolError;
use sai_core::ports::tool::{ToolOutput, ToolPort};
use serde::Deserialize;

use crate::config::ToolConfig;
use crate::validate::parse_input;

/// Input for the file-edit tool.
#[derive(Debug, Deserialize)]
pub struct FileEditInput {
    /// Absolute or project-relative file path.
    pub path: String,
    /// Exact string to find and replace.
    pub old_string: String,
    /// Replacement string.
    pub new_string: String,
}

/// Tool that performs targeted string replacement in files.
pub struct FileEditTool {
    config: ToolConfig,
}

impl FileEditTool {
    /// Create a new file-edit tool with the given config.
    pub fn new(config: ToolConfig) -> Self {
        Self { config }
    }

    fn resolve_path(&self, path: &str) -> std::path::PathBuf {
        let p = std::path::Path::new(path);
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            self.config.project_root.join(p)
        }
    }
}

#[async_trait]
impl ToolPort for FileEditTool {
    fn name(&self) -> &str {
        "file_edit"
    }

    fn description(&self) -> &str {
        "Replace a specific string in a file. The old_string must appear exactly once in the file."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute or project-relative file path"
                },
                "old_string": {
                    "type": "string",
                    "description": "Exact string to find and replace"
                },
                "new_string": {
                    "type": "string",
                    "description": "Replacement string"
                }
            },
            "required": ["path", "old_string", "new_string"]
        })
    }

    async fn execute(&self, input: serde_json::Value) -> Result<ToolOutput, ToolError> {
        let input: FileEditInput = parse_input(input)?;

        if input.path.is_empty() {
            return Err(ToolError::InvalidInput("path must not be empty".into()));
        }
        if input.old_string.is_empty() {
            return Err(ToolError::InvalidInput(
                "old_string must not be empty".into(),
            ));
        }
        if input.old_string == input.new_string {
            return Err(ToolError::InvalidInput(
                "old_string and new_string must be different".into(),
            ));
        }

        let resolved = self.resolve_path(&input.path);

        let content = tokio::fs::read_to_string(&resolved).await.map_err(|e| {
            ToolError::Execution(format!("failed to read '{}': {e}", resolved.display()))
        })?;

        let count = content.matches(&input.old_string).count();

        if count == 0 {
            return Ok(ToolOutput::Error(format!(
                "old_string not found in '{}'",
                resolved.display()
            )));
        }
        if count > 1 {
            return Ok(ToolOutput::Error(format!(
                "old_string found {count} times in '{}' — edit is ambiguous, provide more context",
                resolved.display()
            )));
        }

        let new_content = content.replacen(&input.old_string, &input.new_string, 1);
        tokio::fs::write(&resolved, &new_content)
            .await
            .map_err(|e| {
                ToolError::Execution(format!("failed to write '{}': {e}", resolved.display()))
            })?;

        Ok(ToolOutput::Success(format!(
            "replaced 1 occurrence in '{}'",
            resolved.display()
        )))
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
    async fn replace_found_string() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("f.txt"), "hello world").unwrap();
        let tool = FileEditTool::new(test_config(&dir));

        let result = tool
            .execute(serde_json::json!({
                "path": "f.txt",
                "old_string": "world",
                "new_string": "rust"
            }))
            .await
            .unwrap();

        match result {
            ToolOutput::Success(msg) => assert!(msg.contains("replaced 1 occurrence")),
            ToolOutput::Error(e) => panic!("expected success, got error: {e}"),
        }

        let content = std::fs::read_to_string(dir.path().join("f.txt")).unwrap();
        assert_eq!(content, "hello rust");
    }

    #[tokio::test]
    async fn not_found_returns_error_output() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("f.txt"), "hello world").unwrap();
        let tool = FileEditTool::new(test_config(&dir));

        let result = tool
            .execute(serde_json::json!({
                "path": "f.txt",
                "old_string": "missing",
                "new_string": "x"
            }))
            .await
            .unwrap();

        match result {
            ToolOutput::Error(msg) => assert!(msg.contains("not found")),
            ToolOutput::Success(_) => panic!("expected error output"),
        }

        // File unchanged
        let content = std::fs::read_to_string(dir.path().join("f.txt")).unwrap();
        assert_eq!(content, "hello world");
    }

    #[tokio::test]
    async fn ambiguous_match_returns_error_output() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("f.txt"), "aaa").unwrap();
        let tool = FileEditTool::new(test_config(&dir));

        let result = tool
            .execute(serde_json::json!({
                "path": "f.txt",
                "old_string": "a",
                "new_string": "b"
            }))
            .await
            .unwrap();

        match result {
            ToolOutput::Error(msg) => assert!(msg.contains("3 times")),
            ToolOutput::Success(_) => panic!("expected error output"),
        }

        // File unchanged
        let content = std::fs::read_to_string(dir.path().join("f.txt")).unwrap();
        assert_eq!(content, "aaa");
    }

    #[tokio::test]
    async fn same_old_new_returns_error() {
        let dir = TempDir::new().unwrap();
        let tool = FileEditTool::new(test_config(&dir));

        let result = tool
            .execute(serde_json::json!({
                "path": "f.txt",
                "old_string": "same",
                "new_string": "same"
            }))
            .await;

        assert!(result.is_err());
    }
}
