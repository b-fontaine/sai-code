//! CLI argument definitions for sai-code.

use clap::Parser;

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
}

impl Cli {
    /// Parse arguments from the process environment.
    pub fn parse_args() -> Self {
        Self::parse()
    }
}
