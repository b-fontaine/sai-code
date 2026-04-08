//! Filesystem-backed `SessionPort` implementation.
//!
//! Each session lives in `{base_dir}/{uuid}/` with two files:
//! - `meta.json`    — session metadata, atomically rewritten after each turn
//! - `turns.jsonl`  — append-only JSON-lines file; one `ConversationTurn` per line

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt as _;
use std::path::PathBuf;

use async_trait::async_trait;
use sai_core::domain::session::{ConversationTurn, PersistedSession, SessionMeta};
use sai_core::error::SessionError;
use sai_core::ports::session::SessionPort;
use uuid::Uuid;

/// Adapter that persists sessions to the local filesystem.
pub struct FilesystemSessionAdapter {
    base_dir: PathBuf,
}

impl FilesystemSessionAdapter {
    /// Create a new adapter, resolving the base directory from:
    /// 1. `SAI_SESSION_DIR` environment variable (if set)
    /// 2. Platform data directory via `dirs::data_dir()`
    /// 3. Fallback: `~/.sai/sessions`
    pub fn new() -> Self {
        let base_dir = if let Ok(dir) = std::env::var("SAI_SESSION_DIR") {
            PathBuf::from(dir)
        } else if let Some(data_dir) = dirs::data_dir() {
            data_dir.join("sai").join("sessions")
        } else {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".sai")
                .join("sessions")
        };
        Self { base_dir }
    }

    /// Create an adapter pointing at a custom base directory (for testing).
    pub fn with_base_dir(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    fn session_dir(&self, id: Uuid) -> PathBuf {
        self.base_dir.join(id.to_string())
    }

    fn meta_path(&self, id: Uuid) -> PathBuf {
        self.session_dir(id).join("meta.json")
    }

    fn turns_path(&self, id: Uuid) -> PathBuf {
        self.session_dir(id).join("turns.jsonl")
    }
}

impl Default for FilesystemSessionAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SessionPort for FilesystemSessionAdapter {
    async fn create_session(&self, meta: SessionMeta) -> Result<(), SessionError> {
        use tokio::io::AsyncWriteExt as _;

        let dir = self.session_dir(meta.id);

        // Idempotent: if the directory already exists, we're done
        if dir.exists() {
            return Ok(());
        }

        // Check for name conflict before creating anything
        if let Some(ref name) = meta.name {
            if let Some(_existing) = self.find_by_name(name).await? {
                return Err(SessionError::NameConflict { name: name.clone() });
            }
        }

        // Create the session directory with mode 0700
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(SessionError::Io)?;
        tokio::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))
            .await
            .map_err(SessionError::Io)?;

        // Write meta.json atomically with mode 0600
        let json = serde_json::to_vec_pretty(&meta).map_err(SessionError::Serialization)?;
        let tmp = self.meta_path(meta.id).with_extension("json.tmp");
        {
            let mut f = tokio::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&tmp)
                .await
                .map_err(SessionError::Io)?;
            f.write_all(&json).await.map_err(SessionError::Io)?;
            f.flush().await.map_err(SessionError::Io)?;
        }
        tokio::fs::rename(&tmp, self.meta_path(meta.id))
            .await
            .map_err(SessionError::Io)?;

        Ok(())
    }

    async fn save_turn(
        &self,
        session_id: Uuid,
        turn: ConversationTurn,
    ) -> Result<(), SessionError> {
        use tokio::io::AsyncWriteExt as _;

        let dir = self.session_dir(session_id);
        if !dir.exists() {
            return Err(SessionError::NotFound { id: session_id });
        }

        // Append turn as single JSON line to turns.jsonl
        let mut line = serde_json::to_string(&turn).map_err(SessionError::Serialization)?;
        line.push('\n');

        let turns_path = self.turns_path(session_id);
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .mode(0o600)
            .open(&turns_path)
            .await
            .map_err(SessionError::Io)?;
        file.write_all(line.as_bytes())
            .await
            .map_err(SessionError::Io)?;
        file.flush().await.map_err(SessionError::Io)?;

        // Update metadata atomically
        let mut meta = self.read_meta(session_id).await?;
        meta.turn_count += 1;
        meta.last_active_at = chrono::Utc::now();
        self.write_meta_atomic(&meta).await?;

        Ok(())
    }

    async fn load_session(
        &self,
        session_id: Uuid,
    ) -> Result<Option<PersistedSession>, SessionError> {
        let dir = self.session_dir(session_id);
        if !dir.exists() {
            return Ok(None);
        }

        let meta = self.read_meta(session_id).await?;
        let turns = self.read_turns(session_id).await?;

        Ok(Some(PersistedSession { meta, turns }))
    }

    async fn list_sessions(&self) -> Result<Vec<SessionMeta>, SessionError> {
        // Ensure the base directory exists
        if !self.base_dir.exists() {
            return Ok(Vec::new());
        }

        let mut entries = tokio::fs::read_dir(&self.base_dir)
            .await
            .map_err(SessionError::Io)?;

        let mut sessions = Vec::new();

        while let Some(entry) = entries.next_entry().await.map_err(SessionError::Io)? {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            // Try to parse the directory name as a UUID
            let dir_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };
            let session_id = match Uuid::parse_str(&dir_name) {
                Ok(id) => id,
                Err(_) => continue,
            };

            match self.read_meta(session_id).await {
                Ok(meta) => sessions.push(meta),
                Err(e) => {
                    tracing::warn!(
                        session_id = %session_id,
                        error = %e,
                        "Skipping corrupted session in list"
                    );
                }
            }
        }

        // Sort most-recently-active first
        sessions.sort_by(|a, b| b.last_active_at.cmp(&a.last_active_at));
        Ok(sessions)
    }

    async fn delete_session(&self, session_id: Uuid) -> Result<bool, SessionError> {
        let dir = self.session_dir(session_id);
        if !dir.exists() {
            return Ok(false);
        }
        tokio::fs::remove_dir_all(&dir)
            .await
            .map_err(SessionError::Io)?;
        Ok(true)
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<SessionMeta>, SessionError> {
        let all = self.list_sessions().await?;
        // list_sessions returns most-recent-first; return first match
        Ok(all.into_iter().find(|m| m.name.as_deref() == Some(name)))
    }
}

// ── Private helpers ───────────────────────────────────────────────────────────

impl FilesystemSessionAdapter {
    async fn read_meta(&self, id: Uuid) -> Result<SessionMeta, SessionError> {
        let path = self.meta_path(id);
        let bytes = tokio::fs::read(&path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                SessionError::NotFound { id }
            } else {
                SessionError::Io(e)
            }
        })?;
        serde_json::from_slice(&bytes).map_err(|e| SessionError::Corrupted {
            id,
            reason: format!("invalid meta.json: {e}"),
        })
    }

    async fn write_meta_atomic(&self, meta: &SessionMeta) -> Result<(), SessionError> {
        use tokio::io::AsyncWriteExt as _;

        let json = serde_json::to_vec_pretty(meta).map_err(SessionError::Serialization)?;
        let tmp = self.meta_path(meta.id).with_extension("json.tmp");
        {
            let mut f = tokio::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&tmp)
                .await
                .map_err(SessionError::Io)?;
            f.write_all(&json).await.map_err(SessionError::Io)?;
            f.flush().await.map_err(SessionError::Io)?;
        }
        tokio::fs::rename(&tmp, self.meta_path(meta.id))
            .await
            .map_err(SessionError::Io)?;
        Ok(())
    }

    async fn read_turns(&self, id: Uuid) -> Result<Vec<ConversationTurn>, SessionError> {
        let path = self.turns_path(id);

        // Turns file may not exist yet (zero-turn session)
        if !path.exists() {
            return Ok(Vec::new());
        }

        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(SessionError::Io)?;

        let mut turns = Vec::new();
        for (line_no, line) in content.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let turn: ConversationTurn =
                serde_json::from_str(line).map_err(|e| SessionError::Corrupted {
                    id,
                    reason: format!("invalid JSON on turns.jsonl line {}: {}", line_no + 1, e),
                })?;
            turns.push(turn);
        }
        Ok(turns)
    }
}
