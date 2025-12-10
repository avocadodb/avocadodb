//! Multi-agent orchestration functions for AvocadoDB extension
//!
//! Provides SQL functions for registering agents and tracking
//! agent relations (agreements, disagreements, questions).

use crate::error::AvocadoError;
use crate::spi;
use pgrx::datum::JsonB;
use pgrx::prelude::*;

/// Register a new agent
///
/// # Arguments
/// * `name` - Unique agent name
/// * `role` - Agent's role description
/// * `model` - Model identifier (e.g., "gpt-4", "qwen2.5-coder:32b")
/// * `system_prompt` - Optional system prompt for the agent
/// * `capabilities` - Optional JSONB capabilities list
///
/// # Returns
/// JSONB with agent details
///
/// # Example
/// ```sql
/// SELECT avocado_register_agent(
///     'moderator',
///     'Discussion Moderator',
///     'gpt-4',
///     'You are a neutral moderator...',
///     '["summarize", "mediate"]'::jsonb
/// );
/// ```
#[pg_extern]
fn avocado_register_agent(
    name: &str,
    role: &str,
    model: &str,
    system_prompt: default!(Option<&str>, "NULL"),
    capabilities: default!(Option<JsonB>, "NULL"),
) -> JsonB {
    match spi::register_agent(name, role, model, system_prompt, capabilities.map(|j| j.0)) {
        Ok(agent) => JsonB(serde_json::json!({
            "id": agent.id,
            "name": agent.name,
            "role": agent.role,
            "model": agent.model,
            "system_prompt": agent.system_prompt,
            "capabilities": agent.capabilities,
            "created_at": agent.created_at.to_rfc3339(),
        })),
        Err(e) => e.report(),
    }
}

/// List all registered agents
///
/// # Returns
/// JSONB array of agents
///
/// # Example
/// ```sql
/// SELECT avocado_list_agents();
/// ```
#[pg_extern]
fn avocado_list_agents() -> JsonB {
    match spi::list_agents() {
        Ok(agents) => {
            let json_agents: Vec<serde_json::Value> = agents
                .into_iter()
                .map(|a| {
                    serde_json::json!({
                        "id": a.id,
                        "name": a.name,
                        "role": a.role,
                        "model": a.model,
                        "system_prompt": a.system_prompt,
                        "capabilities": a.capabilities,
                        "created_at": a.created_at.to_rfc3339(),
                    })
                })
                .collect();
            JsonB(serde_json::json!(json_agents))
        }
        Err(e) => e.report(),
    }
}

/// Add an agent relation (agree, disagree, question)
///
/// # Arguments
/// * `session_id` - The session ID
/// * `message_id` - The message ID where this relation is expressed
/// * `from_agent_id` - The agent expressing the stance
/// * `target_message_id` - The message being responded to
/// * `stance` - One of: 'agree', 'disagree', 'neutral', 'question'
///
/// # Returns
/// JSONB with relation details
///
/// # Example
/// ```sql
/// SELECT avocado_add_agent_relation(
///     'session-uuid',
///     'message-uuid',
///     'agent-1-uuid',
///     'target-message-uuid',
///     'agree'
/// );
/// ```
#[pg_extern]
fn avocado_add_agent_relation(
    session_id: &str,
    message_id: &str,
    from_agent_id: &str,
    target_message_id: &str,
    stance: &str,
) -> JsonB {
    // Validate stance
    let valid_stances = ["agree", "disagree", "neutral", "question"];
    if !valid_stances.contains(&stance) {
        AvocadoError::InvalidInput(format!(
            "Invalid stance '{}'. Must be one of: agree, disagree, neutral, question",
            stance
        ))
        .report();
    }

    match add_relation_impl(session_id, message_id, from_agent_id, target_message_id, stance) {
        Ok(relation) => JsonB(relation),
        Err(e) => e.report(),
    }
}

fn add_relation_impl(
    session_id: &str,
    message_id: &str,
    from_agent_id: &str,
    target_message_id: &str,
    stance: &str,
) -> Result<serde_json::Value, AvocadoError> {
    let relation_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now();

    pgrx::Spi::connect(|mut client| {
        client.update(
            "INSERT INTO avocado.agent_relations (id, session_id, message_id, from_agent_id, target_message_id, stance, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
            None,
            Some(vec![
                (PgBuiltInOids::TEXTOID.oid(), relation_id.clone().into_datum()),
                (PgBuiltInOids::TEXTOID.oid(), session_id.into_datum()),
                (PgBuiltInOids::TEXTOID.oid(), message_id.into_datum()),
                (PgBuiltInOids::TEXTOID.oid(), from_agent_id.into_datum()),
                (PgBuiltInOids::TEXTOID.oid(), target_message_id.into_datum()),
                (PgBuiltInOids::TEXTOID.oid(), stance.into_datum()),
                (PgBuiltInOids::TIMESTAMPTZOID.oid(), now.into_datum()),
            ]),
        )?;
        Ok::<_, pgrx::spi::Error>(())
    })
    .map_err(|e| AvocadoError::Database(format!("Failed to add relation: {}", e)))?;

    Ok(serde_json::json!({
        "id": relation_id,
        "session_id": session_id,
        "message_id": message_id,
        "from_agent_id": from_agent_id,
        "target_message_id": target_message_id,
        "stance": stance,
        "created_at": now.to_rfc3339(),
    }))
}

/// Get agent relations for a session
///
/// Returns relations grouped by stance with agent names resolved.
///
/// # Arguments
/// * `session_id` - The session ID
///
/// # Returns
/// JSONB with relations and agents
///
/// # Example
/// ```sql
/// SELECT avocado_get_agent_relations('session-uuid');
/// -- Returns: {"relations": {"agreements": [...], "disagreements": [...], ...}, "agents": [...]}
/// ```
#[pg_extern]
fn avocado_get_agent_relations(session_id: &str) -> JsonB {
    match get_relations_impl(session_id) {
        Ok(result) => JsonB(result),
        Err(e) => e.report(),
    }
}

fn get_relations_impl(session_id: &str) -> Result<serde_json::Value, AvocadoError> {
    let (relations, agents) = pgrx::Spi::connect(|client| {
        // Get relations with agent names
        let relation_rows = client.select(
            "SELECT r.id, r.message_id, r.stance, r.target_message_id, r.created_at,
                    fa.id as from_agent_id, fa.name as from_name, fa.model as from_model,
                    ta.id as to_agent_id, ta.name as to_name, ta.model as to_model
             FROM avocado.agent_relations r
             JOIN avocado.agents fa ON r.from_agent_id = fa.id
             LEFT JOIN avocado.messages tm ON r.target_message_id = tm.id
             LEFT JOIN avocado.agents ta ON (tm.metadata->>'agent_id')::text = ta.id
             WHERE r.session_id = $1
             ORDER BY r.created_at",
            None,
            Some(vec![(PgBuiltInOids::TEXTOID.oid(), session_id.into_datum())]),
        )?;

        let mut agreements = Vec::new();
        let mut disagreements = Vec::new();
        let mut questions = Vec::new();
        let mut neutrals = Vec::new();

        for row in relation_rows {
            let stance: String = row.get_by_name::<String, &str>("stance")?.unwrap_or_default();
            let relation = serde_json::json!({
                "id": row.get_by_name::<String, &str>("id")?.unwrap_or_default(),
                "message_id": row.get_by_name::<String, &str>("message_id")?.unwrap_or_default(),
                "target_message_id": row.get_by_name::<String, &str>("target_message_id")?,
                "from_agent_id": row.get_by_name::<String, &str>("from_agent_id")?.unwrap_or_default(),
                "from_name": row.get_by_name::<String, &str>("from_name")?.unwrap_or_default(),
                "from_model": row.get_by_name::<String, &str>("from_model")?.unwrap_or_default(),
                "to_agent_id": row.get_by_name::<String, &str>("to_agent_id")?,
                "to_name": row.get_by_name::<String, &str>("to_name")?,
                "to_model": row.get_by_name::<String, &str>("to_model")?,
            });

            match stance.as_str() {
                "agree" => agreements.push(relation),
                "disagree" => disagreements.push(relation),
                "question" => questions.push(relation),
                _ => neutrals.push(relation),
            }
        }

        // Get participating agents
        let agent_rows = client.select(
            "SELECT DISTINCT a.id, a.name, a.role, a.model
             FROM avocado.agents a
             JOIN avocado.messages m ON (m.metadata->>'agent_id')::text = a.id
             WHERE m.session_id = $1",
            None,
            Some(vec![(PgBuiltInOids::TEXTOID.oid(), session_id.into_datum())]),
        )?;

        let mut agents = Vec::new();
        for row in agent_rows {
            agents.push(serde_json::json!({
                "id": row.get_by_name::<String, &str>("id")?.unwrap_or_default(),
                "name": row.get_by_name::<String, &str>("name")?.unwrap_or_default(),
                "role": row.get_by_name::<String, &str>("role")?.unwrap_or_default(),
                "model": row.get_by_name::<String, &str>("model")?.unwrap_or_default(),
            }));
        }

        Ok::<_, pgrx::spi::Error>((
            serde_json::json!({
                "agreements": agreements,
                "disagreements": disagreements,
                "questions": questions,
                "neutrals": neutrals,
            }),
            agents,
        ))
    })
    .map_err(|e| AvocadoError::Database(format!("Failed to get relations: {}", e)))?;

    Ok(serde_json::json!({
        "relations": relations,
        "agents": agents,
    }))
}
