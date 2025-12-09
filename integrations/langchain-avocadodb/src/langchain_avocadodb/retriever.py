"""
AvocadoDB retriever for LangChain.

Provides deterministic context retrieval with line-level citations.
"""

from typing import List, Optional, Dict, Any
from langchain_core.retrievers import BaseRetriever
from langchain_core.documents import Document
from langchain_core.callbacks import CallbackManagerForRetrieverRun
from pydantic import Field, PrivateAttr
import logging

try:
    from avocado import AvocadoDB, WorkingSet
except ImportError:
    raise ImportError(
        "AvocadoDB Python SDK is required. " "Install with: pip install avocadodb"
    )

logger = logging.getLogger(__name__)


class AvocadoDBRetriever(BaseRetriever):
    """
    LangChain retriever backed by AvocadoDB for deterministic RAG.

    AvocadoDB provides:
    - 100% deterministic retrieval (same query = same results)
    - Line-level citations with source tracking
    - 6x faster embeddings than OpenAI (pure Rust)
    - 95% token efficiency with smart chunking

    Example:
        >>> from langchain_avocadodb import AvocadoDBRetriever
        >>> from langchain.chains import RetrievalQA
        >>> from langchain_openai import ChatOpenAI
        >>>
        >>> # Initialize retriever
        >>> retriever = AvocadoDBRetriever(
        ...     url="http://localhost:8765",
        ...     budget=8000,
        ...     include_citations=True
        ... )
        >>>
        >>> # Use with chains
        >>> qa_chain = RetrievalQA.from_chain_type(
        ...     llm=ChatOpenAI(),
        ...     retriever=retriever,
        ...     return_source_documents=True
        ... )
        >>>
        >>> result = qa_chain.invoke("How does authentication work?")
        >>> print(result["answer"])
        >>> # Sources include line-level citations
        >>> for doc in result["source_documents"]:
        ...     m = doc.metadata
        ...     print(f"Source: {m['source']}:{m['start_line']}-{m['end_line']}")
    """

    # Configuration fields
    url: str = Field(
        default="http://localhost:8765", description="AvocadoDB server URL"
    )
    mode: str = Field(
        default="http", description="Connection mode: 'http' (server) or 'cli' (direct)"
    )
    budget: int = Field(
        default=8000, description="Token budget for context compilation"
    )
    semantic_weight: float = Field(
        default=0.7, description="Weight for semantic (vector) search (0.0-1.0)"
    )
    lexical_weight: float = Field(
        default=0.3, description="Weight for lexical (keyword) search (0.0-1.0)"
    )
    mmr_lambda: float = Field(
        default=0.5, description="MMR diversity parameter (0.0=diverse, 1.0=relevant)"
    )
    enable_mmr: bool = Field(
        default=True,
        description="Enable Maximal Marginal Relevance for result diversity",
    )
    include_citations: bool = Field(
        default=True, description="Include citation information in metadata"
    )
    include_scores: bool = Field(
        default=True, description="Include relevance scores in metadata"
    )
    combine_spans: bool = Field(
        default=False,
        description="Combine adjacent spans from same file into single documents",
    )
    min_score: Optional[float] = Field(
        default=None, description="Minimum score threshold for returned documents"
    )

    # Private client instance
    _client: Optional[AvocadoDB] = PrivateAttr(default=None)

    class Config:
        """Pydantic config."""

        arbitrary_types_allowed = True
        extra = "forbid"

    def __init__(self, **kwargs):
        """Initialize the retriever."""
        super().__init__(**kwargs)
        self._initialize_client()

    def _initialize_client(self) -> None:
        """Initialize the AvocadoDB client."""
        if self.mode == "cli":
            self._client = AvocadoDB(mode="cli")
        else:
            self._client = AvocadoDB(url=self.url)

    @property
    def client(self) -> AvocadoDB:
        """Get the AvocadoDB client, initializing if needed."""
        if self._client is None:
            self._initialize_client()
        return self._client

    def _get_relevant_documents(
        self,
        query: str,
        *,
        run_manager: Optional[CallbackManagerForRetrieverRun] = None,
    ) -> List[Document]:
        """
        Get documents relevant to a query using AvocadoDB.

        Args:
            query: The search query
            run_manager: Callback manager for the retriever run

        Returns:
            List of Document objects with content and metadata
        """
        # Log the query
        if run_manager:
            run_manager.on_text(f"Querying AvocadoDB: {query}\n")

        # Compile context using AvocadoDB
        try:
            working_set = self.client.compile(
                query=query,
                budget=self.budget,
                semantic_weight=self.semantic_weight,
                lexical_weight=self.lexical_weight,
                mmr_lambda=self.mmr_lambda,
                enable_mmr=self.enable_mmr,
            )
        except Exception as e:
            logger.error(f"AvocadoDB compilation failed: {e}")
            if run_manager:
                run_manager.on_text(f"Error: {e}\n", color="red")
            return []

        # Log statistics
        if run_manager:
            run_manager.on_text(
                f"Found {len(working_set.spans)} spans, "
                f"{working_set.tokens_used} tokens, "
                f"in {working_set.compilation_time_ms}ms\n",
                color="green",
            )

        # Convert spans to LangChain Documents
        documents = self._convert_to_documents(working_set)

        # Filter by minimum score if specified
        if self.min_score is not None:
            documents = [
                doc
                for doc in documents
                if doc.metadata.get("score", 0) >= self.min_score
            ]

        return documents

    async def _aget_relevant_documents(
        self,
        query: str,
        *,
        run_manager: Optional[CallbackManagerForRetrieverRun] = None,
    ) -> List[Document]:
        """
        Async version of document retrieval.

        Currently uses sync implementation. Future versions will
        support native async AvocadoDB client.
        """
        # TODO: Implement native async when AvocadoDB supports it
        return self._get_relevant_documents(query, run_manager=run_manager)

    def _convert_to_documents(self, working_set: WorkingSet) -> List[Document]:
        """
        Convert AvocadoDB WorkingSet to LangChain Documents.

        Args:
            working_set: The compiled working set from AvocadoDB

        Returns:
            List of Document objects
        """
        documents = []

        if self.combine_spans:
            # Group adjacent spans from same file
            documents = self._combine_adjacent_spans(working_set)
        else:
            # Convert each span to a separate document
            for span in working_set.spans:
                metadata = self._create_metadata(span, working_set)

                doc = Document(page_content=span.text, metadata=metadata)
                documents.append(doc)

        return documents

    def _create_metadata(self, span: Any, working_set: WorkingSet) -> Dict[str, Any]:
        """
        Create metadata for a document from a span.

        Args:
            span: The span object from AvocadoDB
            working_set: The working set containing citations

        Returns:
            Metadata dictionary
        """
        metadata = {
            "source": span.artifact_path,
            "start_line": span.start_line,
            "end_line": span.end_line,
            "token_count": span.token_count,
            "span_id": span.id,
            "deterministic_hash": working_set.deterministic_hash()[:16],
            "query": working_set.query,
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
                        "lines": f"{c.start_line}-{c.end_line}",
                        "score": c.score,
                    }
                    for c in citations
                ]

        return metadata

    def _combine_adjacent_spans(self, working_set: WorkingSet) -> List[Document]:
        """
        Combine adjacent spans from the same file into single documents.

        Args:
            working_set: The working set with spans to combine

        Returns:
            List of combined documents
        """
        if not working_set.spans:
            return []

        documents = []
        current_doc = None
        current_spans = []

        for span in working_set.spans:
            # Check if this span should be combined with previous
            if current_doc and self._should_combine(current_spans[-1], span):
                # Add to current document
                current_spans.append(span)
            else:
                # Save current document if exists
                if current_doc:
                    documents.append(
                        self._finalize_combined_doc(
                            current_doc, current_spans, working_set
                        )
                    )

                # Start new document
                current_spans = [span]
                current_doc = {
                    "source": span.artifact_path,
                    "start_line": span.start_line,
                    "text": "",
                }

        # Save final document
        if current_doc:
            documents.append(
                self._finalize_combined_doc(current_doc, current_spans, working_set)
            )

        return documents

    def _should_combine(self, prev_span: Any, curr_span: Any) -> bool:
        """
        Check if two spans should be combined.

        Spans are combined if they're from the same file and adjacent
        (within 5 lines of each other).
        """
        if prev_span.artifact_path != curr_span.artifact_path:
            return False

        # Check if spans are adjacent (within 5 lines)
        gap = curr_span.start_line - prev_span.end_line
        return 0 <= gap <= 5

    def _finalize_combined_doc(
        self, doc_info: Dict[str, Any], spans: List[Any], working_set: WorkingSet
    ) -> Document:
        """
        Finalize a combined document from multiple spans.
        """
        # Combine text from all spans
        combined_text = "\n\n".join(span.text for span in spans)

        # Create metadata
        metadata = {
            "source": doc_info["source"],
            "start_line": spans[0].start_line,
            "end_line": spans[-1].end_line,
            "span_count": len(spans),
            "token_count": sum(span.token_count for span in spans),
            "deterministic_hash": working_set.deterministic_hash()[:16],
            "query": working_set.query,
        }

        # Add average score if requested
        if self.include_scores:
            scores = [s.score for s in spans if hasattr(s, "score")]
            if scores:
                metadata["avg_score"] = sum(scores) / len(scores)
                metadata["max_score"] = max(scores)

        # Add all citations if requested
        if self.include_citations:
            all_citations = []
            for span in spans:
                citations = [c for c in working_set.citations if c.span_id == span.id]
                all_citations.extend(citations)

            if all_citations:
                metadata["citations"] = [
                    {
                        "file": c.artifact_path,
                        "lines": f"{c.start_line}-{c.end_line}",
                        "score": c.score,
                    }
                    for c in all_citations
                ]

        return Document(page_content=combined_text, metadata=metadata)

    def get_relevant_documents(self, query: str, **kwargs) -> List[Document]:
        """
        Public method to get relevant documents.

        This is the main entry point for retrieving documents.
        """
        return self._get_relevant_documents(query)

    async def aget_relevant_documents(self, query: str, **kwargs) -> List[Document]:
        """
        Async public method to get relevant documents.
        """
        return await self._aget_relevant_documents(query)

    def invoke(
        self, input: str, config: Optional[Dict[str, Any]] = None
    ) -> List[Document]:
        """
        Invoke the retriever with a query string.

        Implements the Runnable interface.
        """
        return self.get_relevant_documents(input)

    async def ainvoke(
        self, input: str, config: Optional[Dict[str, Any]] = None
    ) -> List[Document]:
        """
        Async invoke the retriever with a query string.

        Implements the Runnable interface.
        """
        return await self.aget_relevant_documents(input)
