//! Storage backend tests
//!
//! Tests for the StorageBackend trait implementations (SQLite and PostgreSQL).
//! PostgreSQL tests are feature-gated and require a running PostgreSQL server.

use avocado_core::storage::{create_backend, SqliteBackend, StorageBackend, StorageConfig};
use avocado_core::types::{Agent, Artifact, CompilerConfig, MessageRole, Span, WorkingSet};
use tempfile::TempDir;
use uuid::Uuid;

// ========== Helper Functions ==========

async fn create_test_sqlite_backend() -> (SqliteBackend, TempDir) {
    let tmp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = tmp_dir.path().join("test.db");
    let backend = SqliteBackend::new(&db_path)
        .await
        .expect("Failed to create SQLite backend");
    (backend, tmp_dir)
}

fn create_test_artifact(id: &str, path: &str, content: &str) -> Artifact {
    Artifact {
        id: id.to_string(),
        path: path.to_string(),
        content: content.to_string(),
        content_hash: format!("hash_{}", id),
        metadata: None,
        created_at: chrono::Utc::now(),
    }
}

fn create_test_span(artifact_id: &str, start: usize, end: usize, text: &str) -> Span {
    Span {
        id: Uuid::new_v4().to_string(),
        artifact_id: artifact_id.to_string(),
        start_line: start,
        end_line: end,
        text: text.to_string(),
        embedding: Some(vec![0.1; 384]), // Fake embedding
        embedding_model: Some("test-model".to_string()),
        token_count: text.split_whitespace().count(),
        metadata: None,
    }
}

// ========== SQLite Backend Tests ==========

#[tokio::test]
async fn test_sqlite_backend_creation() {
    let (backend, _tmp_dir) = create_test_sqlite_backend().await;

    let (artifacts, spans, tokens) = backend.get_stats().await.expect("Failed to get stats");
    assert_eq!(artifacts, 0);
    assert_eq!(spans, 0);
    assert_eq!(tokens, 0);
}

#[tokio::test]
async fn test_sqlite_artifact_operations() {
    let (backend, _tmp_dir) = create_test_sqlite_backend().await;

    let artifact = create_test_artifact("art-1", "test/path.rs", "fn main() {}");

    // Insert artifact
    backend
        .insert_artifact(&artifact)
        .await
        .expect("Failed to insert artifact");

    // Get artifact by ID
    let retrieved = backend
        .get_artifact("art-1")
        .await
        .expect("Failed to get artifact");
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.path, "test/path.rs");
    assert_eq!(retrieved.content, "fn main() {}");

    // Get artifact by path
    let by_path = backend
        .get_artifact_by_path("test/path.rs")
        .await
        .expect("Failed to get by path");
    assert!(by_path.is_some());
    assert_eq!(by_path.unwrap().id, "art-1");

    // Stats should reflect the artifact
    let (artifacts, _, _) = backend.get_stats().await.expect("Failed to get stats");
    assert_eq!(artifacts, 1);

    // Delete artifact
    let deleted_spans = backend
        .delete_artifact("art-1")
        .await
        .expect("Failed to delete");
    assert_eq!(deleted_spans, 0); // No spans were created

    // Verify deletion
    let should_be_none = backend
        .get_artifact("art-1")
        .await
        .expect("Failed to check deletion");
    assert!(should_be_none.is_none());
}

#[tokio::test]
async fn test_sqlite_span_operations() {
    let (backend, _tmp_dir) = create_test_sqlite_backend().await;

    // First create an artifact
    let artifact = create_test_artifact("art-2", "test/code.rs", "line1\nline2\nline3");
    backend
        .insert_artifact(&artifact)
        .await
        .expect("Failed to insert artifact");

    // Create and insert spans
    let spans = vec![
        create_test_span("art-2", 1, 2, "line1\nline2"),
        create_test_span("art-2", 3, 3, "line3"),
    ];
    backend
        .insert_spans(&spans)
        .await
        .expect("Failed to insert spans");

    // Get all spans
    let all_spans = backend
        .get_all_spans()
        .await
        .expect("Failed to get all spans");
    assert_eq!(all_spans.len(), 2);

    // Search spans by text
    let found = backend
        .search_spans("line2", 10)
        .await
        .expect("Failed to search spans");
    assert_eq!(found.len(), 1);

    // Verify stats
    let (_, span_count, _) = backend.get_stats().await.expect("Failed to get stats");
    assert_eq!(span_count, 2);
}

#[tokio::test]
async fn test_sqlite_session_operations() {
    let (backend, _tmp_dir) = create_test_sqlite_backend().await;

    // Create session
    let session = backend
        .create_session(Some("user-1"), Some("Test Session"))
        .await
        .expect("Failed to create session");

    assert!(!session.id.is_empty());
    assert_eq!(session.user_id, Some("user-1".to_string()));
    assert_eq!(session.title, Some("Test Session".to_string()));

    // Get session
    let retrieved = backend
        .get_session(&session.id)
        .await
        .expect("Failed to get session");
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.title, Some("Test Session".to_string()));

    // List sessions
    let sessions = backend
        .list_sessions(Some("user-1"), None)
        .await
        .expect("Failed to list sessions");
    assert_eq!(sessions.len(), 1);

    // Update session
    backend
        .update_session(&session.id, Some("Updated Title"), None)
        .await
        .expect("Failed to update session");

    let updated = backend
        .get_session(&session.id)
        .await
        .expect("Failed to get updated")
        .unwrap();
    assert_eq!(updated.title, Some("Updated Title".to_string()));

    // Delete session
    backend
        .delete_session(&session.id)
        .await
        .expect("Failed to delete session");

    let deleted = backend
        .get_session(&session.id)
        .await
        .expect("Failed to check deletion");
    assert!(deleted.is_none());
}

#[tokio::test]
async fn test_sqlite_message_operations() {
    let (backend, _tmp_dir) = create_test_sqlite_backend().await;

    // Create session first
    let session = backend
        .create_session(Some("user-2"), None)
        .await
        .expect("Failed to create session");

    // Add messages
    let msg1 = backend
        .add_message(&session.id, MessageRole::User, "Hello", None)
        .await
        .expect("Failed to add user message");
    assert_eq!(msg1.sequence_number, 0);
    assert_eq!(msg1.content, "Hello");

    let msg2 = backend
        .add_message(&session.id, MessageRole::Assistant, "Hi there!", None)
        .await
        .expect("Failed to add assistant message");
    assert_eq!(msg2.sequence_number, 1);

    // Get messages
    let messages = backend
        .get_messages(&session.id, None)
        .await
        .expect("Failed to get messages");
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].content, "Hello");
    assert_eq!(messages[1].content, "Hi there!");

    // Test limit
    let limited = backend
        .get_messages(&session.id, Some(1))
        .await
        .expect("Failed to get limited");
    assert_eq!(limited.len(), 1);
}

#[tokio::test]
async fn test_sqlite_working_set_operations() {
    let (backend, _tmp_dir) = create_test_sqlite_backend().await;

    // Create session
    let session = backend
        .create_session(None, None)
        .await
        .expect("Failed to create session");

    // Create a working set
    let working_set = WorkingSet {
        text: "Test context".to_string(),
        spans: vec![],
        citations: vec![],
        tokens_used: 100,
        query: "test query".to_string(),
        compilation_time_ms: 50,
        manifest: None,
        explain: None,
    };

    let config = CompilerConfig::default();

    // Associate working set with session
    let sws = backend
        .associate_working_set(&session.id, None, &working_set, "test query", &config)
        .await
        .expect("Failed to associate working set");

    assert_eq!(sws.session_id, session.id);
    assert_eq!(sws.query, "test query");

    // Get full session
    let full = backend
        .get_session_full(&session.id)
        .await
        .expect("Failed to get full session");
    assert!(full.is_some());
}

#[tokio::test]
async fn test_sqlite_agent_operations() {
    let (backend, _tmp_dir) = create_test_sqlite_backend().await;

    // Create and register agent
    let agent = Agent {
        id: Uuid::new_v4().to_string(),
        name: "test-agent".to_string(),
        role: "researcher".to_string(),
        model: "gpt-4".to_string(),
        system_prompt: Some("You are a helpful researcher.".to_string()),
        did: None,
        capabilities: Some(vec!["web_search".to_string()]),
        metadata: None,
        created_at: chrono::Utc::now(),
    };

    let registered = backend
        .register_agent(&agent)
        .await
        .expect("Failed to register agent");
    assert_eq!(registered.name, "test-agent");

    // Get agent by ID
    let retrieved = backend
        .get_agent(&agent.id)
        .await
        .expect("Failed to get agent");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().role, "researcher");

    // Get agent by name
    let by_name = backend
        .get_agent_by_name("test-agent")
        .await
        .expect("Failed to get by name");
    assert!(by_name.is_some());

    // List agents
    let agents = backend.list_agents().await.expect("Failed to list agents");
    assert_eq!(agents.len(), 1);
}

#[tokio::test]
async fn test_sqlite_determine_ingest_action() {
    let (backend, _tmp_dir) = create_test_sqlite_backend().await;

    // For a new document, should return Create
    let action = backend
        .determine_ingest_action("new/file.rs", "hash123")
        .await
        .expect("Failed to determine action");
    assert!(matches!(action, avocado_core::IngestAction::Create));

    // Insert an artifact
    let artifact = create_test_artifact("art-3", "existing/file.rs", "content");
    backend
        .insert_artifact(&artifact)
        .await
        .expect("Failed to insert");

    // Same hash should Skip
    let action = backend
        .determine_ingest_action("existing/file.rs", "hash_art-3")
        .await
        .expect("Failed to determine action");
    assert!(matches!(action, avocado_core::IngestAction::Skip { .. }));

    // Different hash should Update
    let action = backend
        .determine_ingest_action("existing/file.rs", "different_hash")
        .await
        .expect("Failed to determine action");
    assert!(matches!(action, avocado_core::IngestAction::Update { .. }));
}

#[tokio::test]
async fn test_sqlite_clear() {
    let (backend, _tmp_dir) = create_test_sqlite_backend().await;

    // Add some data
    let artifact = create_test_artifact("art-4", "test.rs", "content");
    backend
        .insert_artifact(&artifact)
        .await
        .expect("Failed to insert artifact");

    let spans = vec![create_test_span("art-4", 1, 1, "content")];
    backend
        .insert_spans(&spans)
        .await
        .expect("Failed to insert spans");

    // Verify data exists
    let (artifacts, span_count, _) = backend.get_stats().await.expect("Failed to get stats");
    assert!(artifacts > 0);
    assert!(span_count > 0);

    // Clear all data
    backend.clear().await.expect("Failed to clear");

    // Verify everything is cleared
    let (artifacts, span_count, _) = backend
        .get_stats()
        .await
        .expect("Failed to get stats after clear");
    assert_eq!(artifacts, 0);
    assert_eq!(span_count, 0);
}

// ========== Storage Config Tests ==========

#[tokio::test]
async fn test_storage_config_from_env_default() {
    // When AVOCADO_BACKEND is not set, should default to SQLite
    std::env::remove_var("AVOCADO_BACKEND");
    let config = StorageConfig::from_env("/default/path.db");
    assert!(matches!(config, StorageConfig::Sqlite { path } if path == "/default/path.db"));
}

#[tokio::test]
async fn test_storage_config_from_env_sqlite_explicit() {
    std::env::set_var("AVOCADO_BACKEND", "sqlite");
    let config = StorageConfig::from_env("/default/path.db");
    assert!(matches!(config, StorageConfig::Sqlite { path } if path == "/default/path.db"));
    std::env::remove_var("AVOCADO_BACKEND");
}

#[tokio::test]
async fn test_storage_config_from_env_sqlite_with_path() {
    std::env::set_var("AVOCADO_BACKEND", "sqlite:/custom/path.db");
    let config = StorageConfig::from_env("/default/path.db");
    assert!(matches!(config, StorageConfig::Sqlite { path } if path == "/custom/path.db"));
    std::env::remove_var("AVOCADO_BACKEND");
}

// PostgreSQL client-library was removed in favor of avocado-pgext extension
// These tests verify that postgres:// URLs are still parsed but create_backend returns an error

#[tokio::test]
async fn test_storage_config_from_env_postgres_returns_error() {
    std::env::set_var("AVOCADO_BACKEND", "postgres://user:pass@localhost/db");
    let config = StorageConfig::from_env("/default/path.db");
    // Config is still parsed as Postgres
    assert!(matches!(config, StorageConfig::Postgres { .. }));
    // But creating a backend fails with helpful message
    let result = create_backend(config).await;
    match result {
        Ok(_) => panic!("Expected error for postgres config"),
        Err(e) => assert!(e.to_string().contains("avocado-pgext")),
    }
    std::env::remove_var("AVOCADO_BACKEND");
}

#[tokio::test]
async fn test_storage_config_from_env_postgresql_returns_error() {
    std::env::set_var("AVOCADO_BACKEND", "postgresql://user:pass@localhost/db");
    let config = StorageConfig::from_env("/default/path.db");
    // Config is still parsed as Postgres
    assert!(matches!(config, StorageConfig::Postgres { .. }));
    // But creating a backend fails with helpful message
    let result = create_backend(config).await;
    match result {
        Ok(_) => panic!("Expected error for postgresql config"),
        Err(e) => assert!(e.to_string().contains("avocado-pgext")),
    }
    std::env::remove_var("AVOCADO_BACKEND");
}

#[tokio::test]
async fn test_storage_config_from_env_unknown_defaults_to_sqlite() {
    std::env::set_var("AVOCADO_BACKEND", "unknown_backend");
    let config = StorageConfig::from_env("/default/path.db");
    // Should default to SQLite with warning
    assert!(matches!(config, StorageConfig::Sqlite { path } if path == "/default/path.db"));
    std::env::remove_var("AVOCADO_BACKEND");
}

// ========== Factory Function Tests ==========

#[tokio::test]
async fn test_create_backend_sqlite() {
    let tmp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = tmp_dir.path().join("factory_test.db");

    let config = StorageConfig::Sqlite {
        path: db_path.to_string_lossy().to_string(),
    };
    let backend = create_backend(config)
        .await
        .expect("Failed to create backend");

    // Verify it works
    let (artifacts, _, _) = backend.get_stats().await.expect("Failed to get stats");
    assert_eq!(artifacts, 0);
}

// PostgreSQL Backend Tests removed - postgres.rs was deleted in favor of avocado-pgext
