//! AvocadoDB PostgreSQL Extension
//!
//! This extension provides deterministic context compilation for AI agents
//! directly within PostgreSQL, leveraging pgvector for vector similarity search.
//!
//! # Usage
//!
//! ```sql
//! CREATE EXTENSION vector;
//! CREATE EXTENSION avocado;
//!
//! -- Check embedding configuration
//! SELECT avocado_embedding_config();
//!
//! -- Switch to Ollama (if running locally)
//! SELECT avocado_set_embedding_provider('ollama');
//! SELECT avocado_set_ollama_config('http://localhost:11434', 'bge-m3');
//!
//! -- Test embedding generation
//! SELECT avocado_test_embedding('Hello world');
//!
//! -- Ingest documents
//! SELECT avocado_ingest_artifact('docs/auth.md', 'Authentication uses JWT...');
//!
//! -- Compile context for a query
//! SELECT avocado_compile('How does auth work?', '{"token_budget": 4000}'::jsonb);
//! ```

use pgrx::prelude::*;

mod error;
pub mod embedding;  // Public to expose SQL functions
mod spi;
mod functions;

pgrx::pg_module_magic!();

/// Extension initialization
#[pg_extern]
fn avocado_version() -> &'static str {
    "2.2.0"
}

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use pgrx::prelude::*;

    #[pg_test]
    fn test_version() {
        let version = crate::avocado_version();
        assert_eq!(version, "0.1.0");
    }
}

#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {
        // Setup code for tests
    }

    pub fn postgresql_conf_options() -> Vec<&'static str> {
        vec![]
    }
}
