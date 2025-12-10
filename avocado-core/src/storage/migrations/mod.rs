//! Database migrations for each backend

pub mod sqlite;
#[cfg(feature = "postgres")]
pub mod postgres;
