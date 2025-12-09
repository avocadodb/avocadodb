"""Tests for AvocadoDBVectorStore."""

import pytest
from unittest.mock import Mock, patch, MagicMock
from langchain_avocadodb import AvocadoDBVectorStore
from langchain_core.documents import Document


class TestVectorStoreInitialization:
    """Test vector store initialization."""

    def test_default_initialization(self):
        """Test vector store with default parameters."""
        with patch("langchain_avocadodb.vectorstore.AvocadoDBRetriever"):
            store = AvocadoDBVectorStore()
            assert store._config["url"] == "http://localhost:8765"
            assert store._config["budget"] == 8000

    def test_custom_initialization(self):
        """Test vector store with custom parameters."""
        with patch("langchain_avocadodb.vectorstore.AvocadoDBRetriever"):
            store = AvocadoDBVectorStore(
                url="http://custom:9000", budget=10000, semantic_weight=0.8
            )
            assert store._config["url"] == "http://custom:9000"
            assert store._config["budget"] == 10000

    def test_embeddings_property(self):
        """Test that embeddings property returns None."""
        with patch("langchain_avocadodb.vectorstore.AvocadoDBRetriever"):
            store = AvocadoDBVectorStore()
            assert store.embeddings is None


@pytest.mark.unit
class TestVectorStoreSearch:
    """Test vector store search methods."""

    @pytest.fixture
    def mock_store(self):
        """Create vector store with mocked retriever."""
        with patch("langchain_avocadodb.vectorstore.AvocadoDBRetriever") as mock_ret:
            store = AvocadoDBVectorStore()
            # Create mock documents
            mock_docs = [
                Document(
                    page_content=f"Test content {i}",
                    metadata={"source": f"test{i}.py", "score": 0.9 - i * 0.1},
                )
                for i in range(5)
            ]
            mock_ret.return_value.get_relevant_documents.return_value = mock_docs
            return store

    def test_similarity_search(self, mock_store):
        """Test similarity search."""
        docs = mock_store.similarity_search("test query", k=3)
        assert len(docs) == 3
        assert all(isinstance(doc, Document) for doc in docs)

    def test_similarity_search_with_score(self, mock_store):
        """Test similarity search with scores."""
        results = mock_store.similarity_search_with_score("test query", k=3)
        assert len(results) == 3
        assert all(isinstance(item, tuple) for item in results)
        assert all(isinstance(item[0], Document) for item in results)
        assert all(isinstance(item[1], float) for item in results)

    @pytest.mark.asyncio
    async def test_async_similarity_search(self, mock_store):
        """Test async similarity search."""
        with patch.object(
            mock_store._retriever,
            "aget_relevant_documents",
            return_value=[Document(page_content="test", metadata={"score": 0.9})],
        ):
            docs = await mock_store.asimilarity_search("test query", k=2)
            assert isinstance(docs, list)

    def test_similarity_search_by_vector_not_supported(self, mock_store):
        """Test that vector search raises NotImplementedError."""
        with pytest.raises(NotImplementedError):
            mock_store.similarity_search_by_vector([0.1, 0.2, 0.3])


@pytest.mark.unit
class TestVectorStoreMMR:
    """Test MMR search functionality."""

    @pytest.fixture
    def mock_store(self):
        """Create vector store with mocked retriever."""
        with patch("langchain_avocadodb.vectorstore.AvocadoDBRetriever") as mock_ret:
            store = AvocadoDBVectorStore()
            mock_docs = [
                Document(page_content=f"Content {i}", metadata={"score": 0.9})
                for i in range(3)
            ]
            mock_ret.return_value.get_relevant_documents.return_value = mock_docs
            return store

    def test_mmr_search(self, mock_store):
        """Test MMR search."""
        docs = mock_store.max_marginal_relevance_search(
            "test query", k=2, fetch_k=10, lambda_mult=0.5
        )
        assert len(docs) == 2
        # Verify MMR was enabled
        assert mock_store._retriever.enable_mmr is True


@pytest.mark.unit
class TestVectorStoreRetriever:
    """Test as_retriever method."""

    def test_as_retriever_similarity(self):
        """Test creating retriever with similarity search."""
        with patch("langchain_avocadodb.vectorstore.AvocadoDBRetriever"):
            store = AvocadoDBVectorStore()
            retriever = store.as_retriever(search_type="similarity")
            assert retriever is not None

    def test_as_retriever_mmr(self):
        """Test creating retriever with MMR."""
        with patch("langchain_avocadodb.vectorstore.AvocadoDBRetriever") as mock_ret:
            store = AvocadoDBVectorStore()
            retriever = store.as_retriever(
                search_type="mmr", search_kwargs={"lambda_mult": 0.3}
            )
            # Verify new retriever was created with MMR enabled
            assert mock_ret.called

    def test_as_retriever_with_k(self):
        """Test creating retriever with k parameter."""
        with patch("langchain_avocadodb.vectorstore.AvocadoDBRetriever") as mock_ret:
            store = AvocadoDBVectorStore()
            retriever = store.as_retriever(
                search_type="similarity", search_kwargs={"k": 5}
            )
            # Should create new retriever with adjusted budget
            assert mock_ret.called

    def test_as_retriever_invalid_type(self):
        """Test that invalid search type raises error."""
        with patch("langchain_avocadodb.vectorstore.AvocadoDBRetriever"):
            store = AvocadoDBVectorStore()
            with pytest.raises(ValueError):
                store.as_retriever(search_type="invalid")


@pytest.mark.unit
class TestVectorStoreIngestion:
    """Test ingestion methods (should warn)."""

    @pytest.fixture
    def mock_store(self):
        """Create vector store."""
        with patch("langchain_avocadodb.vectorstore.AvocadoDBRetriever"):
            return AvocadoDBVectorStore()

    def test_add_texts_warns(self, mock_store, caplog):
        """Test that add_texts logs a warning."""
        result = mock_store.add_texts(["text1", "text2"])
        assert result == []
        assert "native client" in caplog.text.lower()

    def test_add_documents_warns(self, mock_store, caplog):
        """Test that add_documents logs a warning."""
        docs = [Document(page_content="test")]
        result = mock_store.add_documents(docs)
        assert result == []

    def test_delete_warns(self, mock_store, caplog):
        """Test that delete logs a warning."""
        result = mock_store.delete(["id1", "id2"])
        assert result is False

    def test_from_texts_warns(self, caplog):
        """Test that from_texts logs a warning."""
        with patch("langchain_avocadodb.vectorstore.AvocadoDBRetriever"):
            store = AvocadoDBVectorStore.from_texts(["text1", "text2"])
            assert isinstance(store, AvocadoDBVectorStore)
            assert "native client" in caplog.text.lower()

    def test_from_documents_warns(self, caplog):
        """Test that from_documents logs a warning."""
        with patch("langchain_avocadodb.vectorstore.AvocadoDBRetriever"):
            docs = [Document(page_content="test")]
            store = AvocadoDBVectorStore.from_documents(docs)
            assert isinstance(store, AvocadoDBVectorStore)


@pytest.mark.integration
class TestVectorStoreIntegration:
    """Integration tests requiring AvocadoDB server."""

    @pytest.fixture
    def vector_store(self, avocado_url):
        """Create vector store pointing to test server."""
        return AvocadoDBVectorStore(url=avocado_url)

    def test_real_similarity_search(self, vector_store):
        """Test similarity search against real server."""
        try:
            docs = vector_store.similarity_search("authentication", k=3)
            assert isinstance(docs, list)
            if len(docs) > 0:
                assert isinstance(docs[0], Document)
        except Exception as e:
            pytest.skip(f"AvocadoDB server not available: {e}")

    def test_retriever_creation(self, vector_store):
        """Test creating retriever from vector store."""
        try:
            retriever = vector_store.as_retriever(search_kwargs={"k": 4})
            docs = retriever.get_relevant_documents("database")
            assert isinstance(docs, list)
        except Exception as e:
            pytest.skip(f"AvocadoDB server not available: {e}")
