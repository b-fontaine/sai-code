//! Shell execution tool — runs commands and returns stdout, stderr, exit code.

use std::time::Duration;

use async_trait::async_trait;
use sai_core::error::ToolError;
use sai_core::ports::tool::{ToolOutput, ToolPort};
use serde::Deserialize;
use tokio::process::Command;

use crate::config::ToolConfig;
use crate::shell_safety::{check_command_safety, SafetyVerdict};
use crate::truncate::truncate_output;
use crate::validate::parse_input;

/// Input for the shell tool.
#[derive(Debug, Deserialize)]
pub struct ShellInput {
    /// Shell command to execute.
    pub command: String,
    /// Working directory (default: project root).
    pub working_dir: Option<String>,
    /// Timeout in milliseconds (default: from config).
    pub timeout_ms: Option<u64>,
}

/// Tool that executes shell commands.
pub struct ShellTool {
    config: ToolConfig,
}

impl ShellTool {
    /// Create a new shell tool with the given config.
    pub fn new(config: ToolConfig) -> Self {
        Self { config }
    }

    fn resolve_working_dir(&self, dir: Option<&str>) -> std::path::PathBuf {
        match dir {
            Some(d) => {
                let p = std::path::Path::new(d);
                if p.is_absolute() {
                    p.to_path_buf()
                } else {
                    self.config.project_root.join(p)
                }
            }
            None => self.config.project_root.clone(),
        }
    }
}

#[async_trait]
impl ToolPort for ShellTool {
    fn name(&self) -> &str {
        "shell"
    }

    fn description(&self) -> &str {
        "Execute a shell command and return stdout, stderr, and exit code."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Shell command to execute"
                },
                "working_dir": {
                    "type": "string",
                    "description": "Working directory (default: project root)"
                },
                "timeout_ms": {
                    "type": "integer",
                    "description": "Timeout in milliseconds",
                    "minimum": 1
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, input: serde_json::Value) -> Result<ToolOutput, ToolError> {
        let input: ShellInput = parse_input(input)?;

        if input.command.is_empty() {
            return Err(ToolError::InvalidInput("command must not be empty".into()));
        }

        // Safety check
        match check_command_safety(&input.command) {
            SafetyVerdict::Safe => {}
            SafetyVerdict::Dangerous(reason) => {
                return Ok(ToolOutput::Error(reason));
            }
        }

        let working_dir = self.resolve_working_dir(input.working_dir.as_deref());
        if !working_dir.exists() {
            return Ok(ToolOutput::Error(format!(
                "working directory does not exist: '{}'",
                working_dir.display()
            )));
        }

        let timeout_ms = input.timeout_ms.unwrap_or(self.config.shell_timeout_ms);
        let timeout = Duration::from_millis(timeout_ms);

        let child = Command::new("/bin/sh")
            .arg("-c")
            .arg(&input.command)
            .current_dir(&working_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| ToolError::Execution(format!("failed to spawn shell: {e}")))?;

        let result = tokio::time::timeout(timeout, child.wait_with_output()).await;

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let code = output.status.code().unwrap_or(-1);

                let formatted =
                    format!("Exit code: {code}\n--- stdout ---\n{stdout}--- stderr ---\n{stderr}");

                Ok(ToolOutput::Success(truncate_output(
                    &formatted,
                    self.config.max_output_bytes,
                )))
            }
            Ok(Err(e)) => Ok(ToolOutput::Error(format!("command failed: {e}"))),
            Err(_) => Ok(ToolOutput::Error(format!(
                "command timed out after {timeout_ms}ms"
            ))),
        }
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
    async fn successful_command() {
        let dir = TempDir::new().unwrap();
        let tool = ShellTool::new(test_config(&dir));

        let result = tool
            .execute(serde_json::json!({"command": "echo hello"}))
            .await
            .unwrap();

        match result {
            ToolOutput::Success(content) => {
                assert!(content.contains("Exit code: 0"));
                assert!(content.contains("hello"));
            }
            ToolOutput::Error(e) => panic!("expected success, got error: {e}"),
        }
    }

    #[tokio::test]
    async fn command_with_stderr() {
        let dir = TempDir::new().unwrap();
        let tool = ShellTool::new(test_config(&dir));

        let result = tool
            .execute(serde_json::json!({"command": "echo err >&2"}))
            .await
            .unwrap();

        match result {
            ToolOutput::Success(content) => {
                assert!(content.contains("err"));
                assert!(content.contains("--- stderr ---"));
            }
            ToolOutput::Error(e) => panic!("expected success, got error: {e}"),
        }
    }

    #[tokio::test]
    async fn nonzero_exit_code() {
        let dir = TempDir::new().unwrap();
        let tool = ShellTool::new(test_config(&dir));

        let result = tool
            .execute(serde_json::json!({"command": "exit 42"}))
            .await
            .unwrap();

        match result {
            ToolOutput::Success(content) => {
                assert!(content.contains("Exit code: 42"));
            }
            ToolOutput::Error(e) => panic!("expected success, got error: {e}"),
        }
    }

    #[tokio::test]
    async fn command_timeout() {
        let dir = TempDir::new().unwrap();
        let tool = ShellTool::new(test_config(&dir));

        let result = tool
            .execute(serde_json::json!({"command": "sleep 60", "timeout_ms": 100}))
            .await
            .unwrap();

        match result {
            ToolOutput::Error(msg) => assert!(msg.contains("timed out")),
            ToolOutput::Success(_) => panic!("expected timeout error"),
        }
    }

    #[tokio::test]
    async fn dangerous_command_blocked() {
        let dir = TempDir::new().unwrap();
        let tool = ShellTool::new(test_config(&dir));

        let result = tool
            .execute(serde_json::json!({"command": "rm -rf /"}))
            .await
            .unwrap();

        match result {
            ToolOutput::Error(msg) => assert!(msg.contains("blocked")),
            ToolOutput::Success(_) => panic!("expected dangerous command to be blocked"),
        }
    }

    #[tokio::test]
    async fn nonexistent_working_dir() {
        let dir = TempDir::new().unwrap();
        let tool = ShellTool::new(test_config(&dir));

        let result = tool
            .execute(serde_json::json!({
                "command": "echo hi",
                "working_dir": "/nonexistent_dir_12345"
            }))
            .await
            .unwrap();

        match result {
            ToolOutput::Error(msg) => assert!(msg.contains("does not exist")),
            ToolOutput::Success(_) => panic!("expected error for nonexistent dir"),
        }
    }
}
