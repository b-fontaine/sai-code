use color_eyre::Result;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use sai_core::domain::event::AgentEvent;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    app::{ActiveResponse, ConversationEntry},
    components::Component,
    event::Action,
};

pub struct ConversationPanel {
    entries: Vec<ConversationEntry>,
    active: Option<ActiveResponse>,
    scroll_offset: u16,
    auto_scroll: bool,
    visible_height: u16,
    action_tx: Option<UnboundedSender<Action>>,
}

impl ConversationPanel {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            active: None,
            scroll_offset: 0,
            auto_scroll: true,
            visible_height: 20,
            action_tx: None,
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    fn total_line_count(&self) -> u16 {
        let entry_lines: u16 = self
            .entries
            .iter()
            .map(|e| match e {
                ConversationEntry::User { text } => {
                    1u16.max(text.chars().filter(|&c| c == '\n').count() as u16 + 1)
                }
                ConversationEntry::Assistant { lines, .. } => lines.len() as u16,
                ConversationEntry::System { text } => {
                    1u16.max(text.chars().filter(|&c| c == '\n').count() as u16 + 1)
                }
            })
            .sum();
        let active_lines = self.active.as_ref().map_or(0, |a| a.lines.len() as u16);
        entry_lines + active_lines
    }

    fn max_scroll(&self) -> u16 {
        self.total_line_count().saturating_sub(self.visible_height)
    }

    fn build_render_lines(&self) -> Vec<Line<'static>> {
        let mut all_lines: Vec<Line<'static>> = Vec::new();

        for entry in &self.entries {
            match entry {
                ConversationEntry::User { text } => {
                    let mut first = true;
                    for part in text.split('\n') {
                        if first {
                            all_lines.push(Line::from(vec![
                                Span::styled(
                                    "You: ",
                                    Style::default()
                                        .fg(Color::Cyan)
                                        .add_modifier(Modifier::BOLD),
                                ),
                                Span::raw(part.to_owned()),
                            ]));
                            first = false;
                        } else {
                            all_lines.push(Line::from(Span::raw(part.to_owned())));
                        }
                    }
                    all_lines.push(Line::default()); // blank separator
                }
                ConversationEntry::Assistant { lines, .. } => {
                    if let Some(first_line) = lines.first() {
                        let mut label_line = vec![Span::styled(
                            "AI:  ",
                            Style::default()
                                .fg(Color::Green)
                                .add_modifier(Modifier::BOLD),
                        )];
                        label_line.extend(first_line.spans.iter().cloned());
                        all_lines.push(Line::from(label_line));
                        for line in lines.iter().skip(1) {
                            let mut indented = vec![Span::raw("     ")];
                            indented.extend(line.spans.iter().cloned());
                            all_lines.push(Line::from(indented));
                        }
                    }
                    all_lines.push(Line::default());
                }
                ConversationEntry::System { text } => {
                    for part in text.split('\n') {
                        all_lines.push(Line::from(vec![Span::styled(
                            part.to_owned(),
                            Style::default().fg(Color::Yellow),
                        )]));
                    }
                    all_lines.push(Line::default());
                }
            }
        }

        // Active (streaming) response
        if let Some(active) = &self.active {
            if let Some(first_line) = active.lines.first() {
                let mut label_line = vec![Span::styled(
                    "AI:  ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )];
                label_line.extend(first_line.spans.iter().cloned());
                all_lines.push(Line::from(label_line));
                for line in active.lines.iter().skip(1) {
                    let mut indented = vec![Span::raw("     ")];
                    indented.extend(line.spans.iter().cloned());
                    all_lines.push(Line::from(indented));
                }
            }
        }

        all_lines
    }
}

impl Default for ConversationPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for ConversationPanel {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.action_tx = Some(tx);
        Ok(())
    }

    fn update(&mut self, action: &Action) -> Result<Option<Action>> {
        match action {
            Action::AgentEvent(arc_event) => match arc_event.as_ref() {
                AgentEvent::StreamStart => {
                    self.active = Some(ActiveResponse::default());
                    self.entries.push(ConversationEntry::System {
                        text: "Thinking\u{2026}".into(),
                    });
                    self.auto_scroll = true;
                }
                AgentEvent::TextDelta(text) => {
                    // Remove the "Thinking..." placeholder if present
                    if let Some(last) = self.entries.last() {
                        if matches!(last, ConversationEntry::System { text: t } if t == "Thinking\u{2026}")
                        {
                            self.entries.pop();
                        }
                    }
                    let active = self.active.get_or_insert_with(ActiveResponse::default);
                    for (i, part) in text.split('\n').enumerate() {
                        if i > 0 {
                            active.lines.push(Line::default());
                        }
                        if active.lines.is_empty() {
                            active.lines.push(Line::default());
                        }
                        let last = active.lines.last_mut().unwrap();
                        last.spans.push(Span::raw(part.to_owned()));
                    }
                    active.raw_text.push_str(text);
                    if self.auto_scroll {
                        self.scroll_offset = self.max_scroll();
                    }
                }
                AgentEvent::TurnComplete => {
                    if let Some(active) = self.active.take() {
                        self.entries.push(ConversationEntry::Assistant {
                            lines: active.lines,
                            raw_text: active.raw_text,
                        });
                    }
                    self.auto_scroll = true;
                    self.scroll_offset = self.max_scroll();
                }
                AgentEvent::Error(e) => {
                    self.active = None;
                    self.entries.push(ConversationEntry::System {
                        text: format!("Error: {e}"),
                    });
                    if self.auto_scroll {
                        self.scroll_offset = self.max_scroll();
                    }
                }
                AgentEvent::HistorySizeWarning { message_count } => {
                    self.entries.push(ConversationEntry::System {
                        text: format!(
                            "Note: conversation has {message_count} messages. Consider starting a new session."
                        ),
                    });
                }
                _ => {}
            },
            Action::ScrollUp(n) => {
                self.auto_scroll = false;
                self.scroll_offset = self.scroll_offset.saturating_sub(*n);
            }
            Action::ScrollDown(n) => {
                self.auto_scroll = false;
                let new_offset = self.scroll_offset.saturating_add(*n);
                self.scroll_offset = new_offset.min(self.max_scroll());
            }
            Action::ScrollToBottom => {
                self.auto_scroll = true;
                self.scroll_offset = self.max_scroll();
            }
            Action::ClearConversation => {
                self.entries.clear();
                self.active = None;
                self.scroll_offset = 0;
                self.auto_scroll = true;
            }
            Action::SubmitInput => {
                self.auto_scroll = true;
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Conversation ");
        let inner = block.inner(area);
        frame.render_widget(block, area);

        self.visible_height = inner.height;
        if self.auto_scroll {
            self.scroll_offset = self.max_scroll();
        }

        let lines = self.build_render_lines();
        let paragraph = Paragraph::new(lines)
            .scroll((self.scroll_offset, 0))
            .wrap(Wrap { trim: false });
        frame.render_widget(paragraph, inner);
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
        let mut terminal = Terminal::new(TestBackend::new(80, 20)).unwrap();
        let mut panel = ConversationPanel::new();
        terminal
            .draw(|frame| {
                panel.draw(frame, frame.area()).unwrap();
            })
            .unwrap();
    }

    #[test]
    fn text_delta_appends_to_active_response() {
        let mut panel = ConversationPanel::new();
        panel
            .update(&Action::AgentEvent(Arc::new(AgentEvent::StreamStart)))
            .unwrap();
        panel
            .update(&Action::AgentEvent(Arc::new(AgentEvent::TextDelta(
                "Hello".into(),
            ))))
            .unwrap();
        let active = panel.active.as_ref().unwrap();
        assert!(active.raw_text.contains("Hello"));
    }

    #[test]
    fn turn_complete_promotes_active_to_entry() {
        let mut panel = ConversationPanel::new();
        panel
            .update(&Action::AgentEvent(Arc::new(AgentEvent::StreamStart)))
            .unwrap();
        panel
            .update(&Action::AgentEvent(Arc::new(AgentEvent::TextDelta(
                "World".into(),
            ))))
            .unwrap();
        panel
            .update(&Action::AgentEvent(Arc::new(AgentEvent::TurnComplete)))
            .unwrap();
        assert!(panel.active.is_none());
        let has_assistant = panel.entries.iter().any(|e| {
            matches!(e, ConversationEntry::Assistant { raw_text, .. } if raw_text.contains("World"))
        });
        assert!(has_assistant, "Should have Assistant entry with 'World'");
    }

    #[test]
    fn scroll_up_disables_auto_scroll() {
        let mut panel = ConversationPanel::new();
        panel.update(&Action::ScrollUp(3)).unwrap();
        assert!(!panel.auto_scroll);
    }

    #[test]
    fn scroll_to_bottom_re_enables_auto_scroll() {
        let mut panel = ConversationPanel::new();
        panel.update(&Action::ScrollUp(3)).unwrap();
        panel.update(&Action::ScrollToBottom).unwrap();
        assert!(panel.auto_scroll);
    }
}
