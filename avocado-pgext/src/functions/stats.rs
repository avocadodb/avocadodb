//! Statistics functions for AvocadoDB extension

use crate::error::AvocadoError;
use crate::spi;
use pgrx::datum::JsonB;
use pgrx::prelude::*;

/// Get database statistics
///
/// Returns a JSONB object with counts and totals:
/// ```sql
/// SELECT avocado_stats();
/// -- Returns: {"artifacts": 10, "spans": 150, "sessions": 5, ...}
/// ```
#[pg_extern]
fn avocado_stats() -> JsonB {
    match spi::get_stats() {
        Ok(stats) => JsonB(serde_json::json!({
            "artifacts": stats.artifact_count,
            "spans": stats.span_count,
            "sessions": stats.session_count,
            "messages": stats.message_count,
            "agents": stats.agent_count,
            "total_tokens": stats.total_tokens,
            "embedding_provider": crate::embedding::provider_name(),
            "embedding_model": crate::embedding::model_name(),
            "embedding_dimension": crate::embedding::embedding_dimension(),
            "version": crate::avocado_version(),
        })),
        Err(e) => {
            e.report();
        }
    }
}

/// Initialize the AvocadoDB schema
///
/// Creates all necessary tables if they don't exist.
/// Should be called after CREATE EXTENSION avocado.
#[pg_extern]
fn avocado_init() -> &'static str {
    match spi::ensure_schema() {
        Ok(_) => "AvocadoDB schema initialized successfully",
        Err(e) => e.report(),
    }
}
