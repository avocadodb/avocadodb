//! Storage backend abstraction for AvocadoDB
//!
//! This module provides a unified interface for database operations
//! supporting multiple backends (SQLite, PostgreSQL).
//!
//! # Architecture
//!
//! - `StorageBackend` trait defines all database operations
//! - `SqliteBackend` wraps existing Database with async interface
//! - `PostgresBackend` uses sqlx with pgvector for native vector search
//!
//! # Configuration
//!
//! Set `AVOCADO_BACKEND` environment variable:
//! - `sqlite` or unset: SQLite (default, zero-config)
//! - `sqlite:/path/to/db.sqlite`: SQLite with specific path
//! - `postgres://user:pass@host/db`: PostgreSQL with pgvector

mod traits;
mod sqlite;
#[cfg(feature = "postgres")]
mod postgres;
mod vector;
pub mod migrations;

pub use traits::{StorageBackend, StorageConfig};
pub use sqlite::SqliteBackend;
#[cfg(feature = "postgres")]
pub use postgres::PostgresBackend;
pub use vector::{VectorSearchProvider, VectorSearchResult};

use crate::types::Result;
use std::path::Path;

/// Create a storage backend from configuration
pub async fn create_backend(config: StorageConfig) -> Result<Box<dyn StorageBackend>> {
    match config {
        StorageConfig::Sqlite { path } => {
            Ok(Box::new(SqliteBackend::new(&path).await?))
        }
        #[cfg(feature = "postgres")]
        StorageConfig::Postgres { connection_string } => {
            Ok(Box::new(PostgresBackend::new(&connection_string).await?))
        }
        #[cfg(not(feature = "postgres"))]
        StorageConfig::Postgres { .. } => {
            Err(crate::types::Error::InvalidInput(
                "PostgreSQL support not compiled. Enable 'postgres' feature.".to_string()
            ))
        }
    }
}

/// Create backend from environment variable AVOCADO_BACKEND
///
/// # Arguments
/// * `default_path` - Default SQLite path if AVOCADO_BACKEND is not set
pub async fn create_backend_from_env<P: AsRef<Path>>(default_path: P) -> Result<Box<dyn StorageBackend>> {
    let config = StorageConfig::from_env(default_path.as_ref().to_string_lossy().as_ref());
    create_backend(config).await
}
