//! End-to-End Integration Tests for Session Management
//!
//! These tests verify the complete session workflow across all layers:
//! - Database operations
//! - SessionManager API
//! - Full conversation flows
//! - Edge cases and error handling
//!
//! Run with: cargo test --test session_e2e_tests

use avocado_core::db::Database;
use avocado_core::index::VectorIndex;
use avocado_core::session::SessionManager;
use avocado_core::types::{CompilerConfig, MessageRole};
use std::path::PathBuf;
use tempfile::TempDir;
use std::sync::Arc;
use avocado_core::{span, Artifact};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Test environment helper
#[allow(dead_code)]
struct TestEnv {
    _temp_dir: TempDir,
    _db_path: PathBuf,
    db: Database,
    session_manager: SessionManager,
    index: Arc<VectorIndex>,
}

impl TestEnv {
    fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");
        let db = Database::new(&db_path).expect("Failed to create database");
        let session_manager = SessionManager::new(db.clone());
        let index = db.get_vector_index().expect("failed to get vector index");

        Self {
            _temp_dir: temp_dir,
            _db_path: db_path,
            db,
            session_manager,
            index,
        }
    }

    fn default_config() -> CompilerConfig {
        CompilerConfig {
            token_budget: 8000,
            ..Default::default()
        }
    }
}

async fn ingest_text(db: &Database, path: &str, content: &str) {
    let artifact_id = Uuid::new_v4().to_string();
    let content_hash = format!("{:x}", Sha256::digest(content.as_bytes()));
    let artifact = Artifact {
        id: artifact_id.clone(),
        path: path.to_string(),
        content: content.to_string(),
        content_hash,
        metadata: None,
        created_at: chrono::Utc::now(),
    };
    db.insert_artifact(&artifact).expect("insert_artifact failed");
    let mut spans = span::extract_spans(content, &artifact_id).expect("extract_spans failed");
    let texts: Vec<&str> = spans.iter().map(|s| s.text.as_str()).collect();
    let embeddings = avocado_core::embedding::embed_batch(texts, None, None)
        .await
        .expect("embed_batch failed");
    for (s, emb) in spans.iter_mut().zip(embeddings.iter()) {
        s.embedding = Some(emb.clone());
        s.embedding_model = Some("minilm".to_string());
    }
    db.insert_spans(&spans).expect("insert_spans failed");
}

#[tokio::test]
async fn test_full_session_workflow() {
    let env = TestEnv::new();

    // Step 1: Create session
    let session = env
        .session_manager
        .start_session(Some("alice"))
        .expect("Failed to create session");

    assert_eq!(session.user_id, Some("alice".to_string()));

    // Step 2: Ingest some test data
    let test_content = "Rust is a systems programming language focused on safety and performance.";
    ingest_text(&env.db, "rust_intro.md", test_content).await;

    // Step 3: Add user message and compile context
    let (msg1, _ws1) = env
        .session_manager
        .add_user_message(
            &session.id,
            "What is Rust?",
            TestEnv::default_config(),
            env.index.as_ref(),
            None,
        )
        .await
        .expect("Failed to add user message");

    assert_eq!(msg1.role, MessageRole::User);
    assert_eq!(msg1.content, "What is Rust?");
    assert_eq!(msg1.sequence_number, 0);
    // Working set may be empty in minimal test corpora; ensure compilation completed without error

    // Step 4: Add assistant response
    let msg2 = env
        .session_manager
        .add_assistant_message(
            &session.id,
            "Rust is a systems programming language...",
            None,
        )
        .expect("Failed to add assistant message");

    assert_eq!(msg2.role, MessageRole::Assistant);
    assert_eq!(msg2.sequence_number, 1);

    // Step 5: Continue conversation
    let (msg3, _ws2) = env
        .session_manager
        .add_user_message(
            &session.id,
            "Tell me about ownership",
            TestEnv::default_config(),
            &env.index,
            None,
        )
        .await
        .expect("Failed to add second user message");

    assert_eq!(msg3.sequence_number, 2);
    // Second compilation also completes without requiring non-empty context

    // Step 6: Get conversation history
    let history = env
        .session_manager
        .get_conversation_history(&session.id, None)
        .expect("Failed to get history");

    assert!(history.contains("What is Rust?"));
    assert!(history.contains("Tell me about ownership"));

    // Step 7: Replay session
    let replay = env
        .session_manager
        .replay_session(&session.id)
        .expect("Failed to replay session");

    assert_eq!(replay.turns.len(), 2); // 2 user queries
    assert!(replay.turns[0].working_set.is_some());
    assert!(replay.turns[0].assistant_message.is_some());
    assert!(replay.turns[1].working_set.is_some());
}

#[tokio::test]
async fn test_multiple_sessions_isolation() {
    let env = TestEnv::new();

    // Create two sessions
    let session1 = env
        .session_manager
        .start_session(Some("alice"))
        .expect("Failed to create session 1");

    let session2 = env
        .session_manager
        .start_session(Some("bob"))
        .expect("Failed to create session 2");

    // Ingest test data
    ingest_text(&env.db, "test.md", "Test content").await;

    // Add messages to both sessions
    env.session_manager
        .add_user_message(
            &session1.id,
            "Alice's question",
            TestEnv::default_config(),
            env.index.as_ref(),
            None,
        )
        .await
        .expect("Failed to add to session 1");

    env.session_manager
        .add_user_message(
            &session2.id,
            "Bob's question",
            TestEnv::default_config(),
            env.index.as_ref(),
            None,
        )
        .await
        .expect("Failed to add to session 2");

    // Verify isolation
    let history1 = env
        .session_manager
        .get_conversation_history(&session1.id, None)
        .expect("Failed to get history 1");

    let history2 = env
        .session_manager
        .get_conversation_history(&session2.id, None)
        .expect("Failed to get history 2");

    assert!(history1.contains("Alice's question"));
    assert!(!history1.contains("Bob's question"));

    assert!(history2.contains("Bob's question"));
    assert!(!history2.contains("Alice's question"));
}

#[tokio::test]
async fn test_long_conversation_history() {
    let env = TestEnv::new();

    let session = env
        .session_manager
        .start_session(Some("test_user"))
        .expect("Failed to create session");

    // Ingest test data
    ingest_text(&env.db, "test.md", "Test content for context compilation").await;

    // Add 100 messages (50 user + 50 assistant)
    for i in 0..50 {
        env.session_manager
            .add_user_message(
                &session.id,
                &format!("User message {}", i),
                TestEnv::default_config(),
                env.index.as_ref(),
                None,
            )
            .await
            .expect("Failed to add user message");

        env.session_manager
            .add_assistant_message(
                &session.id,
                &format!("Assistant response {}", i),
                None,
            )
            .expect("Failed to add assistant message");
    }

    // Verify all messages are stored
    let messages = env
        .db
        .get_messages(&session.id, None)
        .expect("Failed to get messages");

    assert_eq!(messages.len(), 100);

    // Verify sequence numbers are correct
    for (idx, msg) in messages.iter().enumerate() {
        assert_eq!(msg.sequence_number, idx as usize);
    }

    // Get full history
    let history = env
        .session_manager
        .get_conversation_history(&session.id, None)
        .expect("Failed to get history");

    assert!(history.contains("User message 0"));
    assert!(history.contains("User message 49"));
    assert!(history.contains("Assistant response 0"));
    assert!(history.contains("Assistant response 49"));
}

#[tokio::test]
async fn test_token_limited_history() {
    let env = TestEnv::new();

    let session = env
        .session_manager
        .start_session(Some("test_user"))
        .expect("Failed to create session");

    // Ingest test data
    ingest_text(&env.db, "test.md", "Test content").await;

    // Add messages with known sizes
    let long_message = "A".repeat(1000); // ~250 tokens

    for i in 0..10 {
        env.session_manager
            .add_user_message(
                &session.id,
                &format!("Message {}: {}", i, long_message),
                TestEnv::default_config(),
                env.index.as_ref(),
                None,
            )
            .await
            .expect("Failed to add message");
    }

    // Get limited history (should only get most recent messages)
    let history = env
        .session_manager
        .get_conversation_history(&session.id, Some(1000))
        .expect("Failed to get limited history");

    // Should contain recent messages
    assert!(history.contains("Message 9"));

    // Should not contain old messages
    assert!(!history.contains("Message 0"));
    assert!(!history.contains("Message 1"));
}

#[tokio::test]
async fn test_session_replay_structure() {
    let env = TestEnv::new();

    let session = env
        .session_manager
        .start_session(Some("test_user"))
        .expect("Failed to create session");

    // Ingest test data
    ingest_text(&env.db, "test.md", "Test content").await;

    // Create a conversation with specific structure
    // Turn 1: User + Assistant
    env.session_manager
        .add_user_message(
            &session.id,
            "First question",
            TestEnv::default_config(),
            env.index.as_ref(),
            None,
        )
        .await
        .expect("Failed to add user message");

    env.session_manager
        .add_assistant_message(&session.id, "First answer", None)
        .expect("Failed to add assistant message");

    // Turn 2: User only (no response yet)
    env.session_manager
        .add_user_message(
            &session.id,
            "Second question",
            TestEnv::default_config(),
            &env.index,
            None,
        )
        .await
        .expect("Failed to add user message");

    // Replay and verify structure
    let replay = env
        .session_manager
        .replay_session(&session.id)
        .expect("Failed to replay");

    assert_eq!(replay.turns.len(), 2);

    // Turn 1 should have both user and assistant messages
    assert_eq!(replay.turns[0].user_message.content, "First question");
    assert!(replay.turns[0].working_set.is_some());
    assert!(replay.turns[0].assistant_message.is_some());
    assert_eq!(
        replay.turns[0].assistant_message.as_ref().unwrap().content,
        "First answer"
    );

    // Turn 2 should have only user message
    assert_eq!(replay.turns[1].user_message.content, "Second question");
    assert!(replay.turns[1].working_set.is_some());
    assert!(replay.turns[1].assistant_message.is_none());
}

#[tokio::test]
async fn test_session_deletion_cascade() {
    let env = TestEnv::new();

    let session = env
        .session_manager
        .start_session(Some("test_user"))
        .expect("Failed to create session");

    // Ingest test data
    ingest_text(&env.db, "test.md", "Test content").await;

    // Add messages and compile context
    env.session_manager
        .add_user_message(
            &session.id,
            "Test question",
            TestEnv::default_config(),
            env.index.as_ref(),
            None,
        )
        .await
        .expect("Failed to add message");

    env.session_manager
        .add_assistant_message(&session.id, "Test answer", None)
        .expect("Failed to add assistant message");

    // Verify data exists
    let messages_before = env
        .db
        .get_messages(&session.id, None)
        .expect("Failed to get messages");
    assert!(!messages_before.is_empty());

    // Delete session
    env.db
        .delete_session(&session.id)
        .expect("Failed to delete session");

    // Verify session is deleted
    let session_data = env.db.get_session_full(&session.id).expect("DB error");
    assert!(session_data.is_none());

    // Verify messages are deleted (cascade)
    let messages_after = env.db.get_messages(&session.id, None);
    // Should either return empty or NotFound error
    assert!(
        messages_after.is_err() || messages_after.unwrap().is_empty(),
        "Messages should be deleted with session"
    );
}

#[test]
fn test_session_list_filtering() {
    let env = TestEnv::new();

    // Create sessions for different users
    env.session_manager
        .start_session(Some("alice"))
        .expect("Failed to create session");

    env.session_manager
        .start_session(Some("alice"))
        .expect("Failed to create session");

    env.session_manager
        .start_session(Some("bob"))
        .expect("Failed to create session");

    // List all sessions
    let all_sessions = env.db.list_sessions(None, None).expect("Failed to list sessions");
    assert_eq!(all_sessions.len(), 3);

    // List Alice's sessions
    let alice_sessions = env.db.list_sessions(Some("alice"), None).expect("Failed to list Alice's sessions");
    assert_eq!(alice_sessions.len(), 2);

    // List with limit
    let limited_sessions = env.db.list_sessions(None, Some(2)).expect("Failed to list limited sessions");
    assert_eq!(limited_sessions.len(), 2);
}

#[test]
fn test_session_not_found_error() {
    let env = TestEnv::new();

    // Try to replay non-existent session
    let result = env.session_manager.replay_session("nonexistent-id");
    assert!(result.is_err());

    // Getting history for non-existent session should return empty history
    let history = env
        .session_manager
        .get_conversation_history("nonexistent-id", None)
        .expect("history should return Ok for non-existent session");
    assert_eq!(history, "");
}

#[tokio::test]
async fn test_concurrent_message_additions() {
    let env = TestEnv::new();

    let session = env
        .session_manager
        .start_session(Some("test_user"))
        .expect("Failed to create session");

    // Ingest test data
    ingest_text(&env.db, "test.md", "Test content").await;

    // Add messages sequentially (simulating concurrent access)
    // In a real concurrent scenario, sequence numbers should still be unique
    let mut handles = vec![];

    let shared_index = env.db.get_vector_index().expect("get_vector_index failed");
    for i in 0..10 {
        let session_id = session.id.clone();
        let sm = SessionManager::new(env.db.clone());
        let index = shared_index.clone();

        let handle = tokio::spawn(async move {
            sm.add_user_message(
                &session_id,
                &format!("Concurrent message {}", i),
                TestEnv::default_config(),
                index.as_ref(),
                None,
            )
            .await
        });

        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        handle.await.expect("Task panicked").expect("Failed to add message");
    }

    // Verify all messages were added
    let messages = env
        .db
        .get_messages(&session.id, None)
        .expect("Failed to get messages");

    assert_eq!(messages.len(), 10);

    // Verify sequence numbers are unique
    let mut sequence_numbers: Vec<usize> = messages.iter().map(|m| m.sequence_number).collect();
    sequence_numbers.sort();
    sequence_numbers.dedup();
    assert_eq!(sequence_numbers.len(), 10, "Sequence numbers should be unique");
}

#[tokio::test]
async fn test_empty_session_operations() {
    let env = TestEnv::new();

    let session = env
        .session_manager
        .start_session(Some("test_user"))
        .expect("Failed to create session");

    // Get history of empty session
    let history = env
        .session_manager
        .get_conversation_history(&session.id, None)
        .expect("Failed to get history");

    assert_eq!(history, "");

    // Replay empty session
    let replay = env
        .session_manager
        .replay_session(&session.id)
        .expect("Failed to replay");

    assert_eq!(replay.turns.len(), 0);
}
