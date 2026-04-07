use std::sync::{Arc, Mutex};
use std::time::Duration;

use color_eyre::Result;
use crossterm::event::EventStream;
use futures::StreamExt;
use ratatui::layout::{Constraint, Direction, Layout};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use crate::app::{AppState, TuiConfig};
use crate::components::Component;
use crate::components::{
    activity::ActivityPanel, conversation::ConversationPanel, input_area::InputArea,
    permission_prompt::PermissionPrompt, status_bar::StatusBar,
};
use crate::event::{Action, Event};
use crate::terminal::{self, Tui};

/// Errors that can occur in the TUI layer.
#[derive(Debug, thiserror::Error)]
pub enum TuiError {
    #[error("terminal initialization failed: {0}")]
    Init(#[from] std::io::Error),

    #[error("terminal is too small ({width}x{height}); minimum is {min_width}x{min_height}")]
    TerminalTooSmall {
        width: u16,
        height: u16,
        min_width: u16,
        min_height: u16,
    },

    #[error("render error: {0}")]
    Render(String),
}

/// The main TUI application that drives the event loop and rendering.
pub struct TuiApp {
    config: TuiConfig,
    state: Arc<Mutex<AppState>>,
    agent_event_tx: UnboundedSender<Event>,
    agent_event_rx: Option<UnboundedReceiver<Event>>,
}

impl TuiApp {
    /// Create a new TUI application with the given configuration.
    pub fn new(config: TuiConfig) -> Self {
        let state = Arc::new(Mutex::new(AppState::new(&config)));
        let (agent_event_tx, agent_event_rx) = mpsc::unbounded_channel();
        Self {
            config,
            state,
            agent_event_tx,
            agent_event_rx: Some(agent_event_rx),
        }
    }

    /// Return a UI adapter that routes `AgentEvent`s into this TUI.
    pub fn ui_adapter(&self) -> crate::adapters::ui::TuiUiAdapter {
        crate::adapters::ui::TuiUiAdapter::new(self.agent_event_tx.clone())
    }

    /// Return a permissions adapter that shows permission prompts in this TUI.
    pub fn permissions_adapter(&self) -> crate::adapters::permissions::TuiPermissionsAdapter {
        use std::io::IsTerminal;
        crate::adapters::permissions::TuiPermissionsAdapter::new(
            Arc::clone(&self.state),
            std::io::stdin().is_terminal(),
        )
    }

    /// Run the TUI event loop until the user quits.
    pub async fn run(&mut self) -> Result<()> {
        terminal::install_panic_hook();
        let mut tui = terminal::init_terminal()?;

        // Check minimum terminal size
        let size = tui.size().map_err(TuiError::Init)?;
        if size.width < self.config.min_width || size.height < self.config.min_height {
            terminal::restore_terminal(&mut tui)?;
            return Err(TuiError::TerminalTooSmall {
                width: size.width,
                height: size.height,
                min_width: self.config.min_width,
                min_height: self.config.min_height,
            }
            .into());
        }

        let result = self.event_loop(&mut tui).await;
        terminal::restore_terminal(&mut tui)?;
        result
    }

    async fn event_loop(&mut self, tui: &mut Tui) -> Result<()> {
        let tick_delay = Duration::from_secs_f64(1.0 / self.config.tick_rate);
        let render_delay = Duration::from_secs_f64(1.0 / self.config.frame_rate);

        // Set up action channel
        let (action_tx, mut action_rx) = mpsc::unbounded_channel::<Action>();

        // Create components
        let mut components: Vec<Box<dyn Component>> = vec![
            Box::new(StatusBar::new(
                self.config.model_name.clone(),
                self.config.working_dir.clone(),
            )),
            Box::new(ConversationPanel::new()),
            Box::new(ActivityPanel::new()),
            Box::new(InputArea::new()),
            Box::new(PermissionPrompt::new(Arc::clone(&self.state))),
        ];

        for component in &mut components {
            component.register_action_handler(action_tx.clone())?;
        }

        let mut event_rx = spawn_event_reader(tick_delay, render_delay);

        let mut agent_event_rx = self
            .agent_event_rx
            .take()
            .expect("event_loop called more than once");

        loop {
            let event = tokio::select! {
                Some(e) = agent_event_rx.recv() => e,
                Some(e) = event_rx.recv() => e,
            };

            if handle_global_keys(&event, &action_tx)? {
                // Global key was handled; continue processing below.
            }

            if let Event::Resize(w, h) = &event {
                if *w < self.config.min_width || *h < self.config.min_height {
                    tui.draw(|frame| {
                        let msg = ratatui::widgets::Paragraph::new(
                            "Terminal too small \u{2014} please resize",
                        );
                        frame.render_widget(msg, frame.area());
                    })?;
                    continue;
                }
                tui.resize(ratatui::layout::Rect::new(0, 0, *w, *h))?;
            }

            for component in &mut components {
                if let Some(action) = component.handle_events(Some(&event))? {
                    action_tx.send(action)?;
                }
            }

            while let Ok(action) = action_rx.try_recv() {
                if matches!(action, Action::Quit) {
                    return Ok(());
                }
                self.handle_action(&action);
                for component in &mut components {
                    if let Some(chained) = component.update(&action)? {
                        action_tx.send(chained)?;
                    }
                }
            }

            if matches!(event, Event::Render) {
                let state = self.state.lock().unwrap();
                render_all(tui, &mut components, &state)?;
            }
        }
    }

    /// Process a single action against the shared `AppState`.
    fn handle_action(&self, action: &Action) {
        match action {
            Action::ToggleHelp => {
                let mut state = self.state.lock().unwrap();
                state.show_help = !state.show_help;
            }
            Action::ClearConversation => {
                let mut state = self.state.lock().unwrap();
                state.conversation.clear();
                state.active_response = None;
            }
            Action::ApprovePermission => {
                let mut state = self.state.lock().unwrap();
                if let Some(pending) = state.pending_permission.take() {
                    state.status = crate::app::AgentStatus::Streaming;
                    let _ = pending
                        .response_tx
                        .send(sai_core::ports::permissions::PermissionDecision::Allow);
                }
            }
            Action::DenyPermission => {
                let mut state = self.state.lock().unwrap();
                if let Some(pending) = state.pending_permission.take() {
                    state.status = crate::app::AgentStatus::Idle;
                    let _ = pending.response_tx.send(
                        sai_core::ports::permissions::PermissionDecision::Deny(
                            "user denied permission".into(),
                        ),
                    );
                }
            }
            Action::SubmitInput => {
                let mut state = self.state.lock().unwrap();
                let text = state.input_buffer.trim().to_owned();
                if !text.is_empty() {
                    state
                        .conversation
                        .push(crate::app::ConversationEntry::User { text });
                    state.input_buffer.clear();
                    state.auto_scroll = true;
                }
            }
            _ => {}
        }
    }
}

/// Spawn a background task that reads crossterm events and emits tick/render
/// events at the configured rates.
fn spawn_event_reader(tick_delay: Duration, render_delay: Duration) -> UnboundedReceiver<Event> {
    let (tx, rx) = mpsc::unbounded_channel::<Event>();
    tokio::spawn(async move {
        let mut reader = EventStream::new();
        let mut tick_interval = tokio::time::interval(tick_delay);
        let mut render_interval = tokio::time::interval(render_delay);
        loop {
            let crossterm_event = reader.next();
            tokio::select! {
                maybe_event = crossterm_event => {
                    match maybe_event {
                        Some(Ok(crossterm::event::Event::Key(k))) => {
                            let _ = tx.send(Event::Key(k));
                        }
                        Some(Ok(crossterm::event::Event::Resize(w, h))) => {
                            let _ = tx.send(Event::Resize(w, h));
                        }
                        Some(Ok(crossterm::event::Event::Mouse(m))) => {
                            let _ = tx.send(Event::Mouse(m));
                        }
                        Some(Err(_)) => {
                            let _ = tx.send(Event::Error);
                        }
                        _ => {}
                    }
                }
                _ = tick_interval.tick() => {
                    let _ = tx.send(Event::Tick);
                }
                _ = render_interval.tick() => {
                    let _ = tx.send(Event::Render);
                }
            }
        }
    });
    rx
}

/// Check for global key bindings (Ctrl-C, Ctrl-Q). Returns `true` if a
/// global key was matched.
fn handle_global_keys(event: &Event, action_tx: &UnboundedSender<Action>) -> Result<bool> {
    if let Event::Key(key) = event {
        use crossterm::event::{KeyCode, KeyModifiers};
        if key.kind == crossterm::event::KeyEventKind::Press {
            if let (KeyModifiers::CONTROL, KeyCode::Char('c' | 'q')) = (key.modifiers, key.code) {
                action_tx.send(Action::Quit)?;
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn render_all(
    tui: &mut Tui,
    components: &mut [Box<dyn Component>],
    _state: &AppState,
) -> Result<()> {
    tui.draw(|frame| {
        let area = frame.area();

        // Check minimum size
        if area.width < 10 || area.height < 5 {
            frame.render_widget(ratatui::widgets::Paragraph::new("Terminal too small"), area);
            return;
        }

        // Layout: status bar (1) | middle (fill) | input (3)
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // status bar
                Constraint::Min(5),    // main area
                Constraint::Length(3), // input area
            ])
            .split(area);

        // Middle area: conversation (70%) | activity (30%)
        let middle_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(70), // conversation
                Constraint::Percentage(30), // activity
            ])
            .split(chunks[1]);

        // Render components in order: status_bar, conversation, activity, input, permission
        let _ = components[0].draw(frame, chunks[0]); // StatusBar
        let _ = components[1].draw(frame, middle_chunks[0]); // ConversationPanel
        let _ = components[2].draw(frame, middle_chunks[1]); // ActivityPanel
        let _ = components[3].draw(frame, chunks[2]); // InputArea
        let _ = components[4].draw(frame, area); // PermissionPrompt overlay
    })?;
    Ok(())
}
