//! Database migrations for each backend
//!
//! The postgres module is always available as reference schema for
//! the avocado-pgext PostgreSQL extension.

pub mod sqlite;
pub mod postgres;
