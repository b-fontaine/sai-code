//! sai-code: Interactive CLI agent entry point.

mod banner;
mod cli;
mod input;
mod repl;
mod terminal_permissions;
mod terminal_ui;

use std::io::IsTerminal;

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

    if std::io::stdin().is_terminal() {
        // Interactive mode: use the rich TUI interface.
        let mut tui_app = sai_tui::TuiApp::new(sai_tui::TuiConfig {
            model_name: cli.model.clone(),
            working_dir: std::env::current_dir()?,
            ..sai_tui::TuiConfig::default()
        });
        let ui = tui_app.ui_adapter();
        let perms = tui_app.permissions_adapter();

        // Spawn the agent REPL as a background task; the TUI owns the main
        // event loop and will block until the user quits.
        tokio::spawn(async move {
            if let Err(e) = repl::run_with_ports(cli, Box::new(ui), Box::new(perms)).await {
                tracing::error!("Agent error: {e}");
            }
        });

        tui_app.run().await?;
    } else {
        // Non-interactive (piped) mode: use plain-text adapters.
        repl::run(cli).await?;
    }

    Ok(())
}
