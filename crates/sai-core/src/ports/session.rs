//! Session persistence port trait.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::session::{ConversationTurn, PersistedSession, SessionMeta};
use crate::error::SessionError;

/// Port trait for session persistence.
///
/// Implementations persist conversation history to durable storage and allow
/// sessions to be listed, loaded, and deleted. The agent loop calls
/// `create_session` once per session and `save_turn` after each completed turn.
#[async_trait]
#[cfg_attr(test, mockall::automock)]
pub trait SessionPort: Send + Sync {
    /// Register a new session.
    ///
    /// Idempotent: calling with the same `meta.id` a second time MUST succeed
    /// and be a no-op. Returns `SessionError::NameConflict` if `meta.name` is
    /// already in use by a different session.
    async fn create_session(&self, meta: SessionMeta) -> Result<(), SessionError>;

    /// Persist a completed conversation turn.
    ///
    /// Returns `SessionError::NotFound` if `create_session` was not called
    /// first for `session_id`.
    async fn save_turn(&self, session_id: Uuid, turn: ConversationTurn)
        -> Result<(), SessionError>;

    /// Load a full session, including all persisted turns.
    ///
    /// Returns `Ok(None)` if no session with the given ID exists.
    /// Returns `Err(SessionError::Corrupted)` if data is unreadable.
    async fn load_session(
        &self,
        session_id: Uuid,
    ) -> Result<Option<PersistedSession>, SessionError>;

    /// List all saved sessions, ordered by `last_active_at` descending.
    async fn list_sessions(&self) -> Result<Vec<SessionMeta>, SessionError>;

    /// Delete a session and all its data.
    ///
    /// Returns `true` if the session existed and was deleted, `false` if not found.
    async fn delete_session(&self, session_id: Uuid) -> Result<bool, SessionError>;

    /// Find a session by its human-readable name.
    ///
    /// Returns `Ok(None)` if no session with this name exists.
    async fn find_by_name(&self, name: &str) -> Result<Option<SessionMeta>, SessionError>;
}

// ── No-op implementation ───────────────────────────────────────────────────────

/// A `SessionPort` implementation that does nothing.
///
/// Used in non-interactive mode and in tests that do not require persistence.
/// All methods succeed silently and return empty/None results.
pub struct NoOpSessionPort;

#[async_trait]
impl SessionPort for NoOpSessionPort {
    async fn create_session(&self, _meta: SessionMeta) -> Result<(), SessionError> {
        Ok(())
    }

    async fn save_turn(
        &self,
        _session_id: Uuid,
        _turn: ConversationTurn,
    ) -> Result<(), SessionError> {
        Ok(())
    }

    async fn load_session(
        &self,
        _session_id: Uuid,
    ) -> Result<Option<PersistedSession>, SessionError> {
        Ok(None)
    }

    async fn list_sessions(&self) -> Result<Vec<SessionMeta>, SessionError> {
        Ok(Vec::new())
    }

    async fn delete_session(&self, _session_id: Uuid) -> Result<bool, SessionError> {
        Ok(false)
    }

    async fn find_by_name(&self, _name: &str) -> Result<Option<SessionMeta>, SessionError> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::session::SessionMeta;
    use chrono::Utc;
    use std::path::PathBuf;
    use uuid::Uuid;

    fn make_meta() -> SessionMeta {
        SessionMeta::new(
            Uuid::new_v4(),
            "test-model".to_string(),
            PathBuf::from("/tmp"),
        )
    }

    #[tokio::test]
    async fn noop_create_session_succeeds() {
        let port = NoOpSessionPort;
        let meta = make_meta();
        assert!(port.create_session(meta).await.is_ok());
    }

    #[tokio::test]
    async fn noop_save_turn_succeeds() {
        let port = NoOpSessionPort;
        let turn = ConversationTurn {
            turn_index: 0,
            user_message: "hello".to_string(),
            messages: vec![],
            completed_at: Utc::now(),
        };
        assert!(port.save_turn(Uuid::new_v4(), turn).await.is_ok());
    }

    #[tokio::test]
    async fn noop_load_session_returns_none() {
        let port = NoOpSessionPort;
        let result = port.load_session(Uuid::new_v4()).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn noop_list_sessions_returns_empty() {
        let port = NoOpSessionPort;
        let result = port.list_sessions().await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn noop_delete_session_returns_false() {
        let port = NoOpSessionPort;
        let result = port.delete_session(Uuid::new_v4()).await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn noop_find_by_name_returns_none() {
        let port = NoOpSessionPort;
        let result = port.find_by_name("anything").await.unwrap();
        assert!(result.is_none());
    }
}
