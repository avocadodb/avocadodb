"""
AvocadoDB Python SDK

Dead simple HTTP client for AvocadoDB - the deterministic context database.

Example:
    >>> from avocado import AvocadoDB
    >>> db = AvocadoDB()
    >>> db.ingest("./docs", recursive=True)
    >>> result = db.compile("How does authentication work?")
    >>> print(result.text)
"""

from .client import AvocadoDB, WorkingSet, Citation, Span
from .deepagents_tool import avocado_compile_context, compile_context

__version__ = "1.0.0"
__all__ = [
    "AvocadoDB",
    "WorkingSet",
    "Citation",
    "Span",
    "avocado_compile_context",
    "compile_context",
]
