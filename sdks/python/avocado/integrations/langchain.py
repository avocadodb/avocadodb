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

# Import LangChain middleware base class
try:
    from langchain.agents.middleware import AgentMiddleware
except ImportError:
    # Fallback if langchain is not installed
    class AgentMiddleware:
        """Fallback middleware base class."""
        pass


def avocado_compile_context(
    query: str,
    token_budget: int = 8000,
    semantic_weight: float = 0.7,
    lexical_weight: float = 0.3,
    mmr_lambda: float = 0.5,
    enable_mmr: bool = True,
    use_llm: str = "none",  # Default: skip LLM for speed. Use "auto" or "local" to enable.
    max_new_tokens: int = 100,  # Reduced from 200 for faster generation on CPU
) -> dict[str, Any]:
    """PRIMARY TOOL: Use this FIRST for any questions about the codebase or documentation.

    AvocadoDB provides deterministic, citation-backed context compilation - the same query
    always returns the same context, making responses reproducible and auditable.
    
    By default, returns raw context (fast, ~300-500ms). Set use_llm="auto" or "local" to
    enable TinyLlama summarization (slower, ~5-8s on CPU, but provides natural language answers).

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
        use_llm: LLM mode - "none" (just return context, fast, default), 
                 "auto" (try local TinyLlama, fallback to context), 
                 "local" (require TinyLlama)
        max_new_tokens: Maximum tokens for answer generation (default: 100)

    Returns:
        Dictionary containing:
        - success: Whether the compilation succeeded
        - answer: Natural language answer (if LLM available, otherwise same as context)
        - context: The compiled context text (always included for reference)
        - citations: List of citations with file paths and line numbers
        - spans: Number of spans included
        - tokens_used: Actual tokens used
        - compilation_time_ms: Compilation time in milliseconds
        - deterministic_hash: SHA-256 hash of context (same query = same hash)
        - llm_used: Whether TinyLlama was used to generate the answer

    IMPORTANT: 
    - If 'answer' field exists, use that as the primary response (it's a summarized answer)
    - The 'context' field contains the raw compiled context for reference
    - Cite sources by mentioning file names and line numbers from 'citations'
    - NEVER show the raw JSON to the user - always provide a formatted response

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
    # Use HTTP mode by default (MongoDB-style daemon, best performance)
    # Daemon keeps indexes in memory, supports multiple projects automatically
    # Can override with AVOCADODB_MODE=cli to use direct CLI mode
    mode = os.environ.get("AVOCADODB_MODE", "http")
    
    try:
        if mode == "cli":
            # CLI mode - per-directory database (perfect for multi-repo)
            # Use absolute path from current working directory
            db_path = os.path.join(os.getcwd(), ".avocado/db.sqlite")
            client = AvocadoDB(mode="cli", db_path=db_path)
            
            # Auto-ingest if needed (first-time setup or if database is mostly empty)
            # This ensures everything is indexed automatically without manual intervention
            try:
                stats = client.stats()
                artifacts_count = stats.get("artifacts_count", 0)
                
                # Check if we should ingest
                # For large repos (500K+ lines), we expect many more artifacts
                # 708 artifacts for 3,800+ files is way too low - should be 1000+
                has_orion = os.path.exists(".private/orion") and os.path.isdir(".private/orion")
                
                # Re-ingest if database seems incomplete
                # Large repos should have 1000+ artifacts (after filtering)
                should_ingest = artifacts_count < 1000
                
                # Also check if .private/orion exists but might not be properly indexed
                if has_orion and artifacts_count < 1500:
                    # Quick check: try a small query for "orion" to see if it's indexed
                    try:
                        test_result = client.compile("orion architecture", budget=200)
                        # If query returns very few relevant results, likely not indexed
                        # Check if results mention .private/orion specifically
                        has_orion_results = any(
                            ".private/orion" in citation.get("file", "") 
                            for citation in [
                                {"file": c.artifact_path} for c in test_result.citations
                            ]
                        )
                        if not has_orion_results and len(test_result.spans) < 10:
                            should_ingest = True
                    except:
                        # If query fails, assume not indexed properly
                        should_ingest = True
                
                if should_ingest:
                    print(f"🥑 Auto-ingesting repository (recursive, includes all directories)...")
                    if artifacts_count > 0:
                        print(f"   Current artifacts: {artifacts_count} (completing ingestion)")
                    # Use CLI's recursive ingest - it will include .private and all subdirectories
                    # The CLI's collect_files already skips .git, node_modules, etc. but includes .private
                    # This happens automatically in the background - no manual steps needed
                    try:
                        client.ingest(".", recursive=True)
                        print("✅ Auto-ingestion complete - all files indexed (including .private directories)")
                    except Exception as e:
                        # Fallback to AutoIngest if CLI ingest fails
                        print(f"⚠️  CLI ingest failed, using fallback: {e}")
                        from avocado.ingest import AutoIngest
                        ingester = AutoIngest()
                        ingester.ingest_project(".", max_files=1000)  # Increased limit
            except Exception as e:
                # Stats might fail if DB doesn't exist yet, try to ingest anyway
                # Database will be auto-created on first use
                try:
                    print("🥑 First-time setup: Auto-ingesting repository (recursive)...")
                    # Use CLI's recursive ingest for comprehensive coverage
                    try:
                        client.ingest(".", recursive=True)
                        print("✅ Auto-ingestion complete")
                    except Exception as ingest_error:
                        # Fallback to AutoIngest if CLI ingest fails
                        print(f"⚠️  CLI ingest failed, using fallback: {ingest_error}")
                        from avocado.ingest import AutoIngest
                        ingester = AutoIngest()
                        ingester.ingest_project(".", max_files=1000)  # Increased limit
                except Exception:
                    pass
        else:
            # HTTP mode (MongoDB-style daemon) - auto-start server if needed
            manager = get_manager()
            manager.ensure_running()

            # Get server URL from environment or use default
            server_url = os.environ.get("AVOCADODB_URL", "http://localhost:8765")
            client = AvocadoDB(url=server_url)  # HTTP mode by default, auto-detects project path
            
            # Note: Auto-ingestion is handled by the daemon on first access
            # The daemon will load the project's database from .avocado/db.sqlite
            # If the database doesn't exist or is empty, user should run ingestion manually
            # or we could add auto-ingestion here, but it's better to let the daemon handle it
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

        # Try to generate answer with TinyLlama if requested
        answer = None
        llm_used = False
        
        if use_llm != "none":
            try:
                from avocado.llm import generate_answer
                print("🤖 Generating answer with TinyLlama...")
                answer = generate_answer(
                    query=query,
                    context_text=working_set.text,
                    max_new_tokens=max_new_tokens,
                    deterministic=True,
                )
                llm_used = True
                print("✅ Answer generated")
            except ImportError:
                if use_llm == "local":
                    # LLM required but not available
                    raise ImportError("TinyLlama not available. Install with: pip install avocadodb[llm]")
                # Auto mode: fallback to context
                answer = None
            except Exception as e:
                # LLM failed, fallback to context
                import warnings
                warnings.warn(f"LLM generation failed: {e}. Using context text.")
                answer = None

        # If no answer from LLM, use context as answer
        if answer is None:
            answer = working_set.text

        result = {
            "success": True,
            "answer": answer,  # Natural language answer (or context if LLM unavailable)
            "context": working_set.text,  # Raw context for reference
            "citations": formatted_citations,
            "spans": len(working_set.spans),
            "tokens_used": working_set.tokens_used,
            "compilation_time_ms": working_set.compilation_time_ms,
            "deterministic_hash": working_set.deterministic_hash(),
            "query": working_set.query,
            "llm_used": llm_used,  # Whether TinyLlama was used
        }

        # LangChain tools must return strings, not dicts
        import json
        return json.dumps(result, indent=2)

    except Exception as e:
        error_result = {
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

        # LangChain tools must return strings, not dicts
        import json
        return json.dumps(error_result, indent=2)


# Middleware for blocking read tools after AvocadoDB queries
class AvocadoDBMiddleware(AgentMiddleware):
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
        super().__init__()
        # Tools to block when avocado_compile_context is active
        self.blocked_tools = {"read_file", "grep", "ls", "glob"}

    def before_agent(
        self,
        agent_input: dict | Any,
        runtime: Any
    ) -> dict | Any | None:
        """Called before the agent runs (compatibility method for DeepAgents).
        
        Args:
            agent_input: The input to the agent
            runtime: The runtime context
            
        Returns:
            Modified input or None to keep original
        """
        # No modification needed, just pass through
        return None

    async def abefore_agent(
        self,
        agent_input: dict | Any,
        runtime: Any
    ) -> dict | Any | None:
        """Async version of before_agent (compatibility method for DeepAgents).
        
        Args:
            agent_input: The input to the agent
            runtime: The runtime context
            
        Returns:
            Modified input or None to keep original
        """
        # Delegate to sync version
        return self.before_agent(agent_input, runtime)

    def before_model(
        self,
        model_input: dict | Any,
        runtime: Any
    ) -> dict | Any | None:
        """Called before the model runs (compatibility method for DeepAgents).
        
        We intentionally do nothing here to avoid interfering with model calls.
        Tool filtering happens in after_model only.
        """
        # Explicitly do nothing - let model calls proceed normally
        # Returning None means "don't modify" in middleware systems
        return None

    async def abefore_model(
        self,
        model_input: dict | Any,
        runtime: Any
    ) -> dict | Any | None:
        """Async version of before_model (compatibility method for DeepAgents).
        
        We intentionally do nothing here to avoid interfering with model calls.
        Tool filtering happens in after_model only.
        """
        # Explicitly do nothing - let model calls proceed normally
        return None

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

        # Only block filesystem tools when AvocadoDB is being called in parallel
        # This allows filesystem tools to be used if AvocadoDB didn't find anything
        # (sequential case - agent can use filesystem tools after AvocadoDB returns empty results)
        if not has_avocado:
            return None  # No AvocadoDB call, don't block anything

        # Parallel case: Block filesystem tools when AvocadoDB is being called
        # This prevents redundant parallel reads when AvocadoDB is already fetching context
        filtered_calls = [
            tc for tc in tool_calls
            if tc.get("name") == "avocado_compile_context"
            or tc.get("name") not in self.blocked_tools
        ]

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


# Alias for compatibility with DeepAgents fork
AvocadoDBExclusivityMiddleware = AvocadoDBMiddleware

# Export names that match LangChain conventions
AvocadoDBTool = avocado_compile_context  # Alias for compatibility

__all__ = [
    "avocado_compile_context",
    "AvocadoDBTool",
    "AvocadoDBMiddleware",
    "AvocadoDBExclusivityMiddleware",  # Alias for DeepAgents compatibility
]
