//! File read tool — reads file contents with optional line-range filtering.

use async_trait::async_trait;
use sai_core::error::ToolError;
use sai_core::ports::tool::{ToolOutput, ToolPort};
use serde::Deserialize;

use crate::config::ToolConfig;
use crate::truncate::truncate_output;
use crate::validate::parse_input;

/// Input for the file-read tool.
#[derive(Debug, Deserialize)]
pub struct FileReadInput {
    /// Absolute or project-relative file path.
    pub path: String,
    /// Start line (1-based, inclusive).
    pub offset: Option<usize>,
    /// Number of lines to return.
    pub limit: Option<usize>,
}

/// Tool that reads file contents.
pub struct FileReadTool {
    config: ToolConfig,
}

impl FileReadTool {
    /// Create a new file-read tool with the given config.
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

/// Check if a buffer looks like binary content (contains null bytes).
fn is_binary(buf: &[u8]) -> bool {
    buf.contains(&0)
}

#[async_trait]
impl ToolPort for FileReadTool {
    fn name(&self) -> &str {
        "file_read"
    }

    fn description(&self) -> &str {
        "Read the contents of a file. Supports optional line-range filtering with offset and limit parameters."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute or project-relative file path"
                },
                "offset": {
                    "type": "integer",
                    "description": "Start line (1-based, inclusive)",
                    "minimum": 1
                },
                "limit": {
                    "type": "integer",
                    "description": "Number of lines to return",
                    "minimum": 1
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, input: serde_json::Value) -> Result<ToolOutput, ToolError> {
        let input: FileReadInput = parse_input(input)?;

        if input.path.is_empty() {
            return Err(ToolError::InvalidInput("path must not be empty".into()));
        }
        if let Some(offset) = input.offset {
            if offset == 0 {
                return Err(ToolError::InvalidInput(
                    "offset must be >= 1 (1-based)".into(),
                ));
            }
        }

        let resolved = self.resolve_path(&input.path);

        let bytes = tokio::fs::read(&resolved).await.map_err(|e| {
            ToolError::Execution(format!("failed to read '{}': {e}", resolved.display()))
        })?;

        // Binary detection on first 8KB
        let check_len = bytes.len().min(8192);
        if is_binary(&bytes[..check_len]) {
            return Ok(ToolOutput::Error(format!(
                "binary file detected: '{}' — cannot display contents",
                resolved.display()
            )));
        }

        let content = String::from_utf8_lossy(&bytes);

        // Apply line-range filtering
        let output = if input.offset.is_some() || input.limit.is_some() {
            let lines: Vec<&str> = content.lines().collect();
            let start = input.offset.unwrap_or(1).saturating_sub(1);
            let end = if let Some(limit) = input.limit {
                (start + limit).min(lines.len())
            } else {
                lines.len()
            };

            if start >= lines.len() {
                String::new()
            } else {
                lines[start..end].join("\n")
            }
        } else {
            content.into_owned()
        };

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
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn test_config(dir: &TempDir) -> ToolConfig {
        ToolConfig::new(dir.path().to_path_buf())
    }

    #[tokio::test]
    async fn read_existing_file_returns_contents() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("hello.txt"), "hello world").unwrap();
        let tool = FileReadTool::new(test_config(&dir));

        let result = tool
            .execute(serde_json::json!({"path": "hello.txt"}))
            .await
            .unwrap();

        match result {
            ToolOutput::Success(content) => assert_eq!(content, "hello world"),
            ToolOutput::Error(e) => panic!("expected success, got error: {e}"),
        }
    }

    #[tokio::test]
    async fn read_nonexistent_file_returns_error() {
        let dir = TempDir::new().unwrap();
        let tool = FileReadTool::new(test_config(&dir));

        let result = tool.execute(serde_json::json!({"path": "nope.txt"})).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn read_with_offset_and_limit() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("lines.txt"), "a\nb\nc\nd\ne").unwrap();
        let tool = FileReadTool::new(test_config(&dir));

        let result = tool
            .execute(serde_json::json!({"path": "lines.txt", "offset": 2, "limit": 2}))
            .await
            .unwrap();

        match result {
            ToolOutput::Success(content) => assert_eq!(content, "b\nc"),
            ToolOutput::Error(e) => panic!("expected success, got error: {e}"),
        }
    }

    #[tokio::test]
    async fn binary_file_returns_error_output() {
        let dir = TempDir::new().unwrap();
        let mut data = vec![0u8; 100];
        data[0] = b'H';
        data[1] = b'i';
        data[5] = 0; // null byte
        std::fs::write(dir.path().join("binary.bin"), &data).unwrap();
        let tool = FileReadTool::new(test_config(&dir));

        let result = tool
            .execute(serde_json::json!({"path": "binary.bin"}))
            .await
            .unwrap();

        match result {
            ToolOutput::Error(msg) => assert!(msg.contains("binary file detected")),
            ToolOutput::Success(_) => panic!("expected error for binary file"),
        }
    }

    #[tokio::test]
    async fn output_truncation() {
        let dir = TempDir::new().unwrap();
        let big_content = "x".repeat(200);
        std::fs::write(dir.path().join("big.txt"), &big_content).unwrap();

        let mut config = test_config(&dir);
        config.max_output_bytes = 50;
        let tool = FileReadTool::new(config);

        let result = tool
            .execute(serde_json::json!({"path": "big.txt"}))
            .await
            .unwrap();

        match result {
            ToolOutput::Success(content) => {
                assert!(content.contains("[output truncated"));
            }
            ToolOutput::Error(e) => panic!("expected success, got error: {e}"),
        }
    }

    #[tokio::test]
    async fn absolute_path_works() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("abs.txt");
        std::fs::write(&file_path, "absolute").unwrap();
        let tool = FileReadTool::new(ToolConfig::new(PathBuf::from("/tmp")));

        let result = tool
            .execute(serde_json::json!({"path": file_path.to_str().unwrap()}))
            .await
            .unwrap();

        match result {
            ToolOutput::Success(content) => assert_eq!(content, "absolute"),
            ToolOutput::Error(e) => panic!("expected success, got error: {e}"),
        }
    }
}
