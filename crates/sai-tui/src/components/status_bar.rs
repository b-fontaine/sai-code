use color_eyre::Result;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use std::path::PathBuf;
use tokio::sync::mpsc::UnboundedSender;

use crate::{app::AgentStatus, components::Component, event::Action};

pub struct StatusBar {
    model_name: String,
    working_dir: PathBuf,
    status: AgentStatus,
    action_tx: Option<UnboundedSender<Action>>,
}

impl StatusBar {
    pub fn new(model_name: String, working_dir: PathBuf) -> Self {
        Self {
            model_name,
            working_dir,
            status: AgentStatus::Idle,
            action_tx: None,
        }
    }
}

impl Component for StatusBar {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.action_tx = Some(tx);
        Ok(())
    }

    fn update(&mut self, action: &Action) -> Result<Option<Action>> {
        if let Action::AgentEvent(ev) = action {
            use sai_core::domain::event::AgentEvent;
            match ev.as_ref() {
                AgentEvent::StreamStart => self.status = AgentStatus::Thinking,
                AgentEvent::TextDelta(_) => self.status = AgentStatus::Streaming,
                AgentEvent::TurnComplete | AgentEvent::Error(_) => {
                    self.status = AgentStatus::Idle;
                }
                _ => {}
            }
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let status_str = match self.status {
            AgentStatus::Idle => "idle",
            AgentStatus::Thinking => "thinking\u{2026}",
            AgentStatus::Streaming => "streaming",
            AgentStatus::AwaitingPermission => "awaiting permission",
        };
        let dir = self.working_dir.display().to_string();
        let line = Line::from(vec![
            Span::styled(" sai-code ", Style::default().fg(Color::Cyan)),
            Span::raw("\u{2502} "),
            Span::styled(self.model_name.as_str(), Style::default().fg(Color::Yellow)),
            Span::raw(" \u{2502} "),
            Span::raw(dir),
            Span::raw(" \u{2502} "),
            Span::styled(status_str, Style::default().fg(Color::Green)),
            Span::raw(" "),
        ]);
        frame.render_widget(Paragraph::new(line), area);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    fn make_terminal(w: u16, h: u16) -> Terminal<TestBackend> {
        Terminal::new(TestBackend::new(w, h)).unwrap()
    }

    #[test]
    fn renders_model_and_status() {
        let mut terminal = make_terminal(80, 1);
        let mut bar = StatusBar::new(
            "claude-sonnet-4".to_string(),
            std::path::PathBuf::from("/home/user/project"),
        );
        terminal
            .draw(|frame| {
                bar.draw(frame, frame.area()).unwrap();
            })
            .unwrap();
        let buffer = terminal.backend().buffer().clone();
        let content: String = buffer
            .content()
            .iter()
            .map(|c| c.symbol().to_string())
            .collect::<Vec<_>>()
            .join("");
        assert!(
            content.contains("claude-sonnet-4"),
            "Model name should appear in status bar"
        );
        assert!(
            content.contains("idle"),
            "Status should be 'idle' initially"
        );
    }
}
