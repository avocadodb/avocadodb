"""AvocadoDB tool for LangChain DeepAgents integration.

This module provides a tool function that can be used with DeepAgents
to enable deterministic, citation-backed context retrieval.
"""

import os
from typing import Any

from avocado.client import AvocadoDB


# Initialize AvocadoDB client
def get_avocadodb_client(url: str | None = None) -> AvocadoDB:
    """Get or create AvocadoDB client instance.

    Args:
        url: AvocadoDB server URL (default: http://localhost:8080)

    Returns:
        AvocadoDB client instance
    """
    server_url = url or os.environ.get("AVOCADODB_URL", "http://localhost:8080")
    return AvocadoDB(server_url)


def avocado_compile_context(
    query: str,
    token_budget: int = 8000,
    semantic_weight: float = 0.7,
    lexical_weight: float = 0.3,
    mmr_lambda: float = 0.5,
    enable_mmr: bool = True,
) -> dict[str, Any]:
    """Compile deterministic, citation-backed context from your knowledge base.

    AvocadoDB provides 100% deterministic context compilation - the same query
    always returns the same context, making your agent's responses reproducible
    and auditable. Every span includes exact line number citations.

    Use this tool when you need to:
    - Retrieve relevant context from ingested documentation
    - Get verifiable, citation-backed information
    - Ensure consistent responses across multiple runs
    - Access your codebase or knowledge base with perfect reproducibility

    Args:
        query: Search query describing what information you need
        token_budget: Maximum tokens to use (default: 8000)
        semantic_weight: Weight for semantic (vector) search 0.0-1.0 (default: 0.7)
        lexical_weight: Weight for lexical (keyword) search 0.0-1.0 (default: 0.3)
        mmr_lambda: Diversity parameter 0.0-1.0 (default: 0.5, higher = more diverse)
        enable_mmr: Enable Maximal Marginal Relevance diversification (default: True)

    Returns:
        Dictionary containing:
        - success: Whether the compilation succeeded
        - context: The compiled context text (use this in your response)
        - citations: List of citations with file paths and line numbers
        - spans: Number of spans included
        - tokens_used: Actual tokens used
        - compilation_time_ms: Compilation time in milliseconds
        - deterministic_hash: SHA-256 hash of context (same query = same hash)

    IMPORTANT: After using this tool:
    1. Read the 'context' field - this is the relevant information
    2. Use the context to answer the user's question naturally
    3. Cite sources using the 'citations' field when appropriate
    4. NEVER show raw JSON - synthesize the information into a clear response

    Example:
        result = avocado_compile_context("How does authentication work?")
        # result['context'] contains relevant spans about authentication
        # result['citations'] contains [{"file": "auth.md", "lines": "10-25"}, ...]
    """
    try:
        client = get_avocadodb_client()

        # Compile context using AvocadoDB
        working_set = client.compile(
            query=query,
            budget=token_budget,
            semantic_weight=semantic_weight,
            lexical_weight=lexical_weight,
            mmr_lambda=mmr_lambda,
            enable_mmr=enable_mmr,
        )

        # Format citations for easy reference
        formatted_citations = [
            {
                "file": citation.artifact_path,
                "lines": f"{citation.start_line}-{citation.end_line}",
            }
            for citation in working_set.citations
        ]

        return {
            "success": True,
            "context": working_set.text,
            "citations": formatted_citations,
            "spans": len(working_set.spans),
            "tokens_used": working_set.tokens_used,
            "compilation_time_ms": working_set.compilation_time_ms,
            "deterministic_hash": working_set.deterministic_hash(),
            "query": working_set.query,
        }

    except Exception as e:
        return {
            "success": False,
            "error": f"AvocadoDB error: {str(e)}",
            "context": "",
            "citations": [],
            "query": query,
            "hint": "Make sure AvocadoDB server is running (./target/release/avocado-server) "
                    "and documents are ingested (./target/release/avocado ingest docs/ --recursive)",
        }


# For convenience, export the function name that matches the pattern
compile_context = avocado_compile_context


__all__ = ["avocado_compile_context", "compile_context", "get_avocadodb_client"]
