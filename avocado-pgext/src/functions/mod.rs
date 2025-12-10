//! SQL functions exposed by the AvocadoDB extension
//!
//! All functions are prefixed with `avocado_` to avoid namespace collisions.

mod stats;
mod ingest;
mod compile;
mod session;
mod agent;

// Re-export all public functions
pub use stats::*;
pub use ingest::*;
pub use compile::*;
pub use session::*;
pub use agent::*;
