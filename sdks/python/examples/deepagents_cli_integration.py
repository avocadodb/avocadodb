"""AvocadoDB integration for DeepAgents CLI.

This file can be copied to:
    deepagents-cli/deepagents_cli/integrations/avocadodb.py

Then import in tools.py:
    from deepagents_cli.integrations.avocadodb import avocado_compile_context
"""

import os
from typing import Any


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

    This tool searches your ingested codebase/documentation and returns relevant
    context that you MUST synthesize into a natural response for the user.

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
    1. Read through the 'context' field - this contains the relevant information
    2. Extract what's needed to answer the user's question
    3. Synthesize this into a clear, natural language response
    4. Cite sources by mentioning file names and line numbers from 'citations'
    5. NEVER show the raw JSON to the user - always provide a formatted response

    Example:
        If context contains authentication docs from auth.md:10-25,
        respond like: "The authentication system uses JWT tokens (see auth.md:10-25)..."
    """
    try:
        import requests

        # Get server URL from environment or use default
        server_url = os.environ.get("AVOCADODB_URL", "http://localhost:8080")

        # Call AvocadoDB compile endpoint
        response = requests.post(
            f"{server_url}/compile",
            json={
                "query": query,
                "token_budget": token_budget,
                "semantic_weight": semantic_weight,
                "lexical_weight": lexical_weight,
                "mmr_lambda": mmr_lambda,
                "enable_mmr": enable_mmr,
            },
            timeout=30,
        )

        if response.status_code != 200:
            return {
                "success": False,
                "error": f"AvocadoDB API error: {response.status_code}",
                "context": "",
                "citations": [],
                "query": query,
                "hint": "Check if AvocadoDB server is running: ./target/release/avocado-server",
            }

        data = response.json()

        # Format citations for easy reference
        formatted_citations = [
            {
                "file": citation["artifact_path"],
                "lines": f"{citation['start_line']}-{citation['end_line']}",
            }
            for citation in data.get("citations", [])
        ]

        # Calculate deterministic hash
        import hashlib
        context_text = data.get("text", "")
        det_hash = hashlib.sha256(context_text.encode("utf-8")).hexdigest()

        return {
            "success": True,
            "context": context_text,
            "citations": formatted_citations,
            "spans": len(data.get("spans", [])),
            "tokens_used": data.get("tokens_used", 0),
            "compilation_time_ms": data.get("compilation_time_ms", 0),
            "deterministic_hash": det_hash,
            "query": query,
        }

    except requests.exceptions.ConnectionError:
        return {
            "success": False,
            "error": "Cannot connect to AvocadoDB server",
            "context": "",
            "citations": [],
            "query": query,
            "hint": (
                "Make sure AvocadoDB server is running:\n"
                "  1. Start server: ./target/release/avocado-server\n"
                "  2. Ingest docs: ./target/release/avocado ingest docs/ --recursive\n"
                "  3. Set AVOCADODB_URL if not using default (http://localhost:8080)"
            ),
        }
    except Exception as e:
        return {
            "success": False,
            "error": f"AvocadoDB error: {str(e)}",
            "context": "",
            "citations": [],
            "query": query,
        }


__all__ = ["avocado_compile_context"]
