"""
AvocadoDB Python SDK

Simple HTTP client for interacting with AvocadoDB server.
"""

from typing import Dict, List, Optional
import requests


class AvocadoDB:
    """Client for AvocadoDB context engine."""

    def __init__(self, url: str = "http://localhost:8080"):
        """Initialize client.

        Args:
            url: Server URL (default: http://localhost:8080)
        """
        self.url = url.rstrip("/")

    def ingest(
        self, path: str, content: Optional[str] = None, metadata: Optional[Dict] = None
    ) -> Dict:
        """Ingest a document.

        Args:
            path: Document path or identifier (must be unique)
            content: Document text. If None, reads from filesystem path
            metadata: Optional metadata dictionary

        Returns:
            Dict with artifact_id, spans_created, tokens_indexed

        Raises:
            requests.HTTPError: If the request fails
        """
        if content is None:
            with open(path, "r") as f:
                content = f.read()

        response = requests.post(
            f"{self.url}/ingest",
            json={"path": path, "content": content, "metadata": metadata},
        )
        response.raise_for_status()
        return response.json()

    def ingest_batch(self, documents: List[Dict]) -> Dict:
        """Ingest multiple documents.

        Args:
            documents: List of dicts with 'path', 'content', and optional 'metadata'

        Returns:
            Dict with 'results' list

        Raises:
            requests.HTTPError: If the request fails
        """
        response = requests.post(
            f"{self.url}/ingest/batch", json={"documents": documents}
        )
        response.raise_for_status()
        return response.json()

    def compile(
        self,
        query: str,
        budget: int = 8000,
        config: Optional[Dict] = None,
    ) -> "WorkingSet":
        """Compile a context working set.

        Args:
            query: Search query
            budget: Token budget (default: 8000)
            config: Optional compiler configuration

        Returns:
            WorkingSet object with compiled context

        Raises:
            requests.HTTPError: If the request fails
        """
        payload = {"query": query, "token_budget": budget}
        if config:
            payload["config"] = config

        response = requests.post(f"{self.url}/compile", json=payload)
        response.raise_for_status()
        return WorkingSet(response.json()["working_set"])

    def stats(self) -> Dict:
        """Get database statistics.

        Returns:
            Dict with artifacts_count, spans_count, total_tokens

        Raises:
            requests.HTTPError: If the request fails
        """
        response = requests.get(f"{self.url}/stats")
        response.raise_for_status()
        return response.json()

    def clear(self) -> None:
        """Clear all data.

        Raises:
            requests.HTTPError: If the request fails
        """
        response = requests.delete(f"{self.url}/clear")
        response.raise_for_status()


class WorkingSet:
    """A compiled context working set."""

    def __init__(self, data: Dict):
        """Initialize from API response data."""
        self.text = data["text"]
        self.spans = data["spans"]
        self.citations = data["citations"]
        self.tokens_used = data["tokens_used"]
        self.query = data.get("query", "")
        self.compilation_time_ms = data.get("compilation_time_ms", 0)

    def get_citation(self, index: int) -> Optional[Dict]:
        """Get citation by index (1-based).

        Args:
            index: Citation number (as shown in [1], [2], etc.)

        Returns:
            Citation dict or None if not found
        """
        if 0 < index <= len(self.citations):
            return self.citations[index - 1]
        return None

    def explain(self) -> Dict:
        """Get explanation of compilation process.

        Returns:
            Dict with compilation metadata
        """
        return {
            "query": self.query,
            "spans_included": len(self.spans),
            "tokens_used": self.tokens_used,
            "compilation_time_ms": self.compilation_time_ms,
            "citations": len(self.citations),
        }

    def __repr__(self) -> str:
        return (
            f"WorkingSet(query={self.query!r}, "
            f"tokens_used={self.tokens_used}, "
            f"spans={len(self.spans)})"
        )
