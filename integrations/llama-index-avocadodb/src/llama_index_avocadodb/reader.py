"""
AvocadoDB reader for LlamaIndex.

Provides deterministic document loading with line-level citations.
"""

from typing import List, Optional, Dict, Any, Iterator
from llama_index.core.readers.base import BaseReader
from llama_index.core.schema import Document, TextNode
import logging

try:
    from avocado import AvocadoDB, WorkingSet
except ImportError:
    raise ImportError(
        "AvocadoDB Python SDK is required. " "Install with: pip install avocadodb"
    )

logger = logging.getLogger(__name__)


class AvocadoDBReader(BaseReader):
    """
    LlamaIndex reader for AvocadoDB deterministic retrieval.

    AvocadoDB provides:
    - 100% deterministic retrieval (same query = same results)
    - Line-level citations with source tracking
    - 6x faster embeddings than OpenAI (pure Rust)
    - 95% token efficiency with smart chunking

    Example:
        >>> from llama_index_avocadodb import AvocadoDBReader
        >>> from llama_index.core import VectorStoreIndex
        >>>
        >>> # Initialize reader
        >>> reader = AvocadoDBReader(
        ...     url="http://localhost:8765",
        ...     budget=8000
        ... )
        >>>
        >>> # Load documents for a query
        >>> documents = reader.load_data("How does authentication work?")
        >>>
        >>> # Create index and query
        >>> index = VectorStoreIndex.from_documents(documents)
        >>> query_engine = index.as_query_engine()
        >>> response = query_engine.query("Explain JWT validation")
        >>>
        >>> # Citations are preserved in metadata
        >>> for node in response.source_nodes:
        ...     print(f"Source: {node.metadata['file_path']}:{node.metadata['start_line']}-{node.metadata['end_line']}")
    """

    def __init__(
        self,
        url: str = "http://localhost:8765",
        mode: str = "http",
        budget: int = 8000,
        semantic_weight: float = 0.7,
        lexical_weight: float = 0.3,
        mmr_lambda: float = 0.5,
        enable_mmr: bool = True,
        include_citations: bool = True,
        include_scores: bool = True,
        create_nodes: bool = False,
        combine_adjacent: bool = False,
        min_score: Optional[float] = None,
    ):
        """
        Initialize AvocadoDB reader.

        Args:
            url: AvocadoDB server URL (for HTTP mode)
            mode: Connection mode ('http' for server, 'cli' for direct)
            budget: Token budget for context compilation
            semantic_weight: Weight for semantic search (0.0-1.0)
            lexical_weight: Weight for lexical search (0.0-1.0)
            mmr_lambda: MMR diversity parameter (0.0=diverse, 1.0=relevant)
            enable_mmr: Enable Maximal Marginal Relevance
            include_citations: Include citation information in metadata
            include_scores: Include relevance scores in metadata
            create_nodes: Return TextNodes instead of Documents
            combine_adjacent: Combine adjacent spans from same file
            min_score: Minimum score threshold for returned documents
        """
        super().__init__()

        # Initialize client based on mode
        if mode == "cli":
            self.client = AvocadoDB(mode="cli")
        else:
            self.client = AvocadoDB(url=url)

        # Store configuration
        self.budget = budget
        self.semantic_weight = semantic_weight
        self.lexical_weight = lexical_weight
        self.mmr_lambda = mmr_lambda
        self.enable_mmr = enable_mmr
        self.include_citations = include_citations
        self.include_scores = include_scores
        self.create_nodes = create_nodes
        self.combine_adjacent = combine_adjacent
        self.min_score = min_score

    def load_data(
        self, query: str, budget: Optional[int] = None, **kwargs: Any
    ) -> List[Document]:
        """
        Load data for a query from AvocadoDB.

        Args:
            query: Search query to find relevant documents
            budget: Override default token budget
            **kwargs: Additional parameters for compile()

        Returns:
            List of Document objects with content and metadata
        """
        # Compile context using AvocadoDB
        try:
            working_set = self.client.compile(
                query=query,
                budget=budget or self.budget,
                semantic_weight=kwargs.get("semantic_weight", self.semantic_weight),
                lexical_weight=kwargs.get("lexical_weight", self.lexical_weight),
                mmr_lambda=kwargs.get("mmr_lambda", self.mmr_lambda),
                enable_mmr=kwargs.get("enable_mmr", self.enable_mmr),
            )
        except Exception as e:
            logger.error(f"AvocadoDB compilation failed: {e}")
            return []

        # Log statistics
        logger.info(
            f"AvocadoDB query '{query}': "
            f"{len(working_set.spans)} spans, "
            f"{working_set.tokens_used} tokens, "
            f"{working_set.compilation_time_ms}ms"
        )

        # Convert to LlamaIndex documents
        if self.combine_adjacent:
            documents = self._create_combined_documents(working_set)
        else:
            documents = self._create_documents(working_set)

        # Filter by minimum score if specified
        if self.min_score is not None:
            documents = [
                doc
                for doc in documents
                if doc.metadata.get("score", 0) >= self.min_score
            ]

        # Convert to TextNodes if requested
        if self.create_nodes:
            return self._convert_to_nodes(documents, working_set)

        return documents

    def load_data_batch(
        self, queries: List[str], budget: Optional[int] = None, **kwargs: Any
    ) -> List[List[Document]]:
        """
        Load data for multiple queries.

        Args:
            queries: List of search queries
            budget: Override default token budget
            **kwargs: Additional parameters

        Returns:
            List of document lists, one per query
        """
        results = []
        for query in queries:
            docs = self.load_data(query, budget, **kwargs)
            results.append(docs)
        return results

    def _create_documents(self, working_set: WorkingSet) -> List[Document]:
        """
        Create Document objects from WorkingSet spans.

        Args:
            working_set: Compiled working set from AvocadoDB

        Returns:
            List of Document objects
        """
        documents = []

        for span in working_set.spans:
            # Create metadata
            metadata = self._create_metadata(span, working_set)

            # Create document
            doc = Document(text=span.text, metadata=metadata, id_=span.id)
            documents.append(doc)

        return documents

    def _create_metadata(self, span: Any, working_set: WorkingSet) -> Dict[str, Any]:
        """
        Create metadata dictionary for a span.

        Args:
            span: Span object from AvocadoDB
            working_set: Working set containing citations

        Returns:
            Metadata dictionary
        """
        metadata = {
            "file_path": span.artifact_path,
            "start_line": span.start_line,
            "end_line": span.end_line,
            "token_count": span.token_count,
            "span_id": span.id,
            "artifact_id": span.artifact_id,
            "query": working_set.query,
            "deterministic_hash": working_set.deterministic_hash()[:16],
            "compilation_time_ms": working_set.compilation_time_ms,
        }

        # Add score if requested
        if self.include_scores and hasattr(span, "score"):
            metadata["score"] = span.score

        # Add citations if requested
        if self.include_citations:
            citations = [c for c in working_set.citations if c.span_id == span.id]
            if citations:
                metadata["citations"] = [
                    {
                        "file": c.artifact_path,
                        "start_line": c.start_line,
                        "end_line": c.end_line,
                        "score": c.score,
                    }
                    for c in citations
                ]

        return metadata

    def _create_combined_documents(self, working_set: WorkingSet) -> List[Document]:
        """
        Create documents by combining adjacent spans from same file.

        Args:
            working_set: Working set with spans to combine

        Returns:
            List of combined documents
        """
        if not working_set.spans:
            return []

        documents = []
        current_spans = []
        current_file = None

        for span in working_set.spans:
            # Check if this span should be combined
            if current_file and self._should_combine(current_spans[-1], span):
                current_spans.append(span)
            else:
                # Save current document if exists
                if current_spans:
                    doc = self._finalize_combined_document(current_spans, working_set)
                    documents.append(doc)

                # Start new document
                current_spans = [span]
                current_file = span.artifact_path

        # Save final document
        if current_spans:
            doc = self._finalize_combined_document(current_spans, working_set)
            documents.append(doc)

        return documents

    def _should_combine(self, prev_span: Any, curr_span: Any) -> bool:
        """
        Check if two spans should be combined.

        Spans are combined if:
        - They're from the same file
        - They're within 5 lines of each other
        """
        if prev_span.artifact_path != curr_span.artifact_path:
            return False

        gap = curr_span.start_line - prev_span.end_line
        return 0 <= gap <= 5

    def _finalize_combined_document(
        self, spans: List[Any], working_set: WorkingSet
    ) -> Document:
        """
        Create a single document from multiple spans.

        Args:
            spans: List of spans to combine
            working_set: Working set for metadata

        Returns:
            Combined Document
        """
        # Combine text with clear separators
        combined_text = "\n\n".join(span.text for span in spans)

        # Create aggregated metadata
        metadata = {
            "file_path": spans[0].artifact_path,
            "start_line": spans[0].start_line,
            "end_line": spans[-1].end_line,
            "span_count": len(spans),
            "token_count": sum(span.token_count for span in spans),
            "query": working_set.query,
            "deterministic_hash": working_set.deterministic_hash()[:16],
            "compilation_time_ms": working_set.compilation_time_ms,
        }

        # Add scores if available
        if self.include_scores:
            scores = [span.score for span in spans if hasattr(span, "score")]
            if scores:
                metadata["avg_score"] = sum(scores) / len(scores)
                metadata["max_score"] = max(scores)
                metadata["min_score"] = min(scores)

        # Aggregate citations
        if self.include_citations:
            all_citations = []
            for span in spans:
                citations = [c for c in working_set.citations if c.span_id == span.id]
                all_citations.extend(citations)

            if all_citations:
                # Deduplicate citations by file and line range
                unique_citations = {}
                for c in all_citations:
                    key = (c.artifact_path, c.start_line, c.end_line)
                    if key not in unique_citations:
                        unique_citations[key] = c

                metadata["citations"] = [
                    {
                        "file": c.artifact_path,
                        "start_line": c.start_line,
                        "end_line": c.end_line,
                        "score": c.score,
                    }
                    for c in unique_citations.values()
                ]

        # Create combined ID from span IDs
        combined_id = "_".join(span.id for span in spans)

        return Document(text=combined_text, metadata=metadata, id_=combined_id)

    def _convert_to_nodes(
        self, documents: List[Document], working_set: WorkingSet
    ) -> List[TextNode]:
        """
        Convert Documents to TextNodes for more control.

        Args:
            documents: List of documents to convert
            working_set: Working set for additional metadata

        Returns:
            List of TextNode objects
        """
        nodes = []

        for doc in documents:
            # Create TextNode with relationships
            node = TextNode(
                text=doc.text,
                metadata=doc.metadata,
                id_=doc.id_,
            )

            # Add relationships if we have citation info
            if "citations" in doc.metadata:
                # Could add source relationships here
                pass

            nodes.append(node)

        return nodes

    def lazy_load_data(
        self, query: str, budget: Optional[int] = None, **kwargs: Any
    ) -> Iterator[Document]:
        """
        Lazy load data (generator version).

        Yields documents one at a time for memory efficiency.

        Args:
            query: Search query
            budget: Token budget override
            **kwargs: Additional parameters

        Yields:
            Document objects one at a time
        """
        # Get working set
        working_set = self.client.compile(
            query=query,
            budget=budget or self.budget,
            semantic_weight=kwargs.get("semantic_weight", self.semantic_weight),
            lexical_weight=kwargs.get("lexical_weight", self.lexical_weight),
            mmr_lambda=kwargs.get("mmr_lambda", self.mmr_lambda),
            enable_mmr=kwargs.get("enable_mmr", self.enable_mmr),
        )

        # Yield documents one at a time
        for span in working_set.spans:
            # Check score threshold
            if self.min_score and hasattr(span, "score"):
                if span.score < self.min_score:
                    continue

            # Create metadata
            metadata = self._create_metadata(span, working_set)

            # Yield document
            yield Document(text=span.text, metadata=metadata, id_=span.id)
