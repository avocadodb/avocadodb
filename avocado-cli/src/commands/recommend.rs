//! Model recommendation command implementation
//!
//! Helps users choose the optimal embedding model for their use case.

use anyhow::Result;
use console::style;

/// Model recommendation with rationale
pub struct ModelRecommendation {
    pub model: String,
    pub dimensions: usize,
    pub reasoning: Vec<String>,
    pub env_var_command: String,
}

/// Recommend optimal embedding model based on use case and corpus size
pub fn recommend_model(
    corpus_size: Option<usize>,
    use_case: Option<&str>,
) -> Result<ModelRecommendation> {
    println!(
        "\n{}",
        style("🥑 AvocadoDB Model Recommendation").bold().green()
    );
    println!("{}", style("─".repeat(60)).dim());

    // Determine recommendation based on inputs
    let recommendation = match (corpus_size, use_case) {
        // Small corpus (<1K documents)
        (Some(size), _) if size < 1000 => ModelRecommendation {
            model: "all-MiniLM-L6-v2 (default)".to_string(),
            dimensions: 384,
            reasoning: vec![
                "Small corpus prioritizes speed over maximum accuracy".to_string(),
                "384 dimensions sufficient for <1K documents".to_string(),
                "Fastest inference time (~1-3ms per embedding)".to_string(),
            ],
            env_var_command: "(no configuration needed - this is the default)".to_string(),
        },

        // Large corpus + production use case
        (Some(size), Some("production")) if size > 10000 => ModelRecommendation {
            model: "nomic-embed-text-v1.5".to_string(),
            dimensions: 768,
            reasoning: vec![
                "Large corpus benefits from higher dimensionality".to_string(),
                "768 dimensions provide better accuracy for complex queries".to_string(),
                "Production use case justifies slightly slower inference (~2-5ms)".to_string(),
                "Good balance between quality and performance".to_string(),
            ],
            env_var_command: "export AVOCADODB_EMBEDDING_MODEL=nomicv15".to_string(),
        },

        // Legal/compliance use case
        (_, Some("legal")) | (_, Some("compliance")) => ModelRecommendation {
            model: "bge-large-en-v1.5".to_string(),
            dimensions: 1024,
            reasoning: vec![
                "Maximum accuracy critical for legal/compliance".to_string(),
                "1024 dimensions provide highest quality embeddings".to_string(),
                "Worth the slower inference for high-stakes use cases".to_string(),
            ],
            env_var_command: "export AVOCADODB_EMBEDDING_MODEL=bgelarge".to_string(),
        },

        // Code search (general case)
        (_, Some("code") | Some("code-search")) => ModelRecommendation {
            model: "all-MiniLM-L6-v2 (default)".to_string(),
            dimensions: 384,
            reasoning: vec![
                "Code search benefits from fast iteration cycles".to_string(),
                "384 dimensions handle code structure well".to_string(),
                "Speed matters more than marginal accuracy gains".to_string(),
            ],
            env_var_command: "(no configuration needed - this is the default)".to_string(),
        },

        // Medium corpus (1K-10K documents)
        (Some(size), _) if size >= 1000 && size <= 10000 => ModelRecommendation {
            model: "nomic-embed-text-v1.5".to_string(),
            dimensions: 768,
            reasoning: vec![
                "Medium corpus benefits from balanced approach".to_string(),
                "768 dimensions improve accuracy without major speed loss".to_string(),
                "Good for most production applications".to_string(),
            ],
            env_var_command: "export AVOCADODB_EMBEDDING_MODEL=nomicv15".to_string(),
        },

        // Very large corpus (>10K documents)
        (Some(size), _) if size > 10000 => ModelRecommendation {
            model: "all-MiniLM-L6-v2 (default)".to_string(),
            dimensions: 384,
            reasoning: vec![
                "Large corpus requires many embeddings".to_string(),
                "Speed matters more at scale".to_string(),
                "Consider server mode to keep index in memory".to_string(),
            ],
            env_var_command: "(no configuration needed - this is the default)".to_string(),
        },

        // Default recommendation
        _ => ModelRecommendation {
            model: "all-MiniLM-L6-v2 (default)".to_string(),
            dimensions: 384,
            reasoning: vec![
                "Default model provides excellent speed/quality balance".to_string(),
                "Works well for most use cases".to_string(),
                "Upgrade to higher dimensions if accuracy is critical".to_string(),
            ],
            env_var_command: "(no configuration needed - this is the default)".to_string(),
        },
    };

    // Print recommendation
    print_recommendation(&recommendation, corpus_size, use_case);

    Ok(recommendation)
}

/// Print the recommendation in a user-friendly format
fn print_recommendation(
    rec: &ModelRecommendation,
    corpus_size: Option<usize>,
    use_case: Option<&str>,
) {
    // Print inputs
    println!("\n{}", style("Your Configuration:").bold());
    if let Some(size) = corpus_size {
        println!(
            "  Corpus size: {} documents",
            style(format!("{}", size)).cyan()
        );
    }
    if let Some(case) = use_case {
        println!("  Use case: {}", style(case).cyan());
    }

    // Print recommendation
    println!("\n{}", style("Recommended Model:").bold().green());
    println!(
        "  {} {} ({} dimensions)",
        style("✓").green(),
        style(&rec.model).cyan().bold(),
        style(rec.dimensions).dim()
    );

    // Print reasoning
    println!("\n{}", style("Why this model:").bold());
    for (i, reason) in rec.reasoning.iter().enumerate() {
        println!("  {}. {}", i + 1, reason);
    }

    // Print configuration
    println!("\n{}", style("To use this model:").bold());
    println!("  {}", style(&rec.env_var_command).yellow());
    if !rec.env_var_command.contains("no configuration") {
        println!("\n  Then re-ingest your documents:");
        println!(
            "  {}",
            style("avocado clear && avocado ingest <path> --recursive").dim()
        );
    }

    // Print comparison table
    println!("\n{}", style("All Available Models:").bold());
    println!("{}", style("─".repeat(60)).dim());
    print_model_table();

    println!();
}

/// Print table of all available models
fn print_model_table() {
    let models = vec![
        ("all-MiniLM-L6-v2", 384, "Fastest", "Good", "Default"),
        ("nomic-embed-text-v1.5", 768, "Medium", "Better", "nomicv15"),
        ("bge-large-en-v1.5", 1024, "Slower", "Best", "bgelarge"),
    ];

    println!(
        "\n  {:<25} {:<6} {:<10} {:<8} {:<10}",
        style("Model").bold(),
        style("Dims").bold(),
        style("Speed").bold(),
        style("Quality").bold(),
        style("Alias").bold()
    );
    println!("  {}", style("─".repeat(70)).dim());

    for (model, dims, speed, quality, alias) in models {
        println!(
            "  {:<25} {:<6} {:<10} {:<8} {:<10}",
            model,
            dims,
            speed,
            quality,
            style(alias).dim()
        );
    }
}
