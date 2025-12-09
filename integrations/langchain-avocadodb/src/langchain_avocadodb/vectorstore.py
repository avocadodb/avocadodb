"""
AvocadoDB VectorStore adapter for LangChain.

Provides a VectorStore-compatible interface that wraps AvocadoDBRetriever.
This allows AvocadoDB to be used as a drop-in replacement for other vector stores.
"""

from typing import List, Optional, Dict, Any, Tuple
from langchain_core.vectorstores import VectorStore
from langchain_core.documents import Document
from langchain_core.embeddings import Embeddings
from langchain_avocadodb.retriever import AvocadoDBRetriever
import logging

logger = logging.getLogger(__name__)


class AvocadoDBVectorStore(VectorStore):
    """
    VectorStore adapter for AvocadoDB.

    This class provides a VectorStore-compatible interface while using
    AvocadoDB's deterministic retrieval under the hood. It doesn't require
    external embeddings since AvocadoDB uses its own Rust-based embeddings.

    Note: AvocadoDB doesn't store vectors directly - it computes them
    on-demand using its blazing-fast Rust implementation.

    Example:
        >>> from langchain_avocadodb import AvocadoDBVectorStore
        >>> from langchain.chains import ConversationalRetrievalChain
        >>> from langchain_openai import ChatOpenAI
        >>>
        >>> # Initialize as vector store
        >>> vectorstore = AvocadoDBVectorStore(
        ...     url="http://localhost:8765",
        ...     budget=8000
        ... )
        >>>
        >>> # Use as retriever in chains
        >>> retriever = vectorstore.as_retriever(
        ...     search_kwargs={"k": 5}
        ... )
        >>>
        >>> chain = ConversationalRetrievalChain.from_llm(
        ...     llm=ChatOpenAI(),
        ...     retriever=retriever
        ... )
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
        embedding: Optional[Embeddings] = None,  # Ignored, for compatibility
        **kwargs,
    ):
        """
        Initialize AvocadoDB VectorStore.

        Args:
            url: AvocadoDB server URL
            mode: Connection mode ('http' or 'cli')
            budget: Token budget for retrieval
            semantic_weight: Weight for semantic search
            lexical_weight: Weight for lexical search
            mmr_lambda: MMR diversity parameter
            enable_mmr: Enable MMR diversification
            embedding: Ignored - AvocadoDB uses internal embeddings
            **kwargs: Additional arguments passed to retriever
        """
        if embedding is not None:
            logger.info(
                "AvocadoDB uses its own Rust-based embeddings. "
                "The provided embedding model will be ignored."
            )

        # Create underlying retriever
        self._retriever = AvocadoDBRetriever(
            url=url,
            mode=mode,
            budget=budget,
            semantic_weight=semantic_weight,
            lexical_weight=lexical_weight,
            mmr_lambda=mmr_lambda,
            enable_mmr=enable_mmr,
            **kwargs,
        )

        # Store config for cloning
        self._config = {
            "url": url,
            "mode": mode,
            "budget": budget,
            "semantic_weight": semantic_weight,
            "lexical_weight": lexical_weight,
            "mmr_lambda": mmr_lambda,
            "enable_mmr": enable_mmr,
            **kwargs,
        }

    @property
    def embeddings(self) -> Optional[Embeddings]:
        """
        Return embeddings model.

        Returns None since AvocadoDB uses internal embeddings.
        """
        return None

    def add_texts(
        self, texts: List[str], metadatas: Optional[List[dict]] = None, **kwargs
    ) -> List[str]:
        """
        Add texts to the database.

        Note: AvocadoDB requires ingestion through its native client.
        This method logs a warning and returns empty list.

        To ingest documents, use:
        - AvocadoDB CLI: `avocado ingest <path> --recursive`
        - Python client: `client.ingest(path, content)`
        """
        logger.warning(
            "AvocadoDB requires document ingestion through its native client. "
            "Use 'avocado ingest' CLI or client.ingest() method. "
            "See: https://github.com/avocadodb/avocadodb#ingestion"
        )
        return []

    def add_documents(self, documents: List[Document], **kwargs) -> List[str]:
        """
        Add documents to the database.

        Note: AvocadoDB requires ingestion through its native client.
        """
        texts = [doc.page_content for doc in documents]
        metadatas = [doc.metadata for doc in documents]
        return self.add_texts(texts, metadatas, **kwargs)

    def similarity_search(self, query: str, k: int = 4, **kwargs) -> List[Document]:
        """
        Search for similar documents.

        Args:
            query: Search query
            k: Number of documents to return (maps to token budget)
            **kwargs: Additional search parameters

        Returns:
            List of similar documents
        """
        # Map k to reasonable token budget (roughly 2000 tokens per doc)
        budget = kwargs.get("budget", k * 2000)

        # Update retriever budget temporarily
        original_budget = self._retriever.budget
        self._retriever.budget = budget

        try:
            documents = self._retriever.get_relevant_documents(query)
            # Return top k documents
            return documents[:k]
        finally:
            # Restore original budget
            self._retriever.budget = original_budget

    async def asimilarity_search(
        self, query: str, k: int = 4, **kwargs
    ) -> List[Document]:
        """
        Async search for similar documents.
        """
        # Map k to reasonable token budget
        budget = kwargs.get("budget", k * 2000)

        # Update retriever budget temporarily
        original_budget = self._retriever.budget
        self._retriever.budget = budget

        try:
            documents = await self._retriever.aget_relevant_documents(query)
            return documents[:k]
        finally:
            self._retriever.budget = original_budget

    def similarity_search_with_score(
        self, query: str, k: int = 4, **kwargs
    ) -> List[Tuple[Document, float]]:
        """
        Search with relevance scores.

        Returns:
            List of (document, score) tuples
        """
        documents = self.similarity_search(query, k, **kwargs)

        # Extract scores from metadata
        results = []
        for doc in documents:
            score = doc.metadata.get("score", 0.0)
            results.append((doc, score))

        return results

    def similarity_search_by_vector(
        self, embedding: List[float], k: int = 4, **kwargs
    ) -> List[Document]:
        """
        Search by embedding vector.

        Note: AvocadoDB doesn't support direct vector search.
        This method raises NotImplementedError.
        """
        raise NotImplementedError(
            "AvocadoDB doesn't support direct vector search. "
            "Use similarity_search() with a text query instead."
        )

    def max_marginal_relevance_search(
        self,
        query: str,
        k: int = 4,
        fetch_k: int = 20,
        lambda_mult: float = 0.5,
        **kwargs,
    ) -> List[Document]:
        """
        Search with Maximal Marginal Relevance.

        AvocadoDB has built-in MMR support via the mmr_lambda parameter.
        """
        # Update MMR settings
        original_lambda = self._retriever.mmr_lambda
        original_enable = self._retriever.enable_mmr

        self._retriever.mmr_lambda = lambda_mult
        self._retriever.enable_mmr = True

        try:
            # Use higher budget to fetch more candidates
            budget = kwargs.get("budget", fetch_k * 1000)
            documents = self.similarity_search(query, k=k, budget=budget)
            return documents
        finally:
            # Restore original settings
            self._retriever.mmr_lambda = original_lambda
            self._retriever.enable_mmr = original_enable

    def as_retriever(
        self,
        search_type: str = "similarity",
        search_kwargs: Optional[Dict[str, Any]] = None,
        **kwargs,
    ) -> AvocadoDBRetriever:
        """
        Return the underlying AvocadoDBRetriever.

        Args:
            search_type: Type of search ("similarity" or "mmr")
            search_kwargs: Additional search parameters
            **kwargs: Additional retriever parameters

        Returns:
            AvocadoDBRetriever instance
        """
        search_kwargs = search_kwargs or {}

        # Handle different search types
        if search_type == "mmr":
            # Enable MMR for this retriever
            config = self._config.copy()
            config["enable_mmr"] = True
            config["mmr_lambda"] = search_kwargs.get("lambda_mult", 0.5)
            config.update(kwargs)
            return AvocadoDBRetriever(**config)
        elif search_type == "similarity":
            # Standard similarity search
            if "k" in search_kwargs:
                # Map k to budget
                config = self._config.copy()
                config["budget"] = search_kwargs.get("k", 4) * 2000
                config.update(kwargs)
                return AvocadoDBRetriever(**config)
            else:
                # Return existing retriever
                return self._retriever
        else:
            raise ValueError(f"Unsupported search type: {search_type}")

    @classmethod
    def from_texts(
        cls,
        texts: List[str],
        embedding: Optional[Embeddings] = None,
        metadatas: Optional[List[dict]] = None,
        **kwargs,
    ) -> "AvocadoDBVectorStore":
        """
        Create VectorStore from texts.

        Note: AvocadoDB requires separate ingestion.
        This creates an empty store and logs instructions.
        """
        logger.warning(
            "AvocadoDB requires document ingestion through its native client.\n"
            "To ingest your texts:\n"
            "1. Save texts to files\n"
            "2. Run: avocado ingest <directory> --recursive\n"
            "Or use the Python client:\n"
            "   client = AvocadoDB()\n"
            "   for i, text in enumerate(texts):\n"
            "       client.ingest(f'doc_{i}.txt', text)"
        )
        return cls(**kwargs)

    @classmethod
    def from_documents(
        cls, documents: List[Document], embedding: Optional[Embeddings] = None, **kwargs
    ) -> "AvocadoDBVectorStore":
        """
        Create VectorStore from documents.

        Note: AvocadoDB requires separate ingestion.
        """
        texts = [doc.page_content for doc in documents]
        metadatas = [doc.metadata for doc in documents]
        return cls.from_texts(texts, embedding, metadatas, **kwargs)

    def delete(self, ids: Optional[List[str]] = None, **kwargs) -> Optional[bool]:
        """
        Delete documents by ID.

        Note: AvocadoDB doesn't support deletion through this interface.
        Manage your database using the native client.
        """
        logger.warning(
            "AvocadoDB doesn't support deletion through VectorStore interface. "
            "Manage your database using the native client or CLI."
        )
        return False
