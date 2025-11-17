"""
AvocadoDB Python SDK

Framework-agnostic SDK for AvocadoDB - the deterministic context database.

Features:
- HTTP client for compile/ingest/stats operations
- Server lifecycle management (auto-start, daemon mode)
- Background file monitoring and re-ingestion
- Smart auto-ingest with project type detection
- Framework integrations (LangChain, AutoGen, CrewAI)
- Utility functions (token counting, citation formatting)

Example:
    >>> # Basic usage (HTTP client)
    >>> from avocado import AvocadoDB
    >>> db = AvocadoDB()
    >>> result = db.compile("How does authentication work?")
    >>> print(result.text)

    >>> # With auto-management
    >>> from avocado import get_manager, AutoIngest
    >>> manager = get_manager()  # Auto-starts server
    >>> ingester = AutoIngest()
    >>> ingester.ingest_project(".")  # Auto-detects project type

    >>> # Framework integrations
    >>> # LangChain
    >>> from avocado.integrations.langchain import AvocadoDBTool, AvocadoDBMiddleware
    >>> # AutoGen
    >>> from avocado.integrations.autogen import avocado_compile_context
    >>> # CrewAI
    >>> from avocado.integrations.crewai import AvocadoDBTool
"""

# Core client
from .client import AvocadoDB, WorkingSet, Citation, Span

# Server lifecycle management
from .manager import AvocadoDBManager, get_manager

# Background monitoring
from .monitor import FileMonitor

# Smart auto-ingest
from .ingest import AutoIngest, ProjectType

# Utilities
from .utils import count_tokens, format_citations, create_system_prompt, format_working_set

# Legacy compatibility
from .deepagents_tool import avocado_compile_context as legacy_compile_context, compile_context

__version__ = "2.0.0"
__all__ = [
    # Core client
    "AvocadoDB",
    "WorkingSet",
    "Citation",
    "Span",
    # Server management
    "AvocadoDBManager",
    "get_manager",
    # File monitoring
    "FileMonitor",
    # Auto-ingest
    "AutoIngest",
    "ProjectType",
    # Utilities
    "count_tokens",
    "format_citations",
    "create_system_prompt",
    "format_working_set",
    # Legacy
    "legacy_compile_context",
    "compile_context",
]
