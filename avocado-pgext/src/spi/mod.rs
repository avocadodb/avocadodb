//! SPI (Server Programming Interface) operations for AvocadoDB
//!
//! This module provides database operations using PostgreSQL's SPI,
//! which allows safe SQL execution within the extension context.

mod backend;

pub use backend::*;
