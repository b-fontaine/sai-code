//! File write tool — creates or overwrites a file with given content.

use async_trait::async_trait;
use sai_core::error::ToolError;
use sai_core::ports::tool::{ToolOutput, ToolPort};
use serde::Deserialize;

use crate::config::ToolConfig;
use crate::validate::parse_input;

/// Input for the file-write tool.
#[derive(Debug, Deserialize)]
pub struct FileWriteInput {
    /// Absolute or project-relative file path.
    pub path: String,
    /// Full file content to write.
    pub content: String,
}

/// Tool that creates or overwrites files.
pub struct FileWriteTool {
    config: ToolConfig,
}

impl FileWriteTool {
    /// Create a new file-write tool with the given config.
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
impl ToolPort for FileWriteTool {
    fn name(&self) -> &str {
        "file_write"
    }

    fn description(&self) -> &str {
        "Create or overwrite a file with the given content. Creates parent directories if needed."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute or project-relative file path"
                },
                "content": {
                    "type": "string",
                    "description": "Full file content to write"
                }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, input: serde_json::Value) -> Result<ToolOutput, ToolError> {
        let input: FileWriteInput = parse_input(input)?;

        if input.path.is_empty() {
            return Err(ToolError::InvalidInput("path must not be empty".into()));
        }

        let resolved = self.resolve_path(&input.path);

        // Create parent directories if they don't exist
        if let Some(parent) = resolved.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                ToolError::Execution(format!(
                    "failed to create directories for '{}': {e}",
                    resolved.display()
                ))
            })?;
        }

        let byte_count = input.content.len();
        tokio::fs::write(&resolved, &input.content)
            .await
            .map_err(|e| {
                ToolError::Execution(format!("failed to write '{}': {e}", resolved.display()))
            })?;

        Ok(ToolOutput::Success(format!(
            "wrote {byte_count} bytes to '{}'",
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
    async fn write_new_file() {
        let dir = TempDir::new().unwrap();
        let tool = FileWriteTool::new(test_config(&dir));

        let result = tool
            .execute(serde_json::json!({"path": "new.txt", "content": "hello"}))
            .await
            .unwrap();

        match &result {
            ToolOutput::Success(msg) => assert!(msg.contains("5 bytes")),
            ToolOutput::Error(e) => panic!("expected success, got error: {e}"),
        }

        let content = std::fs::read_to_string(dir.path().join("new.txt")).unwrap();
        assert_eq!(content, "hello");
    }

    #[tokio::test]
    async fn overwrite_existing_file() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("exist.txt"), "old").unwrap();
        let tool = FileWriteTool::new(test_config(&dir));

        tool.execute(serde_json::json!({"path": "exist.txt", "content": "new"}))
            .await
            .unwrap();

        let content = std::fs::read_to_string(dir.path().join("exist.txt")).unwrap();
        assert_eq!(content, "new");
    }

    #[tokio::test]
    async fn creates_parent_directories() {
        let dir = TempDir::new().unwrap();
        let tool = FileWriteTool::new(test_config(&dir));

        tool.execute(serde_json::json!({"path": "a/b/c.txt", "content": "deep"}))
            .await
            .unwrap();

        let content = std::fs::read_to_string(dir.path().join("a/b/c.txt")).unwrap();
        assert_eq!(content, "deep");
    }

    #[tokio::test]
    async fn empty_path_returns_error() {
        let dir = TempDir::new().unwrap();
        let tool = FileWriteTool::new(test_config(&dir));

        let result = tool
            .execute(serde_json::json!({"path": "", "content": "x"}))
            .await;

        assert!(result.is_err());
    }
}
