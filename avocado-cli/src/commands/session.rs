//! Session management commands
//!
//! Commands for managing conversation sessions via CLI.

use anyhow::Result;
use avocado_core::{
    db::Database, session::SessionManager, CompilerConfig, MessageRole,
};
use clap::Subcommand;
use console::style;
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum SessionCommands {
    /// Create a new session
    Create {
        /// User ID (optional)
        #[arg(long)]
        user_id: Option<String>,

        /// Session title (optional)
        #[arg(long)]
        title: Option<String>,

        /// Database path
        #[arg(short, long, default_value = ".avocado/db.sqlite")]
        db_path: PathBuf,
    },

    /// List sessions
    List {
        /// Filter by user ID (optional)
        #[arg(long)]
        user_id: Option<String>,

        /// Limit number of results
        #[arg(long)]
        limit: Option<usize>,

        /// Database path
        #[arg(short, long, default_value = ".avocado/db.sqlite")]
        db_path: PathBuf,
    },

    /// Show session details
    Show {
        /// Session ID
        session_id: String,

        /// Database path
        #[arg(short, long, default_value = ".avocado/db.sqlite")]
        db_path: PathBuf,
    },

    /// Add a message to a session
    Message {
        /// Session ID
        session_id: String,

        /// Message role (user, assistant, system, tool)
        #[arg(long)]
        role: String,

        /// Message content
        #[arg(long)]
        content: String,

        /// Database path
        #[arg(short, long, default_value = ".avocado/db.sqlite")]
        db_path: PathBuf,
    },

    /// Compile context for a query in a session
    Compile {
        /// Session ID
        session_id: String,

        /// Query
        query: String,

        /// Token budget
        #[arg(short, long, default_value = "8000")]
        budget: usize,

        /// Database path
        #[arg(short, long, default_value = ".avocado/db.sqlite")]
        db_path: PathBuf,
    },

    /// Get conversation history
    History {
        /// Session ID
        session_id: String,

        /// Maximum tokens (optional)
        #[arg(long)]
        max_tokens: Option<usize>,

        /// Database path
        #[arg(short, long, default_value = ".avocado/db.sqlite")]
        db_path: PathBuf,
    },

    /// Replay a session (for debugging)
    Replay {
        /// Session ID
        session_id: String,

        /// Database path
        #[arg(short, long, default_value = ".avocado/db.sqlite")]
        db_path: PathBuf,

        /// Output as JSON
        #[arg(short, long)]
        json: bool,
    },

    /// Delete a session
    Delete {
        /// Session ID
        session_id: String,

        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,

        /// Database path
        #[arg(short, long, default_value = ".avocado/db.sqlite")]
        db_path: PathBuf,
    },
}

pub async fn handle_session_command(cmd: SessionCommands) -> Result<()> {
    match cmd {
        SessionCommands::Create {
            user_id,
            title,
            db_path,
        } => {
            let db = Database::new(&db_path)?;
            let manager = SessionManager::new(db.clone());

            let session = manager.start_session(user_id.as_deref())?;

            // Update title if provided
            if let Some(title) = &title {
                db.update_session(&session.id, Some(title), None)?;
            }

            println!(
                "{} Created session: {}",
                style("✓").green().bold(),
                style(&session.id).cyan()
            );

            if let Some(uid) = &user_id {
                println!("  {} {}", style("User:").bold(), uid);
            }

            if let Some(t) = &title {
                println!("  {} {}", style("Title:").bold(), t);
            }

            println!(
                "  {} {}",
                style("Created:").bold(),
                session.created_at.format("%Y-%m-%d %H:%M:%S")
            );
        }

        SessionCommands::List {
            user_id,
            limit,
            db_path,
        } => {
            let db = Database::new(&db_path)?;
            let sessions = db.list_sessions(user_id.as_deref(), limit)?;

            if sessions.is_empty() {
                println!("{} No sessions found", style("ℹ").blue());
                return Ok(());
            }

            println!(
                "\n{} {}",
                style("Found").bold(),
                style(format!("{} sessions", sessions.len())).cyan().bold()
            );
            println!("{}", style("─".repeat(80)).dim());

            for session in sessions {
                println!(
                    "\n{} {}",
                    style("Session:").bold(),
                    style(&session.id).cyan()
                );

                if let Some(uid) = &session.user_id {
                    println!("  {} {}", style("User:").dim(), uid);
                }

                if let Some(title) = &session.title {
                    println!("  {} {}", style("Title:").dim(), title);
                }

                println!(
                    "  {} {}",
                    style("Created:").dim(),
                    session.created_at.format("%Y-%m-%d %H:%M:%S")
                );

                if let Some(last_msg) = &session.last_message_at {
                    println!(
                        "  {} {}",
                        style("Last message:").dim(),
                        last_msg.format("%Y-%m-%d %H:%M:%S")
                    );
                }
            }

            println!();
        }

        SessionCommands::Show {
            session_id,
            db_path,
        } => {
            let db = Database::new(&db_path)?;

            let session = db.get_session(&session_id)?.ok_or_else(|| {
                anyhow::anyhow!("Session not found: {}", session_id)
            })?;

            let messages = db.get_messages(&session_id, None)?;

            println!(
                "\n{} {}",
                style("Session:").bold(),
                style(&session.id).cyan().bold()
            );
            println!("{}", style("─".repeat(80)).dim());

            if let Some(uid) = &session.user_id {
                println!("{} {}", style("User:       ").bold(), uid);
            }

            if let Some(title) = &session.title {
                println!("{} {}", style("Title:      ").bold(), title);
            }

            println!(
                "{} {}",
                style("Created:    ").bold(),
                session.created_at.format("%Y-%m-%d %H:%M:%S")
            );

            if let Some(last_msg) = &session.last_message_at {
                println!(
                    "{} {}",
                    style("Last message:").bold(),
                    last_msg.format("%Y-%m-%d %H:%M:%S")
                );
            }

            println!("\n{} {}", style("Messages:").bold(), messages.len());

            if !messages.is_empty() {
                println!("{}", style("─".repeat(80)).dim());

                for msg in messages {
                    let role_colored = match msg.role {
                        MessageRole::User => style("User").cyan(),
                        MessageRole::Assistant => style("Assistant").green(),
                        MessageRole::System => style("System").yellow(),
                        MessageRole::Tool => style("Tool").magenta(),
                    };

                    println!(
                        "\n[{}] {}",
                        style(msg.sequence_number).dim(),
                        role_colored.bold()
                    );

                    // Truncate long messages for display
                    let content = if msg.content.len() > 200 {
                        format!("{}...", &msg.content[..200])
                    } else {
                        msg.content.clone()
                    };

                    for line in content.lines() {
                        println!("  {}", line);
                    }
                }
            }

            println!();
        }

        SessionCommands::Message {
            session_id,
            role,
            content,
            db_path,
        } => {
            let db = Database::new(&db_path)?;

            // Parse role
            let role_enum = match role.to_lowercase().as_str() {
                "user" => MessageRole::User,
                "assistant" => MessageRole::Assistant,
                "system" => MessageRole::System,
                "tool" => MessageRole::Tool,
                _ => {
                    return Err(anyhow::anyhow!(
                        "Invalid role: {}. Must be one of: user, assistant, system, tool",
                        role
                    ));
                }
            };

            let message = db.add_message(&session_id, role_enum, &content, None)?;

            println!(
                "{} Added message to session {}",
                style("✓").green().bold(),
                style(&session_id).cyan()
            );
            println!("  {} {}", style("Message ID:").bold(), message.id);
            println!("  {} {}", style("Role:").bold(), message.role.as_str());
            println!("  {} {}", style("Sequence:").bold(), message.sequence_number);
        }

        SessionCommands::Compile {
            session_id,
            query,
            budget,
            db_path,
        } => {
            let db = Database::new(&db_path)?;
            let manager = SessionManager::new(db.clone());

            println!(
                "{} Compiling context for session {}",
                style("🥑").green(),
                style(&session_id).cyan()
            );
            println!("{} {}\n", style("Query:").bold(), query);

            // Build index
            let index = db.get_vector_index()?;

            // Compile in session context
            let config = CompilerConfig {
                token_budget: budget,
                ..Default::default()
            };

            let (message, working_set) = manager
                .add_user_message(&session_id, &query, config, &index, None)
                .await?;

            println!("{} Compilation complete", style("✓").green().bold());
            println!(
                "  {} {}",
                style("Message ID:").bold(),
                message.id
            );
            println!(
                "  {} {} / {}",
                style("Tokens:").bold(),
                style(working_set.tokens_used).cyan(),
                budget
            );
            println!(
                "  {} {}",
                style("Spans:").bold(),
                style(working_set.citations.len()).cyan()
            );
            println!(
                "  {} {}ms",
                style("Time:").bold(),
                style(working_set.compilation_time_ms).cyan()
            );

            // Show context preview
            println!("\n{}", style("Context Preview:").bold());
            println!("{}", style("─".repeat(80)).dim());
            let preview = working_set
                .text
                .lines()
                .take(10)
                .collect::<Vec<_>>()
                .join("\n");
            println!("{}", preview);
            if working_set.text.lines().count() > 10 {
                println!("...");
            }
            println!();
        }

        SessionCommands::History {
            session_id,
            max_tokens,
            db_path,
        } => {
            let db = Database::new(&db_path)?;
            let manager = SessionManager::new(db);

            let history = manager.get_conversation_history(&session_id, max_tokens)?;

            if history.is_empty() {
                println!("{} No messages in session", style("ℹ").blue());
                return Ok(());
            }

            println!(
                "\n{} {}",
                style("Conversation History").bold(),
                style(format!("(session: {})", session_id)).dim()
            );
            println!("{}", style("═".repeat(80)).dim());
            println!("{}", history);
            println!("{}", style("═".repeat(80)).dim());

            if let Some(max) = max_tokens {
                println!("\n{} Limited to ~{} tokens", style("ℹ").blue(), max);
            }
        }

        SessionCommands::Replay {
            session_id,
            db_path,
            json,
        } => {
            let db = Database::new(&db_path)?;
            let manager = SessionManager::new(db);

            let replay = manager.replay_session(&session_id)?;

            if json {
                println!("{}", serde_json::to_string_pretty(&replay)?);
                return Ok(());
            }

            // Pretty print replay
            println!(
                "\n{} {}",
                style("Session Replay").bold(),
                style(&replay.session.id).cyan()
            );
            println!("{}", style("═".repeat(80)).dim());

            if let Some(title) = &replay.session.title {
                println!("{} {}", style("Title:").bold(), title);
            }

            if let Some(uid) = &replay.session.user_id {
                println!("{} {}", style("User:").bold(), uid);
            }

            println!(
                "{} {}",
                style("Created:").bold(),
                replay.session.created_at.format("%Y-%m-%d %H:%M:%S")
            );
            println!("{} {}", style("Turns:").bold(), replay.turns.len());

            println!("\n{}", style("Conversation:").bold());
            println!("{}", style("─".repeat(80)).dim());

            for (i, turn) in replay.turns.iter().enumerate() {
                println!(
                    "\n{} {}",
                    style(format!("Turn {}", i + 1)).bold().cyan(),
                    style("─".repeat(70)).dim()
                );

                // User message
                println!(
                    "\n{} {}",
                    style("User:").bold().cyan(),
                    turn.user_message.content
                );

                // Working set
                if let Some(ws) = &turn.working_set {
                    println!(
                        "\n{} {} tokens, {} spans, {}ms",
                        style("Context:").dim(),
                        style(ws.tokens_used).yellow(),
                        style(ws.citations.len()).yellow(),
                        style(ws.compilation_time_ms).yellow()
                    );

                    // Show top citations
                    if !ws.citations.is_empty() {
                        println!("  {}", style("Top citations:").dim());
                        for citation in ws.citations.iter().take(3) {
                            println!(
                                "    {} {}:{}-{}",
                                style("•").dim(),
                                citation.artifact_path,
                                citation.start_line,
                                citation.end_line
                            );
                        }
                    }
                }

                // Assistant message
                if let Some(asst) = &turn.assistant_message {
                    println!(
                        "\n{} {}",
                        style("Assistant:").bold().green(),
                        asst.content
                    );
                }
            }

            println!("\n{}", style("═".repeat(80)).dim());
        }

        SessionCommands::Delete {
            session_id,
            yes,
            db_path,
        } => {
            if !yes {
                print!(
                    "Delete session {} and all messages? (y/N): ",
                    style(&session_id).cyan()
                );
                use std::io::{self, Write};
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;

                if !input.trim().eq_ignore_ascii_case("y") {
                    println!("Cancelled");
                    return Ok(());
                }
            }

            let db = Database::new(&db_path)?;
            db.delete_session(&session_id)?;

            println!(
                "{} Deleted session {}",
                style("✓").green().bold(),
                style(&session_id).cyan()
            );
        }
    }

    Ok(())
}
