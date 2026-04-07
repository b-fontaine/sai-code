use std::sync::{Arc, Mutex};

use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::{
    app::AppState,
    components::Component,
    event::{Action, Event},
};

pub struct PermissionPrompt {
    state: Arc<Mutex<AppState>>,
}

impl PermissionPrompt {
    pub fn new(state: Arc<Mutex<AppState>>) -> Self {
        Self { state }
    }
}

/// Compute a centered rect within `r`.
/// `percent_x`: width as % of `r.width`
/// `percent_y`: height as % of `r.height`
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

impl Component for PermissionPrompt {
    fn handle_events(&mut self, event: Option<&Event>) -> Result<Option<Action>> {
        let Some(Event::Key(key)) = event else {
            return Ok(None);
        };
        if key.kind != KeyEventKind::Press {
            return Ok(None);
        }

        let state = self.state.lock().unwrap();
        let has_permission_prompt = state.pending_permission.is_some();
        let show_help = state.show_help;
        drop(state);

        if has_permission_prompt {
            match key.code {
                KeyCode::Char('y') | KeyCode::Enter => {
                    return Ok(Some(Action::ApprovePermission));
                }
                KeyCode::Char('n') | KeyCode::Esc => {
                    return Ok(Some(Action::DenyPermission));
                }
                _ => {}
            }
        } else if show_help {
            match key.code {
                KeyCode::Char('?' | 'q') | KeyCode::Esc => {
                    return Ok(Some(Action::ToggleHelp));
                }
                _ => {}
            }
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let state = self.state.lock().unwrap();

        if let Some(pending) = &state.pending_permission {
            let popup_area = centered_rect(60, 30, area);
            frame.render_widget(Clear, popup_area);

            let block = Block::default()
                .title(Line::from(vec![Span::styled(
                    " Permission Required ",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                )]))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red));

            let content = format!(
                "Tool: {}\n\nAction:\n{}\n\n[y/Enter] Allow    [n/Esc] Deny",
                pending.tool_name, pending.action_description,
            );
            let paragraph = Paragraph::new(content)
                .block(block)
                .wrap(ratatui::widgets::Wrap { trim: false });
            frame.render_widget(paragraph, popup_area);
        } else if state.show_help {
            let popup_area = centered_rect(70, 60, area);
            frame.render_widget(Clear, popup_area);

            let block = Block::default()
                .title(" Keyboard Shortcuts ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan));

            let items: Vec<ListItem> = vec![
                ListItem::new("  ?           Toggle this help"),
                ListItem::new("  Enter       Submit message"),
                ListItem::new("  Ctrl-C      Exit"),
                ListItem::new("  Ctrl-Q      Exit"),
                ListItem::new("  PageUp / k  Scroll conversation up"),
                ListItem::new("  PageDown / j Scroll conversation down"),
                ListItem::new("  End / G     Scroll to bottom"),
                ListItem::new("  /exit       Exit via command"),
                ListItem::new("  /quit       Exit via command"),
                ListItem::new("  Esc         Close this overlay"),
            ];
            let list = List::new(items).block(block);
            frame.render_widget(list, popup_area);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use ratatui::{backend::TestBackend, Terminal};
    use std::sync::{Arc, Mutex};

    fn make_state() -> Arc<Mutex<AppState>> {
        Arc::new(Mutex::new(AppState::default()))
    }

    #[test]
    fn no_overlay_when_no_pending_permission() {
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        let state = make_state();
        let mut prompt = PermissionPrompt::new(Arc::clone(&state));
        terminal
            .draw(|frame| {
                prompt.draw(frame, frame.area()).unwrap();
            })
            .unwrap();
        let buffer = terminal.backend().buffer().clone();
        let content: String = buffer
            .content()
            .iter()
            .map(|c| c.symbol().to_string())
            .collect::<Vec<_>>()
            .join("");
        assert!(!content.contains("Permission Required"));
    }
}
