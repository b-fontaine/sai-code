//! Conversation REPL: read → `run_turn` → display → repeat.

use color_eyre::Result;
use std::io::Write as _;
use std::time::Instant;

use sai_core::domain::config::AgentConfig;
use sai_core::domain::session::SessionMeta;
use sai_core::error::{AgentError, LlmError};
use sai_core::services::agent_loop::AgentLoop;
use sai_llm::GenaiLlmAdapter;
use sai_session::FilesystemSessionAdapter;
use sai_tools::{InMemoryToolRegistry, ToolConfig};

use crate::sessions::{load_for_resume, ResumeResult};

use crate::banner;
use crate::cli::Cli;
use crate::input::{self, InputResult};
use crate::terminal_permissions::TerminalPermissions;
use crate::terminal_ui::TerminalUi;

/// Configuration for the REPL loop behavior.
pub struct ReplConfig {
    /// Text shown before user input.
    pub prompt_prefix: String,
    /// Text shown on normal exit.
    pub farewell_message: String,
    /// Millisecond window for detecting a second Ctrl-C (force-exit).
    pub double_ctrl_c_window_ms: u64,
}

impl Default for ReplConfig {
    fn default() -> Self {
        Self {
            prompt_prefix: "> ".into(),
            farewell_message: "Goodbye!".into(),
            double_ctrl_c_window_ms: 1_000,
        }
    }
}

/// Entry point: wire all adapters and start the REPL using the default
/// plain-text terminal adapters.
pub async fn run(cli: Cli) -> Result<()> {
    let ui = Box::new(TerminalUi);
    let permissions = Box::new(TerminalPermissions::new());
    run_with_ports(cli, ui, permissions).await
}

/// Entry point with pre-constructed port adapters.
///
/// This allows callers (e.g. the TUI) to supply their own `UiPort` and
/// `PermissionPort` implementations while reusing the full REPL loop logic.
#[allow(clippy::too_many_lines)]
pub async fn run_with_ports(
    cli: Cli,
    ui: Box<dyn sai_core::ports::ui::UiPort>,
    permissions: Box<dyn sai_core::ports::permissions::PermissionPort>,
) -> Result<()> {
    let model = &cli.model;

    // Validate and create LLM adapter (fail early on bad model name)
    let llm = GenaiLlmAdapter::new(model)
        .map_err(|e| color_eyre::eyre::eyre!("Failed to initialize LLM adapter: {e}"))?;

    // Build tool registry with the current working directory as root
    let cwd = std::env::current_dir()?;
    let tool_config = ToolConfig::new(cwd.clone());
    let tools = InMemoryToolRegistry::with_defaults(tool_config);

    // Build agent configuration
    let agent_config = AgentConfig {
        model_name: model.clone(),
        ..AgentConfig::default()
    };

    // Display startup banner
    banner::display_banner(model, &cwd);

    // Create session persistence adapter
    let session_adapter = FilesystemSessionAdapter::new();

    // Resolve resume / new session
    let (agent, session_meta) = if let Some(ref resume_arg) = cli.resume {
        // --resume was passed; empty string means "most recent in cwd"
        let resume_id = if resume_arg.is_empty() {
            None
        } else {
            Some(resume_arg.as_str())
        };
        match load_for_resume(resume_id, &cwd).await? {
            ResumeResult::Loaded {
                session_id,
                messages,
                turn_count,
            } => {
                let mut stderr = std::io::stderr();
                let _ = writeln!(
                    stderr,
                    "Resumed session {} ({} turn{}, {})",
                    session_id,
                    turn_count,
                    if turn_count == 1 { "" } else { "s" },
                    format_relative_now(&chrono::Utc::now()),
                );
                let session_meta = SessionMeta {
                    id: session_id,
                    name: cli.session_name.clone(),
                    model_name: model.clone(),
                    working_dir: cwd.clone(),
                    created_at: chrono::Utc::now(),
                    last_active_at: chrono::Utc::now(),
                    turn_count,
                };
                let a = AgentLoop::resume(
                    agent_config,
                    session_id,
                    messages,
                    &llm,
                    &tools,
                    ui.as_ref(),
                    permissions.as_ref(),
                    &session_adapter,
                );
                (a, session_meta)
            }
            ResumeResult::NotFound => {
                // Start fresh (--resume without a match is not an error)
                let session_id = uuid::Uuid::new_v4();
                let session_meta = SessionMeta {
                    id: session_id,
                    name: cli.session_name.clone(),
                    model_name: model.clone(),
                    working_dir: cwd.clone(),
                    created_at: chrono::Utc::now(),
                    last_active_at: chrono::Utc::now(),
                    turn_count: 0,
                };
                let a = AgentLoop::new(
                    agent_config,
                    &llm,
                    &tools,
                    ui.as_ref(),
                    permissions.as_ref(),
                    &session_adapter,
                );
                (a, session_meta)
            }
        }
    } else {
        // Fresh session
        let session_id = uuid::Uuid::new_v4();
        let mut session_meta = SessionMeta::new(session_id, model.clone(), cwd.clone());
        session_meta.name.clone_from(&cli.session_name);
        let a = AgentLoop::new(
            agent_config,
            &llm,
            &tools,
            ui.as_ref(),
            permissions.as_ref(),
            &session_adapter,
        );
        (a, session_meta)
    };

    let mut agent = agent.with_session_meta(session_meta);
    let repl_config = ReplConfig::default();

    // If an initial message was provided via CLI arg, run it as the first turn
    if let Some(ref initial) = cli.message {
        run_turn_with_recovery(&mut agent, initial).await;
    }

    // Track timing of first Ctrl-C during a turn for double-Ctrl-C detection
    let mut first_ctrl_c_at: Option<Instant> = None;

    // Interactive loop
    loop {
        // Race input reading against Ctrl-C so we can exit cleanly at the prompt
        let result = tokio::select! {
            r = input::read_input(&repl_config.prompt_prefix) => r?,
            _ = tokio::signal::ctrl_c() => {
                print_farewell(&repl_config.farewell_message);
                return Ok(());
            }
        };

        match result {
            InputResult::Exit => {
                print_farewell(&repl_config.farewell_message);
                return Ok(());
            }
            InputResult::Empty => {
                // Re-display prompt — no LLM call
            }
            InputResult::Message(msg) => {
                // Race the turn against Ctrl-C so the user can cancel mid-response
                let cancelled = tokio::select! {
                    () = run_turn_with_recovery(&mut agent, &msg) => false,
                    _ = tokio::signal::ctrl_c() => true,
                };

                if cancelled {
                    let now = Instant::now();
                    if let Some(first) = first_ctrl_c_at {
                        if now.duration_since(first).as_millis()
                            < u128::from(repl_config.double_ctrl_c_window_ms)
                        {
                            // Second Ctrl-C within window → force exit
                            print_farewell(&repl_config.farewell_message);
                            return Ok(());
                        }
                    }
                    // First Ctrl-C: cancel turn, return to prompt
                    first_ctrl_c_at = Some(now);
                    let mut stderr = std::io::stderr();
                    let _ = writeln!(stderr, "\n^C");
                } else {
                    // Turn completed normally; reset double-Ctrl-C counter
                    first_ctrl_c_at = None;
                }
            }
        }
    }
}

/// Run a single turn and display a human-readable error without exiting the loop.
///
/// Transient errors (connection failures, rate limits) allow the user to
/// retry. The conversation history is preserved across all errors.
async fn run_turn_with_recovery(agent: &mut AgentLoop<'_>, message: &str) {
    match agent.run_turn(message).await {
        Ok(_) => {}
        Err(AgentError::Llm(LlmError::Connection(msg))) => {
            let mut stderr = std::io::stderr();
            let _ = writeln!(stderr, "\nConnection error: {msg}. You can try again.");
        }
        Err(AgentError::Llm(LlmError::RateLimited { retry_after_secs })) => {
            let mut stderr = std::io::stderr();
            let _ = writeln!(
                stderr,
                "\nRate limited. Please wait {retry_after_secs}s and try again."
            );
        }
        Err(AgentError::IterationLimitExceeded { .. }) => {
            let mut stderr = std::io::stderr();
            let _ = writeln!(stderr, "\nTurn stopped: too many tool iterations.");
        }
        Err(e) => {
            let mut stderr = std::io::stderr();
            let _ = writeln!(stderr, "\nError: {e}");
        }
    }
}

fn print_farewell(message: &str) {
    let mut stderr = std::io::stderr();
    let _ = writeln!(stderr, "{message}");
}

fn format_relative_now(dt: &chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let diff = now.signed_duration_since(*dt);
    if diff.num_seconds() < 60 {
        "just now".to_string()
    } else if diff.num_hours() < 1 {
        format!("{} min ago", diff.num_minutes())
    } else if diff.num_days() < 1 {
        format!("{} hours ago", diff.num_hours())
    } else {
        dt.format("%Y-%m-%d").to_string()
    }
}
