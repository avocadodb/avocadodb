"""LangChain / DeepAgents integration for AvocadoDB.

Provides:
- AvocadoDBTool: LangChain tool wrapper with auto-start
- AvocadoDBMiddleware: Blocks sequential read tools after AvocadoDB queries
- avocado_compile_context: Function-based tool for compatibility

Example:
    >>> from avocado.integrations.langchain import AvocadoDBTool, AvocadoDBMiddleware
    >>> from deepagents import create_deep_agent
    >>>
    >>> agent = create_deep_agent(
    ...     model="claude-3-5-sonnet-20241022",
    ...     tools=[AvocadoDBTool()],
    ...     middleware=[AvocadoDBMiddleware()]
    ... )
"""

import os
from typing import Any, Optional

from avocado.client import AvocadoDB
from avocado.manager import get_manager


def avocado_compile_context(
    query: str,
    token_budget: int = 8000,
    semantic_weight: float = 0.7,
    lexical_weight: float = 0.3,
    mmr_lambda: float = 0.5,
    enable_mmr: bool = True,
) -> dict[str, Any]:
    """PRIMARY TOOL: Use this FIRST for any questions about the codebase or documentation.

    AvocadoDB provides deterministic, citation-backed context compilation - the same query
    always returns the same context, making responses reproducible and auditable.

    **WHEN TO USE (DEFAULT for codebase questions):**
    - ANY question about the codebase, documentation, or project
    - Questions like "what is this project", "how does X work", "explain Y"
    - Searching for implementations, patterns, or architecture
    - Understanding features, APIs, or configurations

    **PREFER THIS OVER grep/read_file** - it provides semantic search with citations.
    Only use filesystem tools if this fails or for editing files.

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

    Setup:
        With auto-start enabled (default), just run avacado-cli!

        Or manually:
        1. Start AvocadoDB server: ./target/release/avocado-server (port 8765)
        2. Ingest documents: ./target/release/avocado ingest ./docs --recursive
        3. Set AVOCADODB_URL (optional): export AVOCADODB_URL="http://localhost:8765"

    Example Response:
        "The authentication system uses JWT tokens (see auth.md:10-25). The token
        validation happens in the middleware layer (src/auth.ts:45-78)..."
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
            "hint": (
                "💡 Want deterministic context retrieval? Install AvocadoDB:\n\n"
                "   Quick Install (copy-paste):\n"
                "   curl -fsSL https://raw.githubusercontent.com/avocadodb/avocadodb/main/install.sh | sh\n\n"
                "   Or manual install:\n"
                "   git clone https://github.com/avocadodb/avocadodb && cd avocadodb\n"
                "   cargo build --release && ./target/release/avocado-server &\n\n"
                "   Benefits: 100% deterministic, citation-backed, 95% token efficiency\n"
                "   Docs: https://github.com/avocadodb/avocadodb"
            ),
        }


# Middleware for blocking read tools after AvocadoDB queries
class AvocadoDBMiddleware:
    """LangChain middleware that enforces AvocadoDB-only execution for codebase queries.

    When avocado_compile_context is called, blocks these tools:
    - read_file
    - grep
    - ls
    - glob

    This prevents the agent from calling multiple tools in parallel or sequentially,
    ensuring AvocadoDB is used exclusively for codebase questions.

    Example:
        >>> from avocado.integrations.langchain import AvocadoDBMiddleware
        >>> from deepagents import create_deep_agent
        >>>
        >>> agent = create_deep_agent(
        ...     middleware=[AvocadoDBMiddleware()]
        ... )
    """

    def __init__(self):
        """Initialize middleware."""
        # Tools to block when avocado_compile_context is active
        self.blocked_tools = {"read_file", "grep", "ls", "glob"}

    def after_model(
        self,
        response: dict | Any,
        runtime: Any
    ) -> dict | Any | None:
        """Filter tool calls after model returns but before execution.

        This is the correct lifecycle hook - it runs after the LLM generates
        tool calls but before they are executed, allowing us to filter them.

        Args:
            response: The model's response containing tool calls (can be dict or ModelResponse)
            runtime: The runtime context

        Returns:
            Modified response with filtered tool calls, or None to keep original
        """
        # Handle both dict and ModelResponse object formats
        if isinstance(response, dict):
            # LangGraph state uses "messages" key, not "result"
            result_messages = response.get("messages", []) or response.get("result", [])
        else:
            result_messages = getattr(response, "result", [])

        if not result_messages:
            return None

        # Check the last message for tool calls
        last_message = result_messages[-1] if result_messages else None

        if not last_message or not hasattr(last_message, "tool_calls"):
            return None

        tool_calls = last_message.tool_calls
        if not tool_calls:
            return None

        # Check if avocado_compile_context is being called NOW (parallel case)
        has_avocado = any(
            tc.get("name") == "avocado_compile_context"
            for tc in tool_calls
        )

        # Also check if avocado was called RECENTLY (sequential case)
        # Look back through recent messages to see if avocado was used
        avocado_used_recently = False
        for msg in result_messages[-5:]:  # Check last 5 messages
            if hasattr(msg, "tool_calls") and msg.tool_calls:
                for tc in msg.tool_calls:
                    if tc.get("name") == "avocado_compile_context":
                        avocado_used_recently = True
                        break

        if not has_avocado and not avocado_used_recently:
            return None

        # Filter out blocked tools when AvocadoDB is present OR was recently used
        if has_avocado:
            # Parallel case: Keep avocado and non-blocked tools
            filtered_calls = [
                tc for tc in tool_calls
                if tc.get("name") == "avocado_compile_context"
                or tc.get("name") not in self.blocked_tools
            ]
        elif avocado_used_recently:
            # Sequential case: Block ALL read tools since avocado was just used
            filtered_calls = [
                tc for tc in tool_calls
                if tc.get("name") not in self.blocked_tools
            ]
        else:
            # Should not reach here
            return None

        # If we filtered anything, update the message and log it
        if len(filtered_calls) < len(tool_calls):
            blocked = [
                tc.get("name")
                for tc in tool_calls
                if tc.get("name") in self.blocked_tools
            ]
            if blocked:
                print(f"🥑 AvocadoDB exclusivity: Blocked {', '.join(blocked)}")

            # Create a modified message with filtered tool calls
            last_message.tool_calls = filtered_calls

            # Return modified response (in the same format it came in)
            return response

        return None

    async def aafter_model(
        self,
        response: dict | Any,
        runtime: Any
    ) -> dict | Any | None:
        """Async version of after_model - filter tool calls after model returns.

        Args:
            response: The model's response containing tool calls (can be dict or ModelResponse)
            runtime: The runtime context

        Returns:
            Modified response with filtered tool calls, or None to keep original
        """
        # Delegate to sync version since there's no async work needed
        return self.after_model(response, runtime)


# Export names that match LangChain conventions
AvocadoDBTool = avocado_compile_context  # Alias for compatibility

__all__ = ["avocado_compile_context", "AvocadoDBTool", "AvocadoDBMiddleware"]
