//! Terminal permission adapter: interactive y/n prompts for tool approval.

use async_trait::async_trait;
use sai_core::ports::permissions::{PermissionDecision, PermissionPort, PermissionRequest};
use std::io::{self, IsTerminal, Write as _};

/// Concrete `PermissionPort` that prompts the user interactively.
///
/// - Read-only tools are always allowed without prompting.
/// - In non-interactive mode (stdin is a pipe), non-read-only tools are denied.
/// - In interactive mode, the user is prompted with "Allow {tool}? (y/n): ".
pub struct TerminalPermissions {
    is_interactive: bool,
}

impl TerminalPermissions {
    /// Create a new instance, detecting whether stdin is a terminal.
    pub fn new() -> Self {
        Self {
            is_interactive: io::stdin().is_terminal(),
        }
    }
}

impl Default for TerminalPermissions {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PermissionPort for TerminalPermissions {
    async fn check(&self, request: &PermissionRequest) -> PermissionDecision {
        if request.is_read_only {
            return PermissionDecision::Allow;
        }

        if !self.is_interactive {
            return PermissionDecision::Deny(
                "non-interactive mode: write operations require user confirmation".into(),
            );
        }

        let tool_name = &request.tool_call.name;
        let mut stderr = std::io::stderr();
        let _ = write!(stderr, "\nAllow {tool_name}? (y/n): ");
        let _ = stderr.flush();

        // Read from /dev/tty if available, else stdin
        let response = read_tty_line();

        let trimmed = response.trim().to_lowercase();
        if trimmed == "y" || trimmed == "yes" {
            PermissionDecision::Allow
        } else {
            PermissionDecision::Deny(format!("user denied permission for {tool_name}"))
        }
    }
}

/// Read a line from /dev/tty (works even when stdin is piped).
/// Falls back to stdin if /dev/tty is unavailable.
fn read_tty_line() -> String {
    // Try /dev/tty first so permission prompts work even with piped stdin
    #[cfg(unix)]
    {
        use std::fs::OpenOptions;
        use std::io::BufRead as _;
        if let Ok(tty) = OpenOptions::new().read(true).write(true).open("/dev/tty") {
            let mut reader = io::BufReader::new(tty);
            let mut line = String::new();
            if reader.read_line(&mut line).is_ok() {
                return line;
            }
        }
    }

    // Fallback: read from stdin
    let mut line = String::new();
    let _ = io::stdin().lock().read_line(&mut line);
    line
}

// Bring read_line into scope for the fallback branch
use std::io::BufRead as _;
