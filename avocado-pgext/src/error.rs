//! Error types for the AvocadoDB PostgreSQL extension

use pgrx::prelude::*;
use thiserror::Error;

/// Errors that can occur in the AvocadoDB extension
#[derive(Error, Debug)]
pub enum AvocadoError {
    #[error("Embedding error: {0}")]
    Embedding(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<serde_json::Error> for AvocadoError {
    fn from(err: serde_json::Error) -> Self {
        AvocadoError::Serialization(err.to_string())
    }
}

/// Convert AvocadoError to PostgreSQL error
impl AvocadoError {
    pub fn report(self) -> ! {
        match self {
            AvocadoError::InvalidInput(msg) => {
                pgrx::error!("Invalid input: {}", msg);
            }
            AvocadoError::NotFound(msg) => {
                pgrx::error!("Not found: {}", msg);
            }
            AvocadoError::Embedding(msg) => {
                pgrx::error!("Embedding error: {}", msg);
            }
            AvocadoError::Database(msg) => {
                pgrx::error!("Database error: {}", msg);
            }
            AvocadoError::Serialization(msg) => {
                pgrx::error!("Serialization error: {}", msg);
            }
            AvocadoError::Internal(msg) => {
                pgrx::error!("Internal error: {}", msg);
            }
        }
    }
}

pub type Result<T> = std::result::Result<T, AvocadoError>;
