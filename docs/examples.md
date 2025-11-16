# Real-World Usage Examples

This guide shows how to use AvocadoDB effectively in production scenarios.

## Table of Contents

- [Example 1: Code Documentation Assistant](#example-1-code-documentation-assistant)
- [Example 2: Technical Q&A System](#example-2-technical-qa-system)
- [Example 3: Chatbot with Context](#example-3-chatbot-with-context)
- [Example 4: Multi-Repository Search](#example-4-multi-repository-search)
- [Example 5: Automated Code Review](#example-5-automated-code-review)
- [Example 6: Research Assistant](#example-6-research-assistant)

---

## Example 1: Code Documentation Assistant

**Use Case:** Help developers understand your codebase by answering questions with precise citations.

### Setup

```bash
# Initialize database
avocado init

# Ingest entire codebase
avocado ingest ./src --recursive
avocado ingest ./docs --recursive
avocado ingest README.md

# Check what was indexed
avocado stats
```

### Usage

```bash
# Ask specific questions
avocado compile "How does authentication work?" --budget 8000

# Find implementation patterns
avocado compile "error handling best practices" --budget 12000

# Locate specific features
avocado compile "where is the user registration logic" --budget 6000
```

### Integration Example (Rust)

```rust
use avocado_core::{Database, VectorIndex, compiler, types::CompilerConfig};

async fn answer_code_question(question: &str) -> String {
    let db = Database::new(".avocado/db.sqlite").unwrap();
    let index = VectorIndex::from_database(&db).unwrap();

    let config = CompilerConfig {
        token_budget: 8000,
        semantic_weight: 0.7,
        lexical_weight: 0.3,
        mmr_lambda: 0.5,
        enable_mmr: true,
    };

    let context = compiler::compile(question, config, &db, &index, None)
        .await
        .unwrap();

    // Send to LLM
    let prompt = format!(
        "Given this code context:\n\n{}\n\nQuestion: {}\n\nAnswer:",
        context.text, question
    );

    // Return prompt for LLM (or call LLM API here)
    prompt
}
```

---

## Example 2: Technical Q&A System

**Use Case:** Build a support system that answers technical questions with verifiable citations.

### Setup

```bash
# Ingest documentation
avocado ingest ./docs --recursive
avocado ingest ./api-reference --recursive
avocado ingest ./troubleshooting --recursive
```

### CLI Usage

```bash
# Support queries
avocado compile "How do I configure rate limiting?" --budget 8000

# Troubleshooting
avocado compile "authentication error 401" --budget 6000 \
  --lexical-weight 0.5  # Higher keyword matching for error codes

# API reference
avocado compile "POST /users endpoint" --budget 4000
```

### Integration Example (Python - if SDK exists)

```python
from avocado import AvocadoDB
import openai

db = AvocadoDB(db_path=".avocado/db.sqlite")

def answer_support_question(question: str) -> dict:
    # Compile context
    result = db.compile(
        query=question,
        budget=8000,
        semantic_weight=0.7,
        lexical_weight=0.3
    )

    # Build prompt with context
    prompt = f"""You are a helpful support assistant.

Context from documentation:
{result.text}

Question: {question}

Provide a helpful answer based on the context above.
Include citation numbers [1], [2], etc. when referencing specific information.
"""

    # Call LLM
    response = openai.ChatCompletion.create(
        model="gpt-4",
        messages=[{"role": "user", "content": prompt}]
    )

    return {
        "answer": response.choices[0].message.content,
        "citations": result.citations,
        "deterministic_hash": result.deterministic_hash
    }
```

---

## Example 3: Chatbot with Context

**Use Case:** Build a chatbot that maintains conversation history and retrieves relevant context.

### Approach

```rust
use avocado_core::{Database, VectorIndex, compiler, types::CompilerConfig};

struct Chatbot {
    db: Database,
    index: VectorIndex,
    conversation_history: Vec<(String, String)>,  // (user, assistant) pairs
}

impl Chatbot {
    async fn respond(&mut self, user_message: &str) -> String {
        // Compile context based on current message
        let config = CompilerConfig {
            token_budget: 6000,  // Reserve tokens for conversation history
            semantic_weight: 0.7,
            lexical_weight: 0.3,
            mmr_lambda: 0.5,
            enable_mmr: true,
        };

        let context = compiler::compile(
            user_message,
            config,
            &self.db,
            &self.index,
            None
        ).await.unwrap();

        // Build prompt with context + history
        let mut prompt = format!("Context:\n{}\n\n", context.text);

        // Add conversation history
        prompt.push_str("Conversation:\n");
        for (user, assistant) in &self.conversation_history {
            prompt.push_str(&format!("User: {}\nAssistant: {}\n", user, assistant));
        }

        prompt.push_str(&format!("User: {}\nAssistant:", user_message));

        // Call LLM (pseudo-code)
        let response = call_llm(&prompt).await;

        // Store in history
        self.conversation_history.push((
            user_message.to_string(),
            response.clone()
        ));

        response
    }
}
```

**Key Insight:** AvocadoDB handles the knowledge retrieval deterministically, while your application manages the conversation state.

---

## Example 4: Multi-Repository Search

**Use Case:** Search across multiple codebases or documentation sets.

### Setup

```bash
# Initialize one database per repository
mkdir -p .avocado/repos

# Ingest repo A
AVOCADO_DB_PATH=.avocado/repos/backend.sqlite avocado init
AVOCADO_DB_PATH=.avocado/repos/backend.sqlite avocado ingest ../backend --recursive

# Ingest repo B
AVOCADO_DB_PATH=.avocado/repos/frontend.sqlite avocado init
AVOCADO_DB_PATH=.avocado/repos/frontend.sqlite avocado ingest ../frontend --recursive

# Ingest repo C
AVOCADO_DB_PATH=.avocado/repos/docs.sqlite avocado init
AVOCADO_DB_PATH=.avocado/repos/docs.sqlite avocado ingest ../docs --recursive
```

### Multi-Index Query

```rust
use avocado_core::{Database, VectorIndex, compiler, types::*};

async fn search_across_repos(query: &str, budget: usize) -> Vec<WorkingSet> {
    let repos = vec![
        ".avocado/repos/backend.sqlite",
        ".avocado/repos/frontend.sqlite",
        ".avocado/repos/docs.sqlite",
    ];

    let mut all_results = Vec::new();

    for repo_path in repos {
        let db = Database::new(repo_path).unwrap();
        let index = VectorIndex::from_database(&db).unwrap();

        let config = CompilerConfig {
            token_budget: budget / repos.len(),  // Split budget
            semantic_weight: 0.7,
            lexical_weight: 0.3,
            mmr_lambda: 0.5,
            enable_mmr: true,
        };

        let result = compiler::compile(query, config, &db, &index, None)
            .await
            .unwrap();

        all_results.push(result);
    }

    all_results
}
```

**Alternative:** Merge all repos into one database for unified search.

---

## Example 5: Automated Code Review

**Use Case:** Provide AI-powered code review with relevant context from your codebase standards.

### Setup

```bash
# Ingest coding standards and best practices
avocado ingest ./docs/coding-standards --recursive
avocado ingest ./docs/architecture --recursive
avocado ingest ./docs/security-guidelines --recursive

# Ingest example code for patterns
avocado ingest ./src/examples --recursive
```

### Review Script

```bash
#!/bin/bash
# review-pr.sh - Review a pull request

PR_DIFF=$1

# Extract what changed
SUMMARY=$(echo "$PR_DIFF" | head -50)

# Compile relevant standards
CONTEXT=$(avocado compile "coding standards security best practices" \
  --budget 12000 \
  --mmr-lambda 0.6)  # Higher relevance

# Build review prompt
cat > prompt.txt << EOF
You are a code reviewer. Review this pull request against our coding standards.

Coding Standards:
$CONTEXT

Pull Request Changes:
$PR_DIFF

Provide feedback on:
1. Security concerns
2. Code quality issues
3. Adherence to standards (cite specific standards using [1], [2], etc.)
4. Suggestions for improvement
EOF

# Send to LLM for review
# (use your preferred LLM API)
```

---

## Example 6: Research Assistant

**Use Case:** Help researchers find relevant papers, quotes, and citations.

### Setup

```bash
# Ingest research papers
avocado ingest ./papers --recursive

# Ingest notes and annotations
avocado ingest ./research-notes --recursive

# Ingest book chapters
avocado ingest ./books --recursive
```

### Finding Relevant Research

```bash
# Find papers on a specific topic
avocado compile "neural architecture search methods" \
  --budget 16000 \
  --mmr-lambda 0.4  # Prioritize diversity for research

# Find methodology descriptions
avocado compile "experimental design randomized controlled trial" \
  --budget 12000 \
  --semantic-weight 0.8  # Higher semantic matching

# Find specific citations
avocado compile "Smith et al 2023 deep learning" \
  --budget 8000 \
  --lexical-weight 0.6  # Higher keyword matching for citation searches
```

### Integration Example

```rust
async fn research_query(query: &str) -> ResearchResult {
    let db = Database::new(".avocado/research.sqlite").unwrap();
    let index = VectorIndex::from_database(&db).unwrap();

    let config = CompilerConfig {
        token_budget: 16000,  // Large budget for comprehensive research
        semantic_weight: 0.7,
        lexical_weight: 0.3,
        mmr_lambda: 0.3,  // High diversity to avoid redundant papers
        enable_mmr: true,
    };

    let context = compiler::compile(query, config, &db, &index, None)
        .await
        .unwrap();

    ResearchResult {
        relevant_passages: context.text,
        citations: context.citations,
        papers_referenced: extract_papers(&context),
        hash: context.deterministic_hash,
    }
}

fn extract_papers(context: &WorkingSet) -> Vec<String> {
    // Extract unique paper citations from the context
    context.citations
        .iter()
        .map(|c| c.artifact_path.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect()
}
```

---

## Best Practices

### 1. Right-Size Your Token Budget

```bash
# Quick lookups
avocado compile "query" --budget 2000

# Standard Q&A
avocado compile "query" --budget 8000

# Comprehensive research
avocado compile "query" --budget 16000

# Maximum context (Claude 3)
avocado compile "query" --budget 100000
```

### 2. Tune MMR Lambda for Your Use Case

```bash
# High diversity (research, exploration)
--mmr-lambda 0.3

# Balanced (general Q&A)
--mmr-lambda 0.5

# High relevance (specific lookups)
--mmr-lambda 0.8
```

### 3. Balance Semantic vs Lexical Search

```bash
# Conceptual queries (use semantic)
--semantic-weight 0.8 --lexical-weight 0.2

# Balanced (default)
--semantic-weight 0.7 --lexical-weight 0.3

# Keyword/code searches (use lexical)
--semantic-weight 0.5 --lexical-weight 0.5
```

### 4. Cache Compiled Contexts

If you're using the same query repeatedly:

```rust
use std::collections::HashMap;

struct ContextCache {
    cache: HashMap<String, WorkingSet>,
}

impl ContextCache {
    async fn get_or_compile(&mut self, query: &str) -> &WorkingSet {
        if !self.cache.contains_key(query) {
            let result = compile_context(query).await;
            self.cache.insert(query.to_string(), result);
        }
        self.cache.get(query).unwrap()
    }
}
```

### 5. Verify Determinism in Production

```rust
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_query_determinism() {
        let query = "authentication";

        let result1 = compile_context(query).await;
        let result2 = compile_context(query).await;

        assert_eq!(
            result1.deterministic_hash(),
            result2.deterministic_hash(),
            "Compilation must be deterministic"
        );
    }
}
```

---

## Performance Optimization Tips

### 1. Batch Ingestion

```bash
# Bad: One file at a time
for file in *.md; do
  avocado ingest "$file"
done

# Good: Whole directory
avocado ingest . --recursive
```

### 2. Index Organization

```bash
# Separate frequently-updated content
AVOCADO_DB_PATH=.avocado/stable.sqlite avocado ingest ./docs
AVOCADO_DB_PATH=.avocado/dynamic.sqlite avocado ingest ./api-logs
```

### 3. Monitor Performance

```bash
# Enable debug logging to see timing breakdown
RUST_LOG=avocado_core=debug avocado compile "query" --budget 8000
```

Look for bottlenecks:
- **Embed query > 500ms**: OpenAI API is slow (normal during high load)
- **Search > 50ms**: Index might be too large (consider Phase 2 HNSW)
- **Token counting > 50ms**: Check tiktoken initialization

---

## Common Patterns

### Pattern 1: Context + Examples

```rust
// Compile context for the concept
let concept_context = compile("authentication best practices").await;

// Compile examples separately
let examples_context = compile("authentication code examples").await;

// Combine for LLM
let prompt = format!(
    "Concepts:\n{}\n\nExamples:\n{}\n\nQuestion: {}",
    concept_context.text,
    examples_context.text,
    user_question
);
```

### Pattern 2: Progressive Context Building

```rust
// Start with narrow query
let context = compile("JWT token validation", 4000).await;

// If LLM needs more context, expand
if needs_more_context(&llm_response) {
    let expanded = compile("authentication security patterns", 12000).await;
    // Re-query LLM with expanded context
}
```

### Pattern 3: Multi-Query Fusion

```rust
// Compile context from multiple perspectives
let contexts = vec![
    compile("authentication security").await,
    compile("user session management").await,
    compile("token refresh patterns").await,
];

// Merge and deduplicate
let merged_context = merge_contexts(contexts);
```

---

## Troubleshooting

### Low Token Utilization (<80%)

This usually means your query doesn't have enough relevant content:

```bash
# Check how many spans exist
avocado stats

# Try a broader query
avocado compile "broader search terms" --budget 8000
```

### Redundant Results

Increase diversity:

```bash
avocado compile "query" --mmr-lambda 0.3
```

### Missing Relevant Content

Try adjusting search weights:

```bash
# More semantic matching
avocado compile "query" --semantic-weight 0.8

# More keyword matching
avocado compile "query" --lexical-weight 0.5
```

---

For more information, see:
- [README.md](README.md) - Full documentation
- [QUICKSTART.md](QUICKSTART.md) - Getting started guide
- [docs/performance.md](docs/performance.md) - Performance tuning
