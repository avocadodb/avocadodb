"""Utility functions for AvocadoDB.

Provides:
- Exact token counting (using tiktoken)
- Citation formatting
- System prompt generation
- Response formatting

Example:
    >>> from avocado.utils import count_tokens, format_citations
    >>> tokens = count_tokens("Hello world", model="gpt-4")
    >>> citations_str = format_citations(result.citations)
"""

from typing import Any


def count_tokens(text: str, model: str = "gpt-4") -> int:
    """Count exact tokens using tiktoken.

    Args:
        text: Text to count tokens for
        model: Model to use for tokenization (gpt-4, gpt-3.5-turbo, claude, etc.)

    Returns:
        Exact token count

    Example:
        >>> tokens = count_tokens("Hello world!", model="gpt-4")
        >>> print(f"Tokens: {tokens}")
    """
    try:
        import tiktoken

        # Try to get encoding for specific model
        try:
            encoding = tiktoken.encoding_for_model(model)
        except KeyError:
            # Fallback to cl100k_base (GPT-4/Claude tokenizer)
            encoding = tiktoken.get_encoding("cl100k_base")

        return len(encoding.encode(text))

    except ImportError:
        # Tiktoken not installed - use rough approximation
        # ~4 characters per token on average
        return len(text) // 4


def format_citations(citations: list[dict[str, Any]], style: str = "compact") -> str:
    """Format citations for display.

    Args:
        citations: List of citation dicts with 'file' and 'lines' keys
        style: Format style ('compact', 'verbose', 'markdown')

    Returns:
        Formatted citation string

    Example:
        >>> citations = [
        ...     {"file": "auth.md", "lines": "10-25"},
        ...     {"file": "api.py", "lines": "45-78"}
        ... ]
        >>> print(format_citations(citations, style="compact"))
        auth.md:10-25, api.py:45-78

        >>> print(format_citations(citations, style="markdown"))
        - `auth.md:10-25`
        - `api.py:45-78`
    """
    if not citations:
        return "No citations"

    if style == "compact":
        return ", ".join(f"{c['file']}:{c['lines']}" for c in citations)

    elif style == "verbose":
        lines = []
        for c in citations:
            lines.append(f"  📄 {c['file']} (lines {c['lines']})")
        return "\n".join(lines)

    elif style == "markdown":
        return "\n".join(f"- `{c['file']}:{c['lines']}`" for c in citations)

    else:
        return ", ".join(f"{c['file']}:{c['lines']}" for c in citations)


def create_system_prompt(
    framework: str = "generic",
    enforce_avocado_only: bool = True,
) -> str:
    """Generate AvocadoDB-first system prompt for any framework.

    Args:
        framework: Target framework ("langchain", "autogen", "crewai", "generic")
        enforce_avocado_only: Enforce AvocadoDB-only protocol (block read tools)

    Returns:
        System prompt string

    Example:
        >>> prompt = create_system_prompt(framework="langchain")
        >>> # Use in agent configuration
    """
    base_prompt = """
### AvocadoDB Context Compilation

For ANY codebase or documentation question, you have access to AvocadoDB - a
deterministic context database that provides citation-backed information retrieval.

**When to use AvocadoDB:**
- Questions about the codebase, architecture, or implementation
- Documentation lookups
- Understanding how features work
- Finding relevant code or information

**How AvocadoDB works:**
1. You provide a query describing what information you need
2. AvocadoDB searches ALL indexed documents (not just one file)
3. Returns relevant context with exact file:line citations
4. Same query always returns same context (100% deterministic)

**Key benefits:**
- ✅ Comprehensive: Searches all indexed files
- ✅ Deterministic: Same query → same context
- ✅ Citation-backed: Every span includes exact source location
- ✅ Efficient: Returns only relevant information within token budget
"""

    if enforce_avocado_only:
        avocado_only = """
**AVOCADODB-ONLY PROTOCOL (MANDATORY):**

For ANY codebase question (architecture, files, code, documentation):
1. Call ONLY `avocado_compile_context` with a well-formed query
2. WAIT for results (contains relevant spans with citations)
3. Synthesize answer EXCLUSIVELY from the returned context
4. DO NOT call read_file, ls, grep, or glob afterward - EVER
5. The AvocadoDB context is ALWAYS sufficient for codebase questions

Why AvocadoDB is better than reading files:
- Searches ALL indexed files (comprehensive, won't miss updates in other files)
- Returns multi-source information (not just one file's perspective)
- Provides exact citations (know where each fact came from)
- Deterministic results (same query = same comprehensive answer)
- Prevents incomplete answers from single-file reads

FORBIDDEN PATTERNS:
❌ avocado_compile_context → then read_file (NO! Trust AvocadoDB's comprehensive search)
❌ avocado_compile_context → then ls (NO! Files are in citations)
❌ avocado_compile_context + read_file in parallel (NO! One tool only)

CORRECT PATTERN:
✅ avocado_compile_context → synthesize answer from context + citations

If AvocadoDB results seem insufficient, improve your query or tell the user
what's missing. NEVER fall back to reading files.
"""
        return base_prompt + avocado_only

    return base_prompt


def format_working_set(working_set: Any, include_context: bool = False) -> str:
    """Format WorkingSet for human-readable display.

    Args:
        working_set: WorkingSet object from AvocadoDB
        include_context: Include full context text (can be long)

    Returns:
        Formatted string

    Example:
        >>> from avocado import AvocadoDB
        >>> db = AvocadoDB()
        >>> result = db.compile("authentication")
        >>> print(format_working_set(result))
    """
    lines = []
    lines.append(f"Query: {working_set.query}")
    lines.append(f"Spans: {len(working_set.spans)}")
    lines.append(f"Tokens Used: {working_set.tokens_used:,}")
    lines.append(f"Compilation Time: {working_set.compilation_time_ms}ms")
    lines.append(f"Deterministic Hash: {working_set.deterministic_hash()[:16]}...")
    lines.append("")
    lines.append("Citations:")
    for citation in working_set.citations[:10]:  # Show first 10
        lines.append(f"  - {citation.artifact_path}:{citation.start_line}-{citation.end_line}")

    if len(working_set.citations) > 10:
        lines.append(f"  ... and {len(working_set.citations) - 10} more")

    if include_context:
        lines.append("")
        lines.append("Context:")
        lines.append("─" * 60)
        lines.append(working_set.text[:500])  # First 500 chars
        if len(working_set.text) > 500:
            lines.append("...")

    return "\n".join(lines)


__all__ = [
    "count_tokens",
    "format_citations",
    "create_system_prompt",
    "format_working_set",
]
