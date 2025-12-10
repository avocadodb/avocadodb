//! Integration tests for session management API endpoints
//!
//! Tests all 7 session endpoints:
//! - POST /sessions - Create new session
//! - GET /sessions - List sessions
//! - GET /sessions/:id - Get session with messages
//! - POST /sessions/:id/messages - Add message to session
//! - POST /sessions/:id/compile - Compile query in session context
//! - GET /sessions/:id/history - Get formatted conversation history
//! - DELETE /sessions/:id - Delete session

use serde_json::json;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to setup a test environment
struct TestEnv {
    _temp_dir: TempDir,
    project_path: PathBuf,
    server_url: String,
}

impl TestEnv {
    fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let project_path = temp_dir.path().to_path_buf();

        // Create .avocado directory
        fs::create_dir_all(project_path.join(".avocado")).expect("Failed to create .avocado dir");

        Self {
            _temp_dir: temp_dir,
            project_path,
            server_url: "http://localhost:8765".to_string(),
        }
    }

    fn project_param(&self) -> String {
        self.project_path.to_string_lossy().to_string()
    }
}

#[tokio::test]
#[ignore] // Run with: cargo test --test session_api_tests -- --ignored
async fn test_create_session() {
    let env = TestEnv::new();
    let client = reqwest::Client::new();

    // Create session without user_id or title
    let response = client
        .post(&format!("{}/sessions", env.server_url))
        .json(&json!({
            "project": env.project_param()
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert!(body["session"]["id"].is_string());
    assert!(body["session"]["user_id"].is_null());
    assert!(body["session"]["title"].is_null());

    // Create session with user_id and title
    let response = client
        .post(&format!("{}/sessions", env.server_url))
        .json(&json!({
            "user_id": "test_user",
            "title": "Test Session",
            "project": env.project_param()
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert!(body["session"]["id"].is_string());
    assert_eq!(body["session"]["user_id"], "test_user");
    assert_eq!(body["session"]["title"], "Test Session");
}

#[tokio::test]
#[ignore]
async fn test_list_sessions() {
    let env = TestEnv::new();
    let client = reqwest::Client::new();

    // Create a few sessions
    for i in 0..3 {
        client
            .post(&format!("{}/sessions", env.server_url))
            .json(&json!({
                "user_id": format!("user_{}", i),
                "project": env.project_param()
            }))
            .send()
            .await
            .expect("Failed to create session");
    }

    // List all sessions
    let response = client
        .get(&format!(
            "{}/sessions?project={}",
            env.server_url,
            env.project_param()
        ))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert!(body["sessions"].is_array());
    assert_eq!(body["sessions"].as_array().unwrap().len(), 3);

    // List sessions with limit
    let response = client
        .get(&format!(
            "{}/sessions?project={}&limit=2",
            env.server_url,
            env.project_param()
        ))
        .send()
        .await
        .expect("Failed to send request");

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["sessions"].as_array().unwrap().len(), 2);

    // List sessions for specific user
    let response = client
        .get(&format!(
            "{}/sessions?project={}&user_id=user_1",
            env.server_url,
            env.project_param()
        ))
        .send()
        .await
        .expect("Failed to send request");

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["sessions"].as_array().unwrap().len(), 1);
    assert_eq!(body["sessions"][0]["user_id"], "user_1");
}

#[tokio::test]
#[ignore]
async fn test_get_session() {
    let env = TestEnv::new();
    let client = reqwest::Client::new();

    // Create a session
    let create_response = client
        .post(&format!("{}/sessions", env.server_url))
        .json(&json!({
            "user_id": "test_user",
            "project": env.project_param()
        }))
        .send()
        .await
        .expect("Failed to create session");

    let create_body: serde_json::Value = create_response.json().await.unwrap();
    let session_id = create_body["session"]["id"].as_str().unwrap();

    // Get the session
    let response = client
        .get(&format!(
            "{}/sessions/{}?project={}",
            env.server_url,
            session_id,
            env.project_param()
        ))
        .send()
        .await
        .expect("Failed to get session");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["session"]["id"], session_id);
    assert!(body["messages"].is_array());
    assert_eq!(body["messages"].as_array().unwrap().len(), 0);

    // Test 404 for non-existent session
    let response = client
        .get(&format!(
            "{}/sessions/nonexistent?project={}",
            env.server_url,
            env.project_param()
        ))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 404);
}

#[tokio::test]
#[ignore]
async fn test_add_message() {
    let env = TestEnv::new();
    let client = reqwest::Client::new();

    // Create a session
    let create_response = client
        .post(&format!("{}/sessions", env.server_url))
        .json(&json!({
            "project": env.project_param()
        }))
        .send()
        .await
        .expect("Failed to create session");

    let create_body: serde_json::Value = create_response.json().await.unwrap();
    let session_id = create_body["session"]["id"].as_str().unwrap();

    // Add a user message
    let response = client
        .post(&format!(
            "{}/sessions/{}/messages",
            env.server_url, session_id
        ))
        .json(&json!({
            "role": "user",
            "content": "Hello, world!",
            "project": env.project_param()
        }))
        .send()
        .await
        .expect("Failed to add message");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert!(body["message"]["id"].is_string());
    assert_eq!(body["message"]["role"], "user");
    assert_eq!(body["message"]["content"], "Hello, world!");
    assert_eq!(body["message"]["sequence_number"], 0);

    // Add an assistant message
    let response = client
        .post(&format!(
            "{}/sessions/{}/messages",
            env.server_url, session_id
        ))
        .json(&json!({
            "role": "assistant",
            "content": "Hi there!",
            "project": env.project_param()
        }))
        .send()
        .await
        .expect("Failed to add message");

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["message"]["role"], "assistant");
    assert_eq!(body["message"]["sequence_number"], 1);

    // Test invalid role
    let response = client
        .post(&format!(
            "{}/sessions/{}/messages",
            env.server_url, session_id
        ))
        .json(&json!({
            "role": "invalid",
            "content": "Test",
            "project": env.project_param()
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 400);
}

#[tokio::test]
#[ignore]
async fn test_session_compile() {
    let env = TestEnv::new();
    let client = reqwest::Client::new();

    // First, ingest some test data
    client
        .post(&format!("{}/ingest", env.server_url))
        .json(&json!({
            "path": "test.txt",
            "content": "Rust is a systems programming language that runs blazingly fast.",
            "project": env.project_param()
        }))
        .send()
        .await
        .expect("Failed to ingest");

    // Create a session
    let create_response = client
        .post(&format!("{}/sessions", env.server_url))
        .json(&json!({
            "project": env.project_param()
        }))
        .send()
        .await
        .expect("Failed to create session");

    let create_body: serde_json::Value = create_response.json().await.unwrap();
    let session_id = create_body["session"]["id"].as_str().unwrap();

    // Compile a query in the session
    let response = client
        .post(&format!(
            "{}/sessions/{}/compile",
            env.server_url, session_id
        ))
        .json(&json!({
            "query": "What is Rust?",
            "project": env.project_param()
        }))
        .send()
        .await
        .expect("Failed to compile");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");

    // Check message was created
    assert!(body["message"]["id"].is_string());
    assert_eq!(body["message"]["role"], "user");
    assert_eq!(body["message"]["content"], "What is Rust?");

    // Check working set was returned
    assert!(body["working_set"]["text"].is_string());
    assert!(body["working_set"]["citations"].is_array());
    assert!(body["working_set"]["tokens_used"].is_number());

    // Test 404 for non-existent session
    let response = client
        .post(&format!("{}/sessions/nonexistent/compile", env.server_url))
        .json(&json!({
            "query": "test",
            "project": env.project_param()
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 404);
}

#[tokio::test]
#[ignore]
async fn test_get_history() {
    let env = TestEnv::new();
    let client = reqwest::Client::new();

    // Create a session
    let create_response = client
        .post(&format!("{}/sessions", env.server_url))
        .json(&json!({
            "project": env.project_param()
        }))
        .send()
        .await
        .expect("Failed to create session");

    let create_body: serde_json::Value = create_response.json().await.unwrap();
    let session_id = create_body["session"]["id"].as_str().unwrap();

    // Add some messages
    client
        .post(&format!(
            "{}/sessions/{}/messages",
            env.server_url, session_id
        ))
        .json(&json!({
            "role": "user",
            "content": "Hello",
            "project": env.project_param()
        }))
        .send()
        .await
        .expect("Failed to add message");

    client
        .post(&format!(
            "{}/sessions/{}/messages",
            env.server_url, session_id
        ))
        .json(&json!({
            "role": "assistant",
            "content": "Hi there!",
            "project": env.project_param()
        }))
        .send()
        .await
        .expect("Failed to add message");

    // Get conversation history
    let response = client
        .get(&format!(
            "{}/sessions/{}/history?project={}",
            env.server_url,
            session_id,
            env.project_param()
        ))
        .send()
        .await
        .expect("Failed to get history");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    let history = body["history"].as_str().unwrap();

    assert!(history.contains("User: Hello"));
    assert!(history.contains("Assistant: Hi there!"));

    // Test with token limit
    let response = client
        .get(&format!(
            "{}/sessions/{}/history?project={}&max_tokens=10",
            env.server_url,
            session_id,
            env.project_param()
        ))
        .send()
        .await
        .expect("Failed to get history");

    assert_eq!(response.status(), 200);
}

#[tokio::test]
#[ignore]
async fn test_session_replay() {
    let env = TestEnv::new();
    let client = reqwest::Client::new();

    // Ingest test data
    client
        .post(&format!("{}/ingest", env.server_url))
        .json(&json!({
            "path": "test.txt",
            "content": "Test content for replay.",
            "project": env.project_param()
        }))
        .send()
        .await
        .expect("Failed to ingest");

    // Create a session
    let create_response = client
        .post(&format!("{}/sessions", env.server_url))
        .json(&json!({
            "project": env.project_param()
        }))
        .send()
        .await
        .expect("Failed to create session");

    let create_body: serde_json::Value = create_response.json().await.unwrap();
    let session_id = create_body["session"]["id"].as_str().unwrap();

    // Create a conversation with compile
    client
        .post(&format!(
            "{}/sessions/{}/compile",
            env.server_url, session_id
        ))
        .json(&json!({
            "query": "First query",
            "project": env.project_param()
        }))
        .send()
        .await
        .expect("Failed to compile");

    client
        .post(&format!(
            "{}/sessions/{}/messages",
            env.server_url, session_id
        ))
        .json(&json!({
            "role": "assistant",
            "content": "First response",
            "project": env.project_param()
        }))
        .send()
        .await
        .expect("Failed to add message");

    // Get replay
    let response = client
        .get(&format!(
            "{}/sessions/{}/replay?project={}",
            env.server_url,
            session_id,
            env.project_param()
        ))
        .send()
        .await
        .expect("Failed to get replay");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");

    assert_eq!(body["session"]["id"], session_id);
    assert!(body["turns"].is_array());
    assert_eq!(body["turns"].as_array().unwrap().len(), 1);

    let turn = &body["turns"][0];
    assert_eq!(turn["user_message"]["content"], "First query");
    assert!(turn["working_set"].is_object());
    assert_eq!(turn["assistant_message"]["content"], "First response");
}

#[tokio::test]
#[ignore]
async fn test_delete_session() {
    let env = TestEnv::new();
    let client = reqwest::Client::new();

    // Create a session
    let create_response = client
        .post(&format!("{}/sessions", env.server_url))
        .json(&json!({
            "project": env.project_param()
        }))
        .send()
        .await
        .expect("Failed to create session");

    let create_body: serde_json::Value = create_response.json().await.unwrap();
    let session_id = create_body["session"]["id"].as_str().unwrap();

    // Delete the session
    let response = client
        .delete(&format!(
            "{}/sessions/{}?project={}",
            env.server_url,
            session_id,
            env.project_param()
        ))
        .send()
        .await
        .expect("Failed to delete session");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["success"], true);

    // Verify session is gone
    let response = client
        .get(&format!(
            "{}/sessions/{}?project={}",
            env.server_url,
            session_id,
            env.project_param()
        ))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 404);
}

#[tokio::test]
#[ignore]
async fn test_full_session_workflow() {
    let env = TestEnv::new();
    let client = reqwest::Client::new();

    // 1. Ingest some documents
    for i in 0..3 {
        client
            .post(&format!("{}/ingest", env.server_url))
            .json(&json!({
                "path": format!("doc_{}.txt", i),
                "content": format!("Document {} content about Rust programming.", i),
                "project": env.project_param()
            }))
            .send()
            .await
            .expect("Failed to ingest");
    }

    // 2. Create a session
    let create_response = client
        .post(&format!("{}/sessions", env.server_url))
        .json(&json!({
            "user_id": "alice",
            "title": "Learning Rust",
            "project": env.project_param()
        }))
        .send()
        .await
        .expect("Failed to create session");

    let create_body: serde_json::Value = create_response.json().await.unwrap();
    let session_id = create_body["session"]["id"].as_str().unwrap();

    // 3. Have a multi-turn conversation
    // Turn 1
    client
        .post(&format!(
            "{}/sessions/{}/compile",
            env.server_url, session_id
        ))
        .json(&json!({
            "query": "What is Rust?",
            "project": env.project_param()
        }))
        .send()
        .await
        .expect("Failed to compile");

    client
        .post(&format!(
            "{}/sessions/{}/messages",
            env.server_url, session_id
        ))
        .json(&json!({
            "role": "assistant",
            "content": "Rust is a systems programming language.",
            "project": env.project_param()
        }))
        .send()
        .await
        .expect("Failed to add message");

    // Turn 2
    client
        .post(&format!(
            "{}/sessions/{}/compile",
            env.server_url, session_id
        ))
        .json(&json!({
            "query": "Tell me more",
            "project": env.project_param()
        }))
        .send()
        .await
        .expect("Failed to compile");

    client
        .post(&format!(
            "{}/sessions/{}/messages",
            env.server_url, session_id
        ))
        .json(&json!({
            "role": "assistant",
            "content": "Rust focuses on safety and performance.",
            "project": env.project_param()
        }))
        .send()
        .await
        .expect("Failed to add message");

    // 4. Get conversation history
    let history_response = client
        .get(&format!(
            "{}/sessions/{}/history?project={}",
            env.server_url,
            session_id,
            env.project_param()
        ))
        .send()
        .await
        .expect("Failed to get history");

    let history_body: serde_json::Value = history_response.json().await.unwrap();
    let history = history_body["history"].as_str().unwrap();

    assert!(history.contains("What is Rust?"));
    assert!(history.contains("systems programming language"));
    assert!(history.contains("Tell me more"));

    // 5. Get session details
    let session_response = client
        .get(&format!(
            "{}/sessions/{}?project={}",
            env.server_url,
            session_id,
            env.project_param()
        ))
        .send()
        .await
        .expect("Failed to get session");

    let session_body: serde_json::Value = session_response.json().await.unwrap();
    assert_eq!(session_body["session"]["user_id"], "alice");
    assert_eq!(session_body["session"]["title"], "Learning Rust");
    assert_eq!(session_body["messages"].as_array().unwrap().len(), 4); // 2 user + 2 assistant

    // 6. Replay the session
    let replay_response = client
        .get(&format!(
            "{}/sessions/{}/replay?project={}",
            env.server_url,
            session_id,
            env.project_param()
        ))
        .send()
        .await
        .expect("Failed to replay");

    let replay_body: serde_json::Value = replay_response.json().await.unwrap();
    assert_eq!(replay_body["turns"].as_array().unwrap().len(), 2);

    // 7. List sessions
    let list_response = client
        .get(&format!(
            "{}/sessions?project={}&user_id=alice",
            env.server_url,
            env.project_param()
        ))
        .send()
        .await
        .expect("Failed to list sessions");

    let list_body: serde_json::Value = list_response.json().await.unwrap();
    assert_eq!(list_body["sessions"].as_array().unwrap().len(), 1);

    // 8. Delete the session
    let delete_response = client
        .delete(&format!(
            "{}/sessions/{}?project={}",
            env.server_url,
            session_id,
            env.project_param()
        ))
        .send()
        .await
        .expect("Failed to delete");

    assert_eq!(delete_response.status(), 200);
}
