use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    components::Component,
    event::{Action, Event},
};

pub struct InputArea {
    buffer: String,
    action_tx: Option<UnboundedSender<Action>>,
}

impl InputArea {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            action_tx: None,
        }
    }

    pub fn buffer(&self) -> &str {
        &self.buffer
    }
}

impl Default for InputArea {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for InputArea {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.action_tx = Some(tx);
        Ok(())
    }

    fn handle_events(&mut self, event: Option<&Event>) -> Result<Option<Action>> {
        let Some(Event::Key(key)) = event else {
            return Ok(None);
        };
        if key.kind != KeyEventKind::Press {
            return Ok(None);
        }
        match key.code {
            KeyCode::Enter => {
                if !self.buffer.trim().is_empty() {
                    return Ok(Some(Action::SubmitInput));
                }
            }
            KeyCode::Backspace => {
                self.buffer.pop();
            }
            KeyCode::PageUp => return Ok(Some(Action::ScrollUp(5))),
            KeyCode::PageDown => return Ok(Some(Action::ScrollDown(5))),
            KeyCode::End => return Ok(Some(Action::ScrollToBottom)),
            KeyCode::Char('k') if self.buffer.is_empty() => {
                return Ok(Some(Action::ScrollUp(3)));
            }
            KeyCode::Char('j') if self.buffer.is_empty() => {
                return Ok(Some(Action::ScrollDown(3)));
            }
            KeyCode::Char('G') if self.buffer.is_empty() => {
                return Ok(Some(Action::ScrollToBottom));
            }
            KeyCode::Char('?') if self.buffer.is_empty() => {
                return Ok(Some(Action::ToggleHelp));
            }
            KeyCode::Char(c) => {
                self.buffer.push(c);
                return Ok(Some(Action::AppendInputChar(c)));
            }
            _ => {}
        }
        Ok(None)
    }

    fn update(&mut self, action: &Action) -> Result<Option<Action>> {
        match action {
            Action::ClearInput | Action::SubmitInput => {
                self.buffer.clear();
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray))
            .title(" Input (Enter to send, /exit to quit) ");
        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(Paragraph::new(self.buffer.as_str()), inner);
        // Place cursor at end of input
        let cursor_x = inner
            .x
            .saturating_add(u16::try_from(self.buffer.len()).unwrap_or(u16::MAX));
        frame.set_cursor_position((cursor_x, inner.y));
        Ok(())
    }
}
