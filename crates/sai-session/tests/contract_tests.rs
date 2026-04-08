//! Contract tests for `FilesystemSessionAdapter`.
//!
//! These tests verify that the adapter satisfies the `SessionPort` behavioral
//! contract defined in `contracts/session-port-contract.md`.
//!
//! Tests are organized by user story to match tasks.md.

use std::path::PathBuf;

use chrono::Utc;
use sai_core::domain::message::Message;
use sai_core::domain::session::{ConversationTurn, PersistedSession, SessionMeta};
use sai_core::error::SessionError;
use sai_core::ports::session::SessionPort;
use sai_session::FilesystemSessionAdapter;
use uuid::Uuid;

fn make_adapter(dir: &tempfile::TempDir) -> FilesystemSessionAdapter {
    FilesystemSessionAdapter::with_base_dir(dir.path().to_path_buf())
}

fn make_meta(id: Uuid) -> SessionMeta {
    SessionMeta::new(id, "test-model".to_string(), PathBuf::from("/tmp/project"))
}

fn make_turn(index: usize) -> ConversationTurn {
    ConversationTurn {
        turn_index: index,
        user_message: format!("message {index}"),
        messages: vec![
            Message::user(format!("message {index}")),
            Message::assistant_text(format!("response {index}")),
        ],
        completed_at: Utc::now(),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// US3: Auto-Save — create_session contract tests
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn us3_create_session_creates_directory() {
    let dir = tempfile::TempDir::new().unwrap();
    let adapter = make_adapter(&dir);
    let id = Uuid::new_v4();
    let meta = make_meta(id);

    adapter.create_session(meta).await.unwrap();

    assert!(dir.path().join(id.to_string()).is_dir());
}

#[tokio::test]
async fn us3_create_session_creates_meta_json() {
    let dir = tempfile::TempDir::new().unwrap();
    let adapter = make_adapter(&dir);
    let id = Uuid::new_v4();
    let meta = make_meta(id);

    adapter.create_session(meta).await.unwrap();

    assert!(dir.path().join(id.to_string()).join("meta.json").is_file());
}

#[tokio::test]
async fn us3_create_session_is_idempotent() {
    let dir = tempfile::TempDir::new().unwrap();
    let adapter = make_adapter(&dir);
    let id = Uuid::new_v4();

    adapter.create_session(make_meta(id)).await.unwrap();
    // Second call with same ID must succeed silently
    adapter.create_session(make_meta(id)).await.unwrap();
}

#[tokio::test]
async fn us3_create_session_name_conflict_returns_error() {
    let dir = tempfile::TempDir::new().unwrap();
    let adapter = make_adapter(&dir);

    let mut meta1 = make_meta(Uuid::new_v4());
    meta1.name = Some("my-session".to_string());
    adapter.create_session(meta1).await.unwrap();

    let mut meta2 = make_meta(Uuid::new_v4());
    meta2.name = Some("my-session".to_string());
    let result = adapter.create_session(meta2).await;

    assert!(
        matches!(result, Err(SessionError::NameConflict { .. })),
        "Expected NameConflict, got: {result:?}"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn us3_create_session_directory_has_mode_0700() {
    use std::os::unix::fs::PermissionsExt as _;

    let dir = tempfile::TempDir::new().unwrap();
    let adapter = make_adapter(&dir);
    let id = Uuid::new_v4();

    adapter.create_session(make_meta(id)).await.unwrap();

    let session_dir = dir.path().join(id.to_string());
    let perms = std::fs::metadata(&session_dir).unwrap().permissions();
    assert_eq!(
        perms.mode() & 0o777,
        0o700,
        "Session directory should have mode 0700"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn us3_create_session_meta_file_has_mode_0600() {
    use std::os::unix::fs::PermissionsExt as _;

    let dir = tempfile::TempDir::new().unwrap();
    let adapter = make_adapter(&dir);
    let id = Uuid::new_v4();

    adapter.create_session(make_meta(id)).await.unwrap();

    let meta_path = dir.path().join(id.to_string()).join("meta.json");
    let perms = std::fs::metadata(&meta_path).unwrap().permissions();
    assert_eq!(
        perms.mode() & 0o777,
        0o600,
        "meta.json should have mode 0600"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// US3: Auto-Save — save_turn contract tests
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn us3_save_turn_appends_to_turns_jsonl() {
    let dir = tempfile::TempDir::new().unwrap();
    let adapter = make_adapter(&dir);
    let id = Uuid::new_v4();

    adapter.create_session(make_meta(id)).await.unwrap();
    adapter.save_turn(id, make_turn(0)).await.unwrap();
    adapter.save_turn(id, make_turn(1)).await.unwrap();

    let turns_path = dir.path().join(id.to_string()).join("turns.jsonl");
    let content = std::fs::read_to_string(&turns_path).unwrap();
    assert_eq!(content.lines().count(), 2, "Should have 2 turn lines");
}

#[tokio::test]
async fn us3_save_turn_updates_meta_turn_count() {
    let dir = tempfile::TempDir::new().unwrap();
    let adapter = make_adapter(&dir);
    let id = Uuid::new_v4();

    adapter.create_session(make_meta(id)).await.unwrap();
    adapter.save_turn(id, make_turn(0)).await.unwrap();
    adapter.save_turn(id, make_turn(1)).await.unwrap();

    let loaded = adapter.load_session(id).await.unwrap().unwrap();
    assert_eq!(loaded.meta.turn_count, 2);
}

#[tokio::test]
async fn us3_save_turn_not_found_for_unknown_session() {
    let dir = tempfile::TempDir::new().unwrap();
    let adapter = make_adapter(&dir);

    let result = adapter.save_turn(Uuid::new_v4(), make_turn(0)).await;
    assert!(
        matches!(result, Err(SessionError::NotFound { .. })),
        "Expected NotFound, got: {result:?}"
    );
}

#[tokio::test]
async fn us3_save_turn_meta_updated_atomically() {
    let dir = tempfile::TempDir::new().unwrap();
    let adapter = make_adapter(&dir);
    let id = Uuid::new_v4();

    adapter.create_session(make_meta(id)).await.unwrap();

    // Verify that no .tmp file is left after save_turn
    adapter.save_turn(id, make_turn(0)).await.unwrap();

    let session_dir = dir.path().join(id.to_string());
    let tmp_exists = session_dir.join("meta.json.tmp").exists();
    assert!(!tmp_exists, "No .tmp file should remain after atomic write");
}

#[cfg(unix)]
#[tokio::test]
async fn us3_save_turn_turns_file_has_mode_0600() {
    use std::os::unix::fs::PermissionsExt as _;

    let dir = tempfile::TempDir::new().unwrap();
    let adapter = make_adapter(&dir);
    let id = Uuid::new_v4();

    adapter.create_session(make_meta(id)).await.unwrap();
    adapter.save_turn(id, make_turn(0)).await.unwrap();

    let turns_path = dir.path().join(id.to_string()).join("turns.jsonl");
    let perms = std::fs::metadata(&turns_path).unwrap().permissions();
    assert_eq!(
        perms.mode() & 0o777,
        0o600,
        "turns.jsonl should have mode 0600"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// US1: Resume — load_session contract tests
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn us1_load_session_returns_none_for_unknown_id() {
    let dir = tempfile::TempDir::new().unwrap();
    let adapter = make_adapter(&dir);

    let result = adapter.load_session(Uuid::new_v4()).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn us1_load_session_round_trips_turns() {
    let dir = tempfile::TempDir::new().unwrap();
    let adapter = make_adapter(&dir);
    let id = Uuid::new_v4();

    adapter.create_session(make_meta(id)).await.unwrap();
    adapter.save_turn(id, make_turn(0)).await.unwrap();
    adapter.save_turn(id, make_turn(1)).await.unwrap();

    let loaded = adapter.load_session(id).await.unwrap().unwrap();
    assert_eq!(loaded.turns.len(), 2);
    assert_eq!(loaded.turns[0].user_message, "message 0");
    assert_eq!(loaded.turns[1].user_message, "message 1");
}

#[tokio::test]
async fn us1_load_session_turns_in_order() {
    let dir = tempfile::TempDir::new().unwrap();
    let adapter = make_adapter(&dir);
    let id = Uuid::new_v4();

    adapter.create_session(make_meta(id)).await.unwrap();
    for i in 0..5 {
        adapter.save_turn(id, make_turn(i)).await.unwrap();
    }

    let loaded = adapter.load_session(id).await.unwrap().unwrap();
    for (i, turn) in loaded.turns.iter().enumerate() {
        assert_eq!(turn.turn_index, i);
    }
}

#[tokio::test]
async fn us1_load_session_cold_reload() {
    let dir = tempfile::TempDir::new().unwrap();
    let id = Uuid::new_v4();

    // First adapter instance: create and save
    {
        let adapter = make_adapter(&dir);
        adapter.create_session(make_meta(id)).await.unwrap();
        adapter.save_turn(id, make_turn(0)).await.unwrap();
    }

    // Second adapter instance: reload from disk
    {
        let adapter = make_adapter(&dir);
        let loaded = adapter.load_session(id).await.unwrap().unwrap();
        assert_eq!(loaded.turns.len(), 1);
        assert_eq!(loaded.turns[0].user_message, "message 0");
    }
}

#[tokio::test]
async fn us1_load_session_corrupted_data_returns_error() {
    let dir = tempfile::TempDir::new().unwrap();
    let id = Uuid::new_v4();

    // Create session dir and write bad JSON to meta.json
    let session_dir = dir.path().join(id.to_string());
    std::fs::create_dir_all(&session_dir).unwrap();
    std::fs::write(session_dir.join("meta.json"), b"{ not valid json ").unwrap();

    let adapter = make_adapter(&dir);
    let result = adapter.load_session(id).await;
    assert!(
        matches!(result, Err(SessionError::Corrupted { .. })),
        "Expected Corrupted, got: {result:?}"
    );
}

#[tokio::test]
async fn us1_load_session_corrupted_turns_returns_error() {
    let dir = tempfile::TempDir::new().unwrap();
    let adapter = make_adapter(&dir);
    let id = Uuid::new_v4();

    adapter.create_session(make_meta(id)).await.unwrap();

    // Write bad JSON into turns.jsonl
    let turns_path = dir.path().join(id.to_string()).join("turns.jsonl");
    std::fs::write(&turns_path, b"{ invalid\n").unwrap();

    let result = adapter.load_session(id).await;
    assert!(
        matches!(result, Err(SessionError::Corrupted { .. })),
        "Expected Corrupted, got: {result:?}"
    );
}

#[tokio::test]
async fn us1_persisted_session_into_messages_flattens_turns() {
    let dir = tempfile::TempDir::new().unwrap();
    let adapter = make_adapter(&dir);
    let id = Uuid::new_v4();

    adapter.create_session(make_meta(id)).await.unwrap();
    adapter.save_turn(id, make_turn(0)).await.unwrap(); // 2 messages
    adapter.save_turn(id, make_turn(1)).await.unwrap(); // 2 messages

    let loaded = adapter.load_session(id).await.unwrap().unwrap();
    let messages = loaded.into_messages();
    assert_eq!(messages.len(), 4, "2 turns × 2 messages each = 4 total");
}

// ═══════════════════════════════════════════════════════════════════════════
// US1: Resume — find_by_name contract tests
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn us1_find_by_name_returns_none_for_unknown_name() {
    let dir = tempfile::TempDir::new().unwrap();
    let adapter = make_adapter(&dir);

    let result = adapter.find_by_name("nonexistent").await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn us1_find_by_name_returns_matching_session() {
    let dir = tempfile::TempDir::new().unwrap();
    let adapter = make_adapter(&dir);

    let id = Uuid::new_v4();
    let mut meta = make_meta(id);
    meta.name = Some("my-task".to_string());
    adapter.create_session(meta).await.unwrap();

    let found = adapter.find_by_name("my-task").await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, id);
}

#[tokio::test]
async fn us1_find_by_name_returns_none_for_unnamed_session() {
    let dir = tempfile::TempDir::new().unwrap();
    let adapter = make_adapter(&dir);

    adapter
        .create_session(make_meta(Uuid::new_v4()))
        .await
        .unwrap();

    let result = adapter.find_by_name("anything").await.unwrap();
    assert!(result.is_none());
}

// ═══════════════════════════════════════════════════════════════════════════
// US2: List Sessions — list_sessions contract tests
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn us2_list_sessions_returns_empty_when_none_exist() {
    let dir = tempfile::TempDir::new().unwrap();
    let adapter = make_adapter(&dir);

    let list = adapter.list_sessions().await.unwrap();
    assert!(list.is_empty());
}

#[tokio::test]
async fn us2_list_sessions_returns_all_sessions() {
    let dir = tempfile::TempDir::new().unwrap();
    let adapter = make_adapter(&dir);

    for _ in 0..3 {
        adapter
            .create_session(make_meta(Uuid::new_v4()))
            .await
            .unwrap();
    }

    let list = adapter.list_sessions().await.unwrap();
    assert_eq!(list.len(), 3);
}

#[tokio::test]
async fn us2_list_sessions_ordered_most_recent_first() {
    let dir = tempfile::TempDir::new().unwrap();
    let adapter = make_adapter(&dir);

    // Create 3 sessions and add turns to push last_active_at forward
    let ids: Vec<Uuid> = (0..3).map(|_| Uuid::new_v4()).collect();
    for &id in &ids {
        adapter.create_session(make_meta(id)).await.unwrap();
        // Small sleep to ensure distinct timestamps
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        adapter.save_turn(id, make_turn(0)).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }

    let list = adapter.list_sessions().await.unwrap();
    assert_eq!(list.len(), 3);

    // Verify descending order
    for window in list.windows(2) {
        assert!(
            window[0].last_active_at >= window[1].last_active_at,
            "Sessions should be ordered most-recently-active first"
        );
    }
}

#[tokio::test]
async fn us2_list_sessions_skips_corrupted_sessions() {
    let dir = tempfile::TempDir::new().unwrap();
    let adapter = make_adapter(&dir);

    // Create one valid session
    let valid_id = Uuid::new_v4();
    adapter.create_session(make_meta(valid_id)).await.unwrap();

    // Create a corrupted session directory (valid UUID dir, invalid meta.json)
    let corrupt_id = Uuid::new_v4();
    let corrupt_dir = dir.path().join(corrupt_id.to_string());
    std::fs::create_dir_all(&corrupt_dir).unwrap();
    std::fs::write(corrupt_dir.join("meta.json"), b"not json").unwrap();

    let list = adapter.list_sessions().await.unwrap();
    assert_eq!(list.len(), 1, "Corrupted session should be skipped");
    assert_eq!(list[0].id, valid_id);
}

// ═══════════════════════════════════════════════════════════════════════════
// US5: Delete Sessions — delete_session contract tests
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn us5_delete_session_returns_true_and_removes_directory() {
    let dir = tempfile::TempDir::new().unwrap();
    let adapter = make_adapter(&dir);
    let id = Uuid::new_v4();

    adapter.create_session(make_meta(id)).await.unwrap();
    let result = adapter.delete_session(id).await.unwrap();

    assert!(result, "Should return true for existing session");
    assert!(
        !dir.path().join(id.to_string()).exists(),
        "Directory should be removed"
    );
}

#[tokio::test]
async fn us5_delete_session_returns_false_for_unknown_id() {
    let dir = tempfile::TempDir::new().unwrap();
    let adapter = make_adapter(&dir);

    let result = adapter.delete_session(Uuid::new_v4()).await.unwrap();
    assert!(!result, "Should return false for unknown session");
}

#[tokio::test]
async fn us5_load_session_returns_none_after_delete() {
    let dir = tempfile::TempDir::new().unwrap();
    let adapter = make_adapter(&dir);
    let id = Uuid::new_v4();

    adapter.create_session(make_meta(id)).await.unwrap();
    adapter.delete_session(id).await.unwrap();

    let result = adapter.load_session(id).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn us5_delete_session_not_in_list_after_delete() {
    let dir = tempfile::TempDir::new().unwrap();
    let adapter = make_adapter(&dir);

    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();
    adapter.create_session(make_meta(id1)).await.unwrap();
    adapter.create_session(make_meta(id2)).await.unwrap();

    adapter.delete_session(id1).await.unwrap();

    let list = adapter.list_sessions().await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, id2);
}
