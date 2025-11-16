"""
AvocadoDB HTTP client implementation.

Simple wrapper around the AvocadoDB HTTP API.
"""

import requests
from typing import Optional, Dict, List, Any
from dataclasses import dataclass
import hashlib


@dataclass
class Citation:
    """A citation linking to a source location."""
    span_id: str
    artifact_id: str
    artifact_path: str
    start_line: int
    end_line: int
    score: float

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'Citation':
        return cls(
            span_id=data['span_id'],
            artifact_id=data['artifact_id'],
            artifact_path=data['artifact_path'],
            start_line=data['start_line'],
            end_line=data['end_line'],
            score=data.get('score', 0.0)
        )


@dataclass
class Span:
    """A text span with metadata."""
    id: str
    artifact_id: str
    start_line: int
    end_line: int
    text: str
    token_count: int
    embedding: Optional[List[float]] = None
    embedding_model: Optional[str] = None
    metadata: Optional[Dict[str, Any]] = None

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'Span':
        return cls(
            id=data['id'],
            artifact_id=data['artifact_id'],
            start_line=data['start_line'],
            end_line=data['end_line'],
            text=data['text'],
            token_count=data['token_count'],
            embedding=data.get('embedding'),
            embedding_model=data.get('embedding_model'),
            metadata=data.get('metadata')
        )


class WorkingSet:
    """A compiled context working set with deterministic guarantees."""

    def __init__(self, data: Dict[str, Any]):
        """Initialize from API response data."""
        self.text: str = data['text']
        self.spans: List[Span] = [Span.from_dict(s) for s in data['spans']]
        self.citations: List[Citation] = [Citation.from_dict(c) for c in data['citations']]
        self.tokens_used: int = data['tokens_used']
        self.query: str = data['query']
        self.compilation_time_ms: int = data['compilation_time_ms']

    def deterministic_hash(self) -> str:
        """Calculate deterministic hash of the context text.

        Returns:
            SHA-256 hash of the compiled text (hex string)
        """
        return hashlib.sha256(self.text.encode('utf-8')).hexdigest()

    def __repr__(self) -> str:
        return (
            f"WorkingSet(query='{self.query}', "
            f"spans={len(self.spans)}, "
            f"tokens={self.tokens_used}, "
            f"time={self.compilation_time_ms}ms)"
        )


class AvocadoDB:
    """AvocadoDB client for deterministic context compilation.

    Simple HTTP client for interacting with AvocadoDB server.

    Example:
        >>> db = AvocadoDB("http://localhost:8080")
        >>> db.ingest("./docs")
        >>> result = db.compile("How does authentication work?")
        >>> print(result.text)
    """

    def __init__(self, url: str = "http://localhost:8080"):
        """Initialize AvocadoDB client.

        Args:
            url: Base URL of AvocadoDB server (default: http://localhost:8080)
        """
        self.url = url.rstrip('/')
        self.session = requests.Session()

    def compile(
        self,
        query: str,
        budget: int = 8000,
        semantic_weight: float = 0.7,
        lexical_weight: float = 0.3,
        mmr_lambda: float = 0.5,
        enable_mmr: bool = True
    ) -> WorkingSet:
        """Compile a deterministic context for a query.

        Args:
            query: Search query
            budget: Token budget for compiled context (default: 8000)
            semantic_weight: Weight for semantic search (default: 0.7)
            lexical_weight: Weight for lexical search (default: 0.3)
            mmr_lambda: MMR diversity parameter, 0.0-1.0 (default: 0.5)
                - Higher (0.7-1.0) = more relevant but potentially redundant
                - Lower (0.0-0.3) = more diverse but potentially less relevant
            enable_mmr: Enable MMR diversification (default: True)

        Returns:
            WorkingSet with compiled context and citations

        Raises:
            requests.HTTPError: If API request fails

        Example:
            >>> result = db.compile("authentication", budget=8000)
            >>> print(f"Compiled {len(result.spans)} spans")
            >>> print(f"Hash: {result.deterministic_hash()}")
        """
        response = self.session.post(
            f"{self.url}/compile",
            json={
                "query": query,
                "token_budget": budget,
                "semantic_weight": semantic_weight,
                "lexical_weight": lexical_weight,
                "mmr_lambda": mmr_lambda,
                "enable_mmr": enable_mmr
            }
        )
        response.raise_for_status()
        return WorkingSet(response.json())

    def ingest(self, path: str, content: Optional[str] = None) -> Dict[str, Any]:
        """Ingest a document into the database.

        Args:
            path: Document path (used as artifact identifier)
            content: Document content (if None, reads from file)

        Returns:
            Dict with artifact_id and span count

        Raises:
            requests.HTTPError: If API request fails
            FileNotFoundError: If content is None and path doesn't exist

        Example:
            >>> result = db.ingest("docs/auth.md")
            >>> print(f"Created {result['spans']} spans")
        """
        if content is None:
            with open(path, 'r', encoding='utf-8') as f:
                content = f.read()

        response = self.session.post(
            f"{self.url}/ingest",
            json={
                "path": path,
                "content": content
            }
        )
        response.raise_for_status()
        return response.json()

    def stats(self) -> Dict[str, int]:
        """Get database statistics.

        Returns:
            Dict with artifacts, spans, and tokens counts

        Example:
            >>> stats = db.stats()
            >>> print(f"Database: {stats['spans']} spans, {stats['tokens']} tokens")
        """
        response = self.session.get(f"{self.url}/stats")
        response.raise_for_status()
        return response.json()

    def close(self):
        """Close the HTTP session."""
        self.session.close()

    def __enter__(self):
        """Context manager entry."""
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        """Context manager exit."""
        self.close()
