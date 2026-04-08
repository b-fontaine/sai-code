//! Session management commands: list, show, delete.

use std::io::{self, IsTerminal, Write as _};
use std::path::PathBuf;

use color_eyre::eyre::{bail, eyre};
use color_eyre::Result;

use sai_core::domain::message::Message;
use sai_core::domain::session::SessionMeta;
use sai_core::ports::session::SessionPort;
use sai_session::FilesystemSessionAdapter;

// ── List ──────────────────────────────────────────────────────────────────────

/// Print a table of saved sessions to stdout.
pub async fn cmd_list(dir_filter: Option<PathBuf>, limit: usize) -> Result<()> {
    let adapter = FilesystemSessionAdapter::new();
    let mut sessions = adapter.list_sessions().await?;

    if let Some(ref filter) = dir_filter {
        let filter = filter.canonicalize().unwrap_or_else(|_| filter.clone());
        sessions.retain(|s| {
            s.working_dir
                .canonicalize()
                .unwrap_or_else(|_| s.working_dir.clone())
                == filter
        });
    }

    sessions.truncate(limit);

    if sessions.is_empty() {
        println!("No saved sessions.");
        return Ok(());
    }

    // If stdout is not a terminal, emit JSON
    if !io::stdout().is_terminal() {
        let json = serde_json::to_string_pretty(&sessions)?;
        println!("{json}");
        return Ok(());
    }

    // Human-readable table
    println!(
        "{:<38}  {:<20}  {:>5}  {:<16}  DIR",
        "ID", "NAME", "TURNS", "LAST ACTIVE"
    );
    println!("{}", "-".repeat(100_usize));

    for s in &sessions {
        let name = s.name.as_deref().unwrap_or("—");
        let last = format_relative(s.last_active_at);
        let dir = home_relative(&s.working_dir);
        println!(
            "{:<38}  {:<20}  {:>5}  {:<16}  {}",
            s.id, name, s.turn_count, last, dir
        );
    }

    Ok(())
}

// ── Show ──────────────────────────────────────────────────────────────────────

/// Print full details of one session.
pub async fn cmd_show(session_id: &str) -> Result<()> {
    let adapter = FilesystemSessionAdapter::new();
    let meta = resolve_session(&adapter, session_id).await?;

    let loaded = adapter
        .load_session(meta.id)
        .await?
        .ok_or_else(|| eyre!("Session {session_id} not found"))?;

    println!("Session: {}", loaded.meta.id);
    println!("Name:    {}", loaded.meta.name.as_deref().unwrap_or("—"));
    println!("Model:   {}", loaded.meta.model_name);
    println!("Dir:     {}", home_relative(&loaded.meta.working_dir));
    println!(
        "Created: {}",
        loaded.meta.created_at.format("%Y-%m-%d %H:%M:%S UTC")
    );
    println!(
        "Active:  {}",
        loaded.meta.last_active_at.format("%Y-%m-%d %H:%M:%S UTC")
    );
    println!("Turns:   {}", loaded.meta.turn_count);
    println!();

    for turn in &loaded.turns {
        let user_msg = if turn.user_message.len() > 80 {
            format!("{}…", &turn.user_message[..79])
        } else {
            turn.user_message.clone()
        };
        println!(
            "Turn {} ({}): {}",
            turn.turn_index + 1,
            turn.completed_at.format("%Y-%m-%d %H:%M:%S"),
            user_msg
        );
    }

    Ok(())
}

// ── Delete ────────────────────────────────────────────────────────────────────

/// Delete a specific session or all sessions.
pub async fn cmd_delete(session_id: Option<&str>, delete_all: bool) -> Result<()> {
    let adapter = FilesystemSessionAdapter::new();

    if delete_all {
        let all = adapter.list_sessions().await?;
        if all.is_empty() {
            println!("No saved sessions to delete.");
            return Ok(());
        }

        // Require interactive confirmation
        if !io::stdin().is_terminal() {
            bail!(
                "Cannot delete all sessions in non-interactive mode. \
                 Run interactively and confirm with 'y'."
            );
        }

        print!(
            "Delete all {} session(s)? This cannot be undone. [y/N]: ",
            all.len()
        );
        io::stdout().flush()?;

        let mut response = String::new();
        io::stdin().read_line(&mut response)?;
        if response.trim().to_lowercase() != "y" {
            println!("Aborted.");
            return Ok(());
        }

        for s in &all {
            adapter.delete_session(s.id).await?;
        }
        println!("Deleted {} session(s).", all.len());
        return Ok(());
    }

    let id_str = session_id.ok_or_else(|| eyre!("Provide a session ID or --all"))?;
    let meta = resolve_session(&adapter, id_str).await?;
    adapter.delete_session(meta.id).await?;
    println!("Deleted session {}.", meta.id);
    Ok(())
}

// ── Resume helpers ────────────────────────────────────────────────────────────

/// Result of attempting to load a prior session for resume.
pub enum ResumeResult {
    /// A session was found; these are the prior messages.
    Loaded {
        session_id: uuid::Uuid,
        messages: Vec<Message>,
        turn_count: usize,
    },
    /// No matching session was found; start fresh.
    NotFound,
}

/// Load a session for resume based on `--resume` arg value.
///
/// - `None` → resume most recent in current directory
/// - `Some(id_or_name)` → find by UUID or name
pub async fn load_for_resume(arg: Option<&str>, cwd: &std::path::Path) -> Result<ResumeResult> {
    let adapter = FilesystemSessionAdapter::new();

    let meta = if let Some(id_str) = arg {
        // Try UUID first, then name
        match uuid::Uuid::parse_str(id_str) {
            Ok(id) => adapter.load_session(id).await?.map(|s| s.meta),
            Err(_) => adapter.find_by_name(id_str).await?,
        }
    } else {
        // Resume most recent in current directory
        let sessions = adapter.list_sessions().await?;
        let cwd_canonical = cwd.canonicalize().unwrap_or_else(|_| cwd.to_path_buf());
        sessions.into_iter().find(|s| {
            s.working_dir
                .canonicalize()
                .unwrap_or_else(|_| s.working_dir.clone())
                == cwd_canonical
        })
    };

    let Some(meta) = meta else {
        return Ok(ResumeResult::NotFound);
    };

    let loaded = adapter
        .load_session(meta.id)
        .await?
        .ok_or_else(|| eyre!("Session {} disappeared during load", meta.id))?;

    let turn_count = loaded.turns.len();
    let messages = loaded.into_messages();

    Ok(ResumeResult::Loaded {
        session_id: meta.id,
        messages,
        turn_count,
    })
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Resolve a session by UUID string or by name.
async fn resolve_session(adapter: &FilesystemSessionAdapter, id_str: &str) -> Result<SessionMeta> {
    // Try UUID parse first
    if let Ok(id) = uuid::Uuid::parse_str(id_str) {
        let loaded = adapter
            .load_session(id)
            .await?
            .ok_or_else(|| eyre!("Session '{id_str}' not found"))?;
        return Ok(loaded.meta);
    }

    // Try by name
    adapter
        .find_by_name(id_str)
        .await?
        .ok_or_else(|| eyre!("No session found with name '{id_str}'"))
}

fn format_relative(dt: chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let diff = now.signed_duration_since(dt);
    if diff.num_seconds() < 60 {
        "just now".to_string()
    } else if diff.num_minutes() < 60 {
        format!("{} min ago", diff.num_minutes())
    } else if diff.num_hours() < 24 {
        format!("{} hours ago", diff.num_hours())
    } else if diff.num_days() < 7 {
        format!("{} days ago", diff.num_days())
    } else {
        dt.format("%Y-%m-%d").to_string()
    }
}

fn home_relative(path: &std::path::Path) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Ok(rel) = path.strip_prefix(&home) {
            return format!("~/{}", rel.display());
        }
    }
    path.display().to_string()
}
