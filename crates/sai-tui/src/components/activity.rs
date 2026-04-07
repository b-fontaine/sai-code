use color_eyre::Result;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};
use sai_core::domain::event::AgentEvent;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    app::{ToolActivityEntry, ToolStatus},
    components::Component,
    event::Action,
};

const MAX_ENTRIES: usize = 50;

pub struct ActivityPanel {
    entries: Vec<ToolActivityEntry>,
    action_tx: Option<UnboundedSender<Action>>,
}

impl ActivityPanel {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            action_tx: None,
        }
    }
}

impl Default for ActivityPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for ActivityPanel {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.action_tx = Some(tx);
        Ok(())
    }

    fn update(&mut self, action: &Action) -> Result<Option<Action>> {
        if let Action::AgentEvent(arc_event) = action {
            match arc_event.as_ref() {
                AgentEvent::StreamStart => {
                    self.entries.clear();
                }
                AgentEvent::ToolCallStart { name, call_id } => {
                    self.entries.push(ToolActivityEntry {
                        call_id: call_id.clone(),
                        name: name.clone(),
                        status: ToolStatus::Running,
                        summary: None,
                    });
                    if self.entries.len() > MAX_ENTRIES {
                        self.entries.remove(0);
                    }
                }
                AgentEvent::ToolCallComplete {
                    call_id,
                    success,
                    summary,
                } => {
                    if let Some(entry) = self.entries.iter_mut().find(|e| e.call_id == *call_id) {
                        entry.status = if *success {
                            ToolStatus::Success
                        } else {
                            ToolStatus::Failure
                        };
                        if !summary.is_empty() {
                            entry.summary = Some(summary.clone());
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let block = Block::default().borders(Borders::ALL).title(" Tools ");
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let items: Vec<ListItem> = self
            .entries
            .iter()
            .map(|entry| {
                let (icon, style) = match entry.status {
                    ToolStatus::Running => ("\u{27f3}", Style::default().fg(Color::Yellow)),
                    ToolStatus::Success => ("\u{2713}", Style::default().fg(Color::Green)),
                    ToolStatus::Failure => ("\u{2717}", Style::default().fg(Color::Red)),
                };
                let label = match &entry.summary {
                    Some(s) if entry.status == ToolStatus::Failure => {
                        format!("{icon} {}: {s}", entry.name)
                    }
                    _ => format!("{icon} {}", entry.name),
                };
                ListItem::new(Line::from(Span::styled(label, style)))
            })
            .collect();

        let list = List::new(items);
        frame.render_widget(list, inner);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};
    use sai_core::domain::event::AgentEvent;
    use std::sync::Arc;

    #[test]
    fn empty_panel_renders_without_panic() {
        let mut terminal = Terminal::new(TestBackend::new(40, 10)).unwrap();
        let mut panel = ActivityPanel::new();
        terminal
            .draw(|frame| {
                panel.draw(frame, frame.area()).unwrap();
            })
            .unwrap();
    }

    #[test]
    fn tracks_tool_call_start_and_complete() {
        let mut panel = ActivityPanel::new();

        let start = Action::AgentEvent(Arc::new(AgentEvent::ToolCallStart {
            name: "read_file".into(),
            call_id: "c1".into(),
        }));
        panel.update(&start).unwrap();
        assert_eq!(panel.entries.len(), 1);
        assert_eq!(panel.entries[0].status, ToolStatus::Running);

        let complete = Action::AgentEvent(Arc::new(AgentEvent::ToolCallComplete {
            call_id: "c1".into(),
            success: true,
            summary: "Read 100 bytes".into(),
        }));
        panel.update(&complete).unwrap();
        assert_eq!(panel.entries[0].status, ToolStatus::Success);
    }

    #[test]
    fn clears_on_stream_start() {
        let mut panel = ActivityPanel::new();
        let start = Action::AgentEvent(Arc::new(AgentEvent::ToolCallStart {
            name: "tool_a".into(),
            call_id: "c1".into(),
        }));
        panel.update(&start).unwrap();
        assert_eq!(panel.entries.len(), 1);

        let new_stream = Action::AgentEvent(Arc::new(AgentEvent::StreamStart));
        panel.update(&new_stream).unwrap();
        assert!(panel.entries.is_empty());
    }
}
