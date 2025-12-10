//! Storage backend abstraction for AvocadoDB
//!
//! This module provides a unified interface for database operations.
//! The SQLite backend is the default for the standalone server.
//!
//! For PostgreSQL support, use the `avocado-pgext` PostgreSQL extension
//! which provides native SQL functions within PostgreSQL.
//!
//! # Architecture
//!
//! - `StorageBackend` trait defines all database operations
//! - `SqliteBackend` wraps existing Database with async interface
//!
//! # Configuration
//!
//! Set `AVOCADO_BACKEND` environment variable:
//! - `sqlite` or unset: SQLite (default, zero-config)
//! - `sqlite:/path/to/db.sqlite`: SQLite with specific path

pub mod migrations;
mod sqlite;
mod traits;
mod vector;

pub use sqlite::SqliteBackend;
pub use traits::{StorageBackend, StorageConfig};
pub use vector::{VectorSearchProvider, VectorSearchResult};

use crate::types::Result;
use std::path::Path;

/// Create a storage backend from configuration
pub async fn create_backend(config: StorageConfig) -> Result<Box<dyn StorageBackend>> {
    match config {
        StorageConfig::Sqlite { path } => Ok(Box::new(SqliteBackend::new(&path).await?)),
        StorageConfig::Postgres { .. } => Err(crate::types::Error::InvalidInput(
            "PostgreSQL client-library support removed. Use avocado-pgext extension instead."
                .to_string(),
        )),
    }
}

/// Create backend from environment variable AVOCADO_BACKEND
///
/// # Arguments
/// * `default_path` - Default SQLite path if AVOCADO_BACKEND is not set
pub async fn create_backend_from_env<P: AsRef<Path>>(
    default_path: P,
) -> Result<Box<dyn StorageBackend>> {
    let config = StorageConfig::from_env(default_path.as_ref().to_string_lossy().as_ref());
    create_backend(config).await
}
