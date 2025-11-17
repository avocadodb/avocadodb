"""AutoGen integration for AvocadoDB.

Provides avocado_compile_context as an AutoGen-compatible tool function.

Example:
    >>> from autogen import AssistantAgent, UserProxyAgent
    >>> from avocado.integrations.autogen import avocado_compile_context
    >>>
    >>> assistant = AssistantAgent(
    ...     name="assistant",
    ...     llm_config={"model": "gpt-4"},
    ...     functions=[avocado_compile_context]
    ... )
    >>>
    >>> user_proxy = UserProxyAgent(name="user")
    >>> user_proxy.initiate_chat(assistant, message="How does authentication work?")
"""

import os
from typing import Any, Annotated

from avocado.client import AvocadoDB
from avocado.manager import get_manager


def avocado_compile_context(
    query: Annotated[str, "Search query describing what information you need"],
    token_budget: Annotated[int, "Maximum tokens to use (default: 8000)"] = 8000,
    semantic_weight: Annotated[float, "Weight for semantic search 0.0-1.0 (default: 0.7)"] = 0.7,
    lexical_weight: Annotated[float, "Weight for lexical search 0.0-1.0 (default: 0.3)"] = 0.3,
    mmr_lambda: Annotated[float, "Diversity parameter 0.0-1.0 (default: 0.5)"] = 0.5,
    enable_mmr: Annotated[bool, "Enable diversification (default: True)"] = True,
) -> dict[str, Any]:
    """Compile deterministic, citation-backed context from your knowledge base (AutoGen tool).

    AvocadoDB provides 100% deterministic context compilation - the same query
    always returns the same context, making responses reproducible and auditable.

    Use this tool when you need to:
    - Retrieve relevant context from ingested documentation
    - Get verifiable, citation-backed information
    - Ensure consistent responses across multiple runs
    - Access your codebase or knowledge base with perfect reproducibility

    Args:
        query: Search query describing what information you need (be specific)
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
    """
    # Auto-start server if configured
    manager = get_manager()
    manager.ensure_running()

    # Auto-ingest if needed (first-time setup)
    stats = manager.get_stats()
    if stats.get("artifacts_count", 0) == 0:
        print("🥑 First-time setup: Auto-ingesting current directory...")
        from avocado.ingest import AutoIngest
        ingester = AutoIngest()
        ingester.ingest_project(".", max_files=100)

    try:
        # Get server URL from environment or use default
        server_url = os.environ.get("AVOCADODB_URL", "http://localhost:8765")

        # Use AvocadoDB client
        client = AvocadoDB(server_url)
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

        # Show token usage stats
        print(f"📊 AvocadoDB: {working_set.tokens_used:,} tokens used (budget: {token_budget:,}) | {len(working_set.spans)} spans | {working_set.compilation_time_ms}ms")

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
            "hint": "Make sure AvocadoDB server is running and documents are ingested",
        }


__all__ = ["avocado_compile_context"]
