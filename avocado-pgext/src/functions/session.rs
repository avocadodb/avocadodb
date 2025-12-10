//! Session management functions for AvocadoDB extension
//!
//! Provides SQL functions for creating sessions, adding messages,
//! and retrieving conversation history.

use crate::error::AvocadoError;
use crate::spi;
use pgrx::datum::JsonB;
use pgrx::prelude::*;

/// Create a new session
///
/// # Arguments
/// * `user_id` - Optional user identifier
/// * `title` - Optional session title
///
/// # Returns
/// JSONB with session details (id, user_id, title, created_at)
///
/// # Example
/// ```sql
/// SELECT avocado_create_session('user@example.com', 'Support Chat');
/// ```
#[pg_extern]
fn avocado_create_session(
    user_id: default!(Option<&str>, "NULL"),
    title: default!(Option<&str>, "NULL"),
) -> JsonB {
    match spi::create_session(user_id, title) {
        Ok(session) => JsonB(serde_json::json!({
            "id": session.id,
            "user_id": session.user_id,
            "title": session.title,
            "created_at": session.created_at.to_rfc3339(),
        })),
        Err(e) => e.report(),
    }
}

/// Add a message to a session
///
/// # Arguments
/// * `session_id` - The session ID
/// * `role` - Message role: 'user', 'assistant', 'system', or 'tool'
/// * `content` - Message content
/// * `metadata` - Optional JSONB metadata
///
/// # Returns
/// JSONB with message details
///
/// # Example
/// ```sql
/// SELECT avocado_add_message(
///     'session-uuid',
///     'user',
///     'How do I login?',
///     '{"agent_id": "agent-uuid"}'::jsonb
/// );
/// ```
#[pg_extern]
fn avocado_add_message(
    session_id: &str,
    role: &str,
    content: &str,
    metadata: default!(Option<JsonB>, "NULL"),
) -> JsonB {
    // Validate role
    let valid_roles = ["user", "assistant", "system", "tool"];
    if !valid_roles.contains(&role) {
        AvocadoError::InvalidInput(format!(
            "Invalid role '{}'. Must be one of: user, assistant, system, tool",
            role
        ))
        .report();
    }

    match spi::add_message(session_id, role, content, metadata.map(|j| j.0)) {
        Ok(message) => JsonB(serde_json::json!({
            "id": message.id,
            "session_id": message.session_id,
            "role": message.role,
            "content": message.content,
            "sequence_number": message.sequence_number,
            "created_at": message.created_at.to_rfc3339(),
        })),
        Err(e) => e.report(),
    }
}

/// Get conversation history as formatted text
///
/// # Arguments
/// * `session_id` - The session ID
/// * `max_tokens` - Optional maximum tokens to include
///
/// # Returns
/// Formatted conversation history as TEXT
///
/// # Example
/// ```sql
/// SELECT avocado_get_conversation_history('session-uuid', 8000);
/// ```
#[pg_extern]
fn avocado_get_conversation_history(
    session_id: &str,
    max_tokens: default!(Option<i32>, "NULL"),
) -> String {
    match spi::get_conversation_history(session_id, max_tokens) {
        Ok(history) => history,
        Err(e) => e.report(),
    }
}

/// List sessions for a user
///
/// # Arguments
/// * `user_id` - Optional user ID to filter by
/// * `limit` - Maximum number of sessions to return (default 20)
///
/// # Returns
/// JSONB array of sessions
///
/// # Example
/// ```sql
/// SELECT avocado_list_sessions('user@example.com', 10);
/// ```
#[pg_extern]
fn avocado_list_sessions(
    user_id: default!(Option<&str>, "NULL"),
    limit: default!(i32, "20"),
) -> JsonB {
    match list_sessions_impl(user_id, limit) {
        Ok(sessions) => JsonB(serde_json::json!(sessions)),
        Err(e) => e.report(),
    }
}

fn list_sessions_impl(
    user_id: Option<&str>,
    limit: i32,
) -> Result<Vec<serde_json::Value>, AvocadoError> {
    use pgrx::spi::SpiClient;

    let sessions = pgrx::Spi::connect(|client| {
        let query = if user_id.is_some() {
            "SELECT id, user_id, title, metadata, created_at
             FROM avocado.sessions
             WHERE user_id = $1
             ORDER BY created_at DESC
             LIMIT $2"
        } else {
            "SELECT id, user_id, title, metadata, created_at
             FROM avocado.sessions
             ORDER BY created_at DESC
             LIMIT $1"
        };

        let args = if let Some(uid) = user_id {
            Some(vec![
                (PgBuiltInOids::TEXTOID.oid(), uid.into_datum()),
                (PgBuiltInOids::INT4OID.oid(), limit.into_datum()),
            ])
        } else {
            Some(vec![(PgBuiltInOids::INT4OID.oid(), limit.into_datum())])
        };

        let table_result = client.select(query, None, args)?;

        let mut sessions = Vec::new();
        for row in table_result {
            let session = serde_json::json!({
                "id": row.get_by_name::<String, &str>("id")?.unwrap_or_default(),
                "user_id": row.get_by_name::<String, &str>("user_id")?,
                "title": row.get_by_name::<String, &str>("title")?,
                "metadata": row.get_by_name::<JsonB, &str>("metadata")?.map(|j| j.0),
                "created_at": row.get_by_name::<chrono::DateTime<chrono::Utc>, &str>("created_at")?
                    .map(|dt| dt.to_rfc3339()),
            });
            sessions.push(session);
        }
        Ok::<_, pgrx::spi::Error>(sessions)
    })
    .map_err(|e| AvocadoError::Database(format!("Failed to list sessions: {}", e)))?;

    Ok(sessions)
}
