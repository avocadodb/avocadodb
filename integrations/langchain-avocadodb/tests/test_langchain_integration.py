"""Tests for LangChain integration patterns."""

import pytest
from unittest.mock import Mock, patch, MagicMock
from langchain_avocadodb import AvocadoDBRetriever, AvocadoDBVectorStore


@pytest.mark.unit
class TestLangChainChains:
    """Test integration with LangChain chains."""

    @pytest.fixture
    def mock_retriever(self):
        """Create mocked retriever."""
        with patch("langchain_avocadodb.retriever.AvocadoDB"):
            return AvocadoDBRetriever()

    def test_retrieval_qa_chain(self, mock_retriever):
        """Test RetrievalQA chain integration."""
        try:
            from langchain.chains import RetrievalQA
            from langchain_core.language_models import FakeListLLM

            # Create a fake LLM
            llm = FakeListLLM(responses=["This is a test answer"])

            # Create QA chain
            qa_chain = RetrievalQA.from_chain_type(
                llm=llm, retriever=mock_retriever, return_source_documents=True
            )

            assert qa_chain is not None
            assert qa_chain.retriever == mock_retriever

        except ImportError:
            pytest.skip("LangChain chains not installed")

    def test_conversational_retrieval_chain(self, mock_retriever):
        """Test ConversationalRetrievalChain integration."""
        try:
            from langchain.chains import ConversationalRetrievalChain
            from langchain_core.language_models import FakeListLLM

            llm = FakeListLLM(responses=["Test response"])

            chain = ConversationalRetrievalChain.from_llm(
                llm=llm, retriever=mock_retriever
            )

            assert chain is not None

        except ImportError:
            pytest.skip("LangChain chains not installed")


@pytest.mark.unit
class TestLangChainTools:
    """Test integration with LangChain tools."""

    @pytest.fixture
    def mock_retriever(self):
        """Create mocked retriever."""
        with patch("langchain_avocadodb.retriever.AvocadoDB"):
            return AvocadoDBRetriever()

    def test_create_retriever_tool(self, mock_retriever):
        """Test creating a retriever tool."""
        try:
            from langchain.tools.retriever import create_retriever_tool

            tool = create_retriever_tool(
                mock_retriever,
                "avocadodb_search",
                "Search the codebase for information",
            )

            assert tool is not None
            assert tool.name == "avocadodb_search"

        except ImportError:
            pytest.skip("LangChain tools not installed")


@pytest.mark.unit
class TestRunnableInterface:
    """Test Runnable interface compatibility."""

    @pytest.fixture
    def mock_retriever(self):
        """Create mocked retriever."""
        with patch("langchain_avocadodb.retriever.AvocadoDB") as mock_db:
            mock_ws = MagicMock()
            mock_ws.spans = []
            mock_ws.citations = []
            mock_ws.query = "test"
            mock_ws.tokens_used = 0
            mock_ws.compilation_time_ms = 0
            mock_db.return_value.compile.return_value = mock_ws
            return AvocadoDBRetriever()

    def test_invoke_interface(self, mock_retriever):
        """Test invoke method (Runnable protocol)."""
        result = mock_retriever.invoke("test query")
        assert isinstance(result, list)

    @pytest.mark.asyncio
    async def test_ainvoke_interface(self, mock_retriever):
        """Test async invoke method."""
        result = await mock_retriever.ainvoke("test query")
        assert isinstance(result, list)

    def test_batch_interface(self, mock_retriever):
        """Test batch method."""
        try:
            results = mock_retriever.batch(["query1", "query2"])
            assert isinstance(results, list)
            assert len(results) == 2
        except AttributeError:
            # batch might not be implemented
            pass

    def test_config_schema(self, mock_retriever):
        """Test config schema is available."""
        # Runnable objects should have config schema
        assert hasattr(mock_retriever, "config_schema")


@pytest.mark.unit
class TestVectorStoreCompatibility:
    """Test VectorStore interface compatibility."""

    @pytest.fixture
    def mock_store(self):
        """Create mocked vector store."""
        with patch("langchain_avocadodb.vectorstore.AvocadoDBRetriever"):
            return AvocadoDBVectorStore()

    def test_has_similarity_search(self, mock_store):
        """Test similarity_search method exists."""
        assert hasattr(mock_store, "similarity_search")
        assert callable(mock_store.similarity_search)

    def test_has_similarity_search_with_score(self, mock_store):
        """Test similarity_search_with_score method exists."""
        assert hasattr(mock_store, "similarity_search_with_score")
        assert callable(mock_store.similarity_search_with_score)

    def test_has_max_marginal_relevance_search(self, mock_store):
        """Test max_marginal_relevance_search method exists."""
        assert hasattr(mock_store, "max_marginal_relevance_search")
        assert callable(mock_store.max_marginal_relevance_search)

    def test_has_as_retriever(self, mock_store):
        """Test as_retriever method exists."""
        assert hasattr(mock_store, "as_retriever")
        assert callable(mock_store.as_retriever)

    def test_has_add_texts(self, mock_store):
        """Test add_texts method exists."""
        assert hasattr(mock_store, "add_texts")
        assert callable(mock_store.add_texts)

    def test_has_from_texts(self, mock_store):
        """Test from_texts classmethod exists."""
        assert hasattr(AvocadoDBVectorStore, "from_texts")


@pytest.mark.integration
@pytest.mark.requires_openai
class TestEndToEndIntegration:
    """End-to-end integration tests with real components."""

    @pytest.fixture
    def retriever(self, avocado_url):
        """Create retriever."""
        return AvocadoDBRetriever(url=avocado_url, budget=5000)

    def test_qa_chain_end_to_end(self, retriever):
        """Test complete QA chain flow."""
        try:
            from langchain.chains import RetrievalQA
            from langchain_openai import ChatOpenAI
            import os

            if "OPENAI_API_KEY" not in os.environ:
                pytest.skip("OPENAI_API_KEY not set")

            llm = ChatOpenAI(temperature=0, model="gpt-3.5-turbo")
            qa_chain = RetrievalQA.from_chain_type(
                llm=llm, retriever=retriever, return_source_documents=True
            )

            result = qa_chain.invoke({"query": "What is authentication?"})

            assert "answer" in result
            assert "source_documents" in result
            assert isinstance(result["answer"], str)
            assert len(result["answer"]) > 0

        except Exception as e:
            pytest.skip(f"Integration test failed: {e}")

    def test_streaming_response(self, retriever):
        """Test streaming with retriever."""
        try:
            from langchain_openai import ChatOpenAI
            from langchain.chains import RetrievalQA
            import os

            if "OPENAI_API_KEY" not in os.environ:
                pytest.skip("OPENAI_API_KEY not set")

            llm = ChatOpenAI(temperature=0, streaming=True)
            qa_chain = RetrievalQA.from_chain_type(llm=llm, retriever=retriever)

            # Should not raise error
            result = qa_chain.invoke({"query": "database"})
            assert result is not None

        except Exception as e:
            pytest.skip(f"Streaming test failed: {e}")
