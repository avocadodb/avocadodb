"""CrewAI integration for AvocadoDB.

Provides AvocadoDBTool as a CrewAI-compatible tool.

Example:
    >>> from crewai import Agent, Task, Crew
    >>> from avocado.integrations.crewai import AvocadoDBTool
    >>>
    >>> agent = Agent(
    ...     role="Research Assistant",
    ...     goal="Answer questions about the codebase",
    ...     tools=[AvocadoDBTool()]
    ... )
    >>>
    >>> task = Task(
    ...     description="Explain how authentication works",
    ...     agent=agent
    ... )
    >>>
    >>> crew = Crew(agents=[agent], tasks=[task])
    >>> result = crew.kickoff()
"""

import os
from typing import Type

from pydantic import BaseModel, Field

from avocado.client import AvocadoDB
from avocado.manager import get_manager


class AvocadoQueryInput(BaseModel):
    """Input schema for AvocadoDB tool."""
    query: str = Field(..., description="Search query describing what information you need")
    token_budget: int = Field(8000, description="Maximum tokens to use")
    semantic_weight: float = Field(0.7, description="Weight for semantic search (0.0-1.0)")
    lexical_weight: float = Field(0.3, description="Weight for lexical search (0.0-1.0)")
    mmr_lambda: float = Field(0.5, description="Diversity parameter (0.0-1.0)")
    enable_mmr: bool = Field(True, description="Enable diversification")


class AvocadoDBTool:
    """CrewAI tool for deterministic, citation-backed context retrieval.

    AvocadoDB provides 100% deterministic context compilation - the same query
    always returns the same context, making responses reproducible and auditable.

    Example:
        >>> from crewai import Agent
        >>> from avocado.integrations.crewai import AvocadoDBTool
        >>>
        >>> agent = Agent(
        ...     role="Codebase Expert",
        ...     tools=[AvocadoDBTool()]
        ... )
    """

    name: str = "avocado_compile_context"
    description: str = (
        "Compile deterministic, citation-backed context from your knowledge base. "
        "Use this tool for ANY codebase or documentation questions. "
        "Returns relevant context with exact file:line citations."
    )
    args_schema: Type[BaseModel] = AvocadoQueryInput

    def _run(
        self,
        query: str,
        token_budget: int = 8000,
        semantic_weight: float = 0.7,
        lexical_weight: float = 0.3,
        mmr_lambda: float = 0.5,
        enable_mmr: bool = True,
    ) -> str:
        """Execute the tool.

        Args:
            query: Search query
            token_budget: Maximum tokens
            semantic_weight: Semantic search weight
            lexical_weight: Lexical search weight
            mmr_lambda: Diversity parameter
            enable_mmr: Enable diversification

        Returns:
            Formatted string with context and citations
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

            # Format citations for display
            citations_str = "\n".join(
                f"  - {c.artifact_path}:{c.start_line}-{c.end_line}"
                for c in working_set.citations[:10]
            )

            # Show token usage stats
            print(f"📊 AvocadoDB: {working_set.tokens_used:,} tokens used (budget: {token_budget:,}) | {len(working_set.spans)} spans | {working_set.compilation_time_ms}ms")

            # Return formatted result
            return (
                f"Context (from {len(working_set.spans)} spans, {working_set.tokens_used:,} tokens):\n\n"
                f"{working_set.text}\n\n"
                f"Citations:\n{citations_str}"
            )

        except Exception as e:
            return f"AvocadoDB error: {str(e)}\n\nMake sure server is running and documents are ingested."


__all__ = ["AvocadoDBTool", "AvocadoQueryInput"]
