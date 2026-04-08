//! CLI argument definitions for sai-code.

use clap::{Parser, Subcommand};

/// Interactive AI coding agent.
#[derive(Debug, Parser)]
#[command(name = "sai-code", version, about)]
pub struct Cli {
    /// Initial message to process before entering interactive mode.
    pub message: Option<String>,

    /// LLM model identifier.
    #[arg(long, env = "SAI_MODEL", default_value = "claude-sonnet-4")]
    pub model: String,

    /// Enable verbose logging output.
    #[arg(long, short)]
    pub verbose: bool,

    /// Resume a prior session. If `SESSION_ID` is not given, resumes the most
    /// recent session in the current directory.
    #[arg(long, value_name = "SESSION_ID", num_args = 0..=1, default_missing_value = "")]
    pub resume: Option<String>,

    /// Assign a human-readable name to this session.
    /// Allowed characters: letters, digits, hyphens, underscores.
    #[arg(long, value_name = "NAME", value_parser = validate_session_name)]
    pub session_name: Option<String>,

    /// Session management subcommands.
    #[command(subcommand)]
    pub command: Option<SessionCommand>,
}

/// Session management subcommands.
#[derive(Debug, Subcommand)]
pub enum SessionCommand {
    /// List, show, or delete saved sessions.
    Sessions {
        #[command(subcommand)]
        action: SessionAction,
    },
}

/// Actions for session management.
#[derive(Debug, Subcommand)]
pub enum SessionAction {
    /// List saved sessions (most recent first).
    List {
        /// Filter to sessions started in this directory.
        #[arg(long)]
        dir: Option<std::path::PathBuf>,
        /// Maximum number of sessions to show.
        #[arg(long, default_value = "20")]
        limit: usize,
    },
    /// Show full details of a session.
    Show {
        /// Session UUID or name.
        session_id: String,
    },
    /// Delete one or all sessions.
    Delete {
        /// Session UUID or name to delete.
        session_id: Option<String>,
        /// Delete ALL sessions (requires confirmation).
        #[arg(long)]
        all: bool,
    },
}

fn validate_session_name(name: &str) -> Result<String, String> {
    if name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        Ok(name.to_string())
    } else {
        Err(format!(
            "Session name '{name}' contains invalid characters. \
             Only letters, digits, hyphens (-), and underscores (_) are allowed."
        ))
    }
}

impl Cli {
    /// Parse arguments from the process environment.
    pub fn parse_args() -> Self {
        Self::parse()
    }
}
