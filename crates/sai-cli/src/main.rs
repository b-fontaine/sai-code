//! sai-code: Interactive CLI agent entry point.

mod banner;
mod cli;
mod input;
mod repl;
mod terminal_permissions;
mod terminal_ui;

use color_eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_target(false)
        .init();

    let cli = cli::Cli::parse_args();
    repl::run(cli).await
}
