
//! CLI command implementations

pub mod benchmark;
pub mod recommend;
pub mod session;

pub use benchmark::run_benchmark;
pub use recommend::recommend_model;
pub use session::{handle_session_command, SessionCommands};
