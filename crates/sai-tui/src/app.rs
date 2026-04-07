use std::path::PathBuf;

use sai_core::ports::permissions::PermissionDecision;
use tokio::sync::oneshot;

/// Current operational phase of the agent loop.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum AgentStatus {
    #[default]
    Idle,
    Thinking,
    Streaming,
    AwaitingPermission,
}

/// Construction-time configuration for the TUI.
#[derive(Debug, Clone)]
pub struct TuiConfig {
    pub frame_rate: f64,
    pub tick_rate: f64,
    pub min_width: u16,
    pub min_height: u16,
    pub model_name: String,
    pub working_dir: PathBuf,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            frame_rate: 30.0,
            tick_rate: 4.0,
            min_width: 80,
            min_height: 24,
            model_name: "claude-sonnet-4".into(),
            working_dir: std::env::current_dir().unwrap_or_default(),
        }
    }
}

/// One completed exchange in the conversation history.
#[derive(Debug, Clone)]
pub enum ConversationEntry {
    User {
        text: String,
    },
    Assistant {
        lines: Vec<ratatui::text::Line<'static>>,
        raw_text: String,
    },
    System {
        text: String,
    },
}

/// The currently-streaming AI response being built token by token.
#[derive(Debug, Default, Clone)]
pub struct ActiveResponse {
    pub lines: Vec<ratatui::text::Line<'static>>,
    pub raw_text: String,
    pub visible_height: u16,
}

impl ActiveResponse {
    pub fn push_token(&mut self, token: &str) {
        self.raw_text.push_str(token);
        self.rebuild_lines();
    }

    /// Rebuild lines from `raw_text` (call after mutations).
    pub fn rebuild_lines(&mut self) {
        self.lines.clear();
        for line_str in self.raw_text.split('\n') {
            self.lines
                .push(ratatui::text::Line::from(line_str.to_owned()));
        }
    }
}

/// Status of a single tool execution event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolStatus {
    Running,
    Success,
    Failure,
}

/// One tool execution event displayed in the activity panel.
#[derive(Debug, Clone)]
pub struct ToolActivityEntry {
    pub call_id: String,
    pub name: String,
    pub status: ToolStatus,
    pub summary: Option<String>,
}

/// A permission request waiting for the user's y/n keypress.
pub struct PendingPermission {
    pub tool_name: String,
    pub action_description: String,
    pub response_tx: oneshot::Sender<PermissionDecision>,
}

// Manual Debug impl because oneshot::Sender is not Debug.
impl std::fmt::Debug for PendingPermission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingPermission")
            .field("tool_name", &self.tool_name)
            .field("action_description", &self.action_description)
            .finish_non_exhaustive()
    }
}

/// The root shared state for the TUI.
#[derive(Debug, Default)]
pub struct AppState {
    pub conversation: Vec<ConversationEntry>,
    pub active_response: Option<ActiveResponse>,
    pub tool_activity: Vec<ToolActivityEntry>,
    pub pending_permission: Option<PendingPermission>,
    pub input_buffer: String,
    pub status: AgentStatus,
    pub model_name: String,
    pub working_dir: PathBuf,
    pub scroll_offset: u16,
    pub auto_scroll: bool,
    pub should_quit: bool,
    pub error_message: Option<String>,
    pub show_help: bool,
}

impl AppState {
    pub fn new(config: &TuiConfig) -> Self {
        Self {
            model_name: config.model_name.clone(),
            working_dir: config.working_dir.clone(),
            auto_scroll: true,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_state_is_idle() {
        let config = TuiConfig::default();
        let state = AppState::new(&config);
        assert_eq!(state.status, AgentStatus::Idle);
        assert!(state.auto_scroll);
        assert!(state.conversation.is_empty());
        assert!(state.pending_permission.is_none());
    }

    #[test]
    fn default_tui_config_has_sensible_values() {
        let config = TuiConfig::default();
        assert_eq!(config.frame_rate, 30.0);
        assert_eq!(config.tick_rate, 4.0);
        assert_eq!(config.min_width, 80);
        assert_eq!(config.min_height, 24);
    }

    #[test]
    fn agent_status_default_is_idle() {
        assert_eq!(AgentStatus::default(), AgentStatus::Idle);
    }
}
