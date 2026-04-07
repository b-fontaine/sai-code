//! Terminal UI adapter: renders `AgentEvent`s to stdout/stderr.

use async_trait::async_trait;
use sai_core::domain::event::AgentEvent;
use sai_core::ports::ui::UiPort;
use std::io::Write as _;

/// Concrete `UiPort` implementation that renders events to the terminal.
///
/// - Agent text responses stream to **stdout**.
/// - All metadata (thinking indicator, tool activity, errors, warnings)
///   goes to **stderr** so stdout can be piped cleanly.
pub struct TerminalUi;

#[async_trait]
impl UiPort for TerminalUi {
    async fn emit_event(&self, event: AgentEvent) {
        match event {
            AgentEvent::StreamStart => {
                let mut stderr = std::io::stderr();
                let _ = write!(stderr, "Thinking...");
                let _ = stderr.flush();
            }
            AgentEvent::TextDelta(text) => {
                let mut stdout = std::io::stdout();
                let _ = write!(stdout, "{text}");
                let _ = stdout.flush();
            }
            AgentEvent::ToolCallStart { name, .. } => {
                let mut stderr = std::io::stderr();
                let _ = writeln!(stderr, "\n[tool: {name}]");
            }
            AgentEvent::ToolCallComplete { success, .. } => {
                let mut stderr = std::io::stderr();
                if success {
                    let _ = writeln!(stderr, "✓");
                } else {
                    let _ = writeln!(stderr, "✗");
                }
            }
            AgentEvent::TurnComplete => {
                let mut stdout = std::io::stdout();
                let _ = writeln!(stdout);
                let _ = stdout.flush();
            }
            AgentEvent::Error(err) => {
                let mut stderr = std::io::stderr();
                let _ = writeln!(stderr, "\nError: {err}");
            }
            AgentEvent::HistorySizeWarning { message_count } => {
                let mut stderr = std::io::stderr();
                let _ = writeln!(
                    stderr,
                    "\nNote: conversation history is long ({message_count} messages). \
                     Consider starting a new session."
                );
            }
        }
    }
}
