"""
Comprehensive integration tests for AvocadoDBReader with LlamaIndex.

Tests cover:
- Basic reader functionality
- Document loading and metadata
- Batch operations
- Lazy loading
- Node conversion
- LlamaIndex integration (indexes, query engines)
- Error handling
- Configuration options
"""

import pytest
from unittest.mock import Mock, patch, MagicMock
from typing import List

from llama_index_avocadodb import AvocadoDBReader
from llama_index.core.schema import Document, TextNode
from llama_index.core import VectorStoreIndex


# Mock AvocadoDB classes
class MockSpan:
    """Mock span object from AvocadoDB."""

    def __init__(self, id: str, text: str, artifact_path: str,
                 start_line: int, end_line: int, token_count: int,
                 artifact_id: str, score: float = 0.8):
        self.id = id
        self.text = text
        self.artifact_path = artifact_path
        self.start_line = start_line
        self.end_line = end_line
        self.token_count = token_count
        self.artifact_id = artifact_id
        self.score = score


class MockCitation:
    """Mock citation object from AvocadoDB."""

    def __init__(self, span_id: str, artifact_path: str,
                 start_line: int, end_line: int, score: float):
        self.span_id = span_id
        self.artifact_path = artifact_path
        self.start_line = start_line
        self.end_line = end_line
        self.score = score


class MockWorkingSet:
    """Mock working set from AvocadoDB."""

    def __init__(self, query: str, spans: List[MockSpan],
                 citations: List[MockCitation] = None,
                 tokens_used: int = 500,
                 compilation_time_ms: int = 50):
        self.query = query
        self.spans = spans
        self.citations = citations or []
        self.tokens_used = tokens_used
        self.compilation_time_ms = compilation_time_ms

    def deterministic_hash(self) -> str:
        return "abc123def456"


# Fixtures
@pytest.fixture
def mock_avocadodb():
    """Create a mock AvocadoDB client."""
    with patch('llama_index_avocadodb.reader.AvocadoDB') as mock:
        client = Mock()
        mock.return_value = client
        yield client


@pytest.fixture
def sample_spans():
    """Create sample spans for testing."""
    return [
        MockSpan(
            id="span1",
            text="def authenticate(user, password):\n    return verify_credentials(user, password)",
            artifact_path="auth.py",
            start_line=10,
            end_line=11,
            token_count=20,
            artifact_id="art1",
            score=0.95
        ),
        MockSpan(
            id="span2",
            text="class AuthenticationService:\n    def __init__(self):\n        self.db = Database()",
            artifact_path="auth_service.py",
            start_line=5,
            end_line=7,
            token_count=25,
            artifact_id="art2",
            score=0.85
        ),
        MockSpan(
            id="span3",
            text="JWT_SECRET = 'your-secret-key'\nJWT_ALGORITHM = 'HS256'",
            artifact_path="config.py",
            start_line=15,
            end_line=16,
            token_count=15,
            artifact_id="art3",
            score=0.75
        )
    ]


@pytest.fixture
def sample_citations():
    """Create sample citations for testing."""
    return [
        MockCitation(
            span_id="span1",
            artifact_path="auth.py",
            start_line=10,
            end_line=11,
            score=0.95
        ),
        MockCitation(
            span_id="span1",
            artifact_path="utils.py",
            start_line=50,
            end_line=52,
            score=0.80
        )
    ]


@pytest.fixture
def sample_working_set(sample_spans, sample_citations):
    """Create a sample working set."""
    return MockWorkingSet(
        query="authentication implementation",
        spans=sample_spans,
        citations=sample_citations,
        tokens_used=60,
        compilation_time_ms=45
    )


# Test Class
class TestAvocadoDBReader:
    """Test suite for AvocadoDBReader."""

    def test_initialization_default(self, mock_avocadodb):
        """Test reader initialization with default parameters."""
        reader = AvocadoDBReader()

        assert reader.budget == 8000
        assert reader.semantic_weight == 0.7
        assert reader.lexical_weight == 0.3
        assert reader.mmr_lambda == 0.5
        assert reader.enable_mmr is True
        assert reader.include_citations is True
        assert reader.include_scores is True
        assert reader.create_nodes is False
        assert reader.combine_adjacent is False
        assert reader.min_score is None

    def test_initialization_custom(self, mock_avocadodb):
        """Test reader initialization with custom parameters."""
        reader = AvocadoDBReader(
            url="http://localhost:9000",
            budget=10000,
            semantic_weight=0.8,
            lexical_weight=0.2,
            mmr_lambda=0.7,
            enable_mmr=False,
            include_citations=False,
            include_scores=False,
            create_nodes=True,
            combine_adjacent=True,
            min_score=0.6
        )

        assert reader.budget == 10000
        assert reader.semantic_weight == 0.8
        assert reader.lexical_weight == 0.2
        assert reader.mmr_lambda == 0.7
        assert reader.enable_mmr is False
        assert reader.include_citations is False
        assert reader.include_scores is False
        assert reader.create_nodes is True
        assert reader.combine_adjacent is True
        assert reader.min_score == 0.6

    def test_load_data_basic(self, mock_avocadodb, sample_working_set):
        """Test basic document loading."""
        mock_avocadodb.compile.return_value = sample_working_set

        reader = AvocadoDBReader()
        reader.client = mock_avocadodb
        documents = reader.load_data("authentication")

        # Verify compile was called correctly
        mock_avocadodb.compile.assert_called_once()
        call_kwargs = mock_avocadodb.compile.call_args[1]
        assert call_kwargs['query'] == "authentication"
        assert call_kwargs['budget'] == 8000

        # Verify documents
        assert len(documents) == 3
        assert all(isinstance(doc, Document) for doc in documents)

        # Verify first document
        doc = documents[0]
        assert doc.text == sample_working_set.spans[0].text
        assert doc.metadata['file_path'] == "auth.py"
        assert doc.metadata['start_line'] == 10
        assert doc.metadata['end_line'] == 11
        assert doc.metadata['score'] == 0.95
        assert doc.id_ == "span1"

    def test_load_data_with_custom_budget(self, mock_avocadodb, sample_working_set):
        """Test loading with custom budget override."""
        mock_avocadodb.compile.return_value = sample_working_set

        reader = AvocadoDBReader(budget=8000)
        reader.client = mock_avocadodb
        documents = reader.load_data("test query", budget=12000)

        call_kwargs = mock_avocadodb.compile.call_args[1]
        assert call_kwargs['budget'] == 12000

    def test_load_data_with_citations(self, mock_avocadodb, sample_working_set):
        """Test document loading with citations included."""
        mock_avocadodb.compile.return_value = sample_working_set

        reader = AvocadoDBReader(include_citations=True)
        reader.client = mock_avocadodb
        documents = reader.load_data("test")

        # First span should have citations
        doc = documents[0]
        assert 'citations' in doc.metadata
        assert len(doc.metadata['citations']) == 2

        citation = doc.metadata['citations'][0]
        assert citation['file'] == "auth.py"
        assert citation['start_line'] == 10
        assert citation['score'] == 0.95

    def test_load_data_without_citations(self, mock_avocadodb, sample_working_set):
        """Test document loading without citations."""
        mock_avocadodb.compile.return_value = sample_working_set

        reader = AvocadoDBReader(include_citations=False)
        reader.client = mock_avocadodb
        documents = reader.load_data("test")

        # Citations should not be in metadata
        for doc in documents:
            assert 'citations' not in doc.metadata

    def test_load_data_with_score_filter(self, mock_avocadodb, sample_working_set):
        """Test filtering documents by minimum score."""
        mock_avocadodb.compile.return_value = sample_working_set

        reader = AvocadoDBReader(min_score=0.8)
        reader.client = mock_avocadodb
        documents = reader.load_data("test")

        # Only spans with score >= 0.8 should be included
        assert len(documents) == 2  # span1 (0.95) and span2 (0.85)
        assert all(doc.metadata['score'] >= 0.8 for doc in documents)

    def test_load_data_empty_result(self, mock_avocadodb):
        """Test handling of empty results."""
        empty_working_set = MockWorkingSet(query="test", spans=[])
        mock_avocadodb.compile.return_value = empty_working_set

        reader = AvocadoDBReader()
        reader.client = mock_avocadodb
        documents = reader.load_data("test")

        assert len(documents) == 0

    def test_load_data_error_handling(self, mock_avocadodb):
        """Test error handling when compilation fails."""
        mock_avocadodb.compile.side_effect = Exception("Connection error")

        reader = AvocadoDBReader()
        reader.client = mock_avocadodb
        documents = reader.load_data("test")

        # Should return empty list on error
        assert len(documents) == 0

    def test_load_data_batch(self, mock_avocadodb, sample_working_set):
        """Test batch loading of multiple queries."""
        mock_avocadodb.compile.return_value = sample_working_set

        reader = AvocadoDBReader()
        reader.client = mock_avocadodb

        queries = ["query1", "query2", "query3"]
        results = reader.load_data_batch(queries, budget=5000)

        # Verify results structure
        assert len(results) == 3
        assert all(isinstance(docs, list) for docs in results)
        assert all(len(docs) == 3 for docs in results)

        # Verify compile was called for each query
        assert mock_avocadodb.compile.call_count == 3

    def test_convert_to_nodes(self, mock_avocadodb, sample_working_set):
        """Test conversion of documents to TextNodes."""
        mock_avocadodb.compile.return_value = sample_working_set

        reader = AvocadoDBReader(create_nodes=True)
        reader.client = mock_avocadodb
        nodes = reader.load_data("test")

        # Verify nodes
        assert len(nodes) == 3
        assert all(isinstance(node, TextNode) for node in nodes)

        # Verify node properties
        node = nodes[0]
        assert node.text == sample_working_set.spans[0].text
        assert node.metadata['file_path'] == "auth.py"
        assert node.id_ == "span1"

    def test_combine_adjacent_spans(self, mock_avocadodb):
        """Test combining adjacent spans from the same file."""
        # Create adjacent spans
        adjacent_spans = [
            MockSpan("span1", "line 1", "file.py", 10, 10, 10, "art1", 0.9),
            MockSpan("span2", "line 2", "file.py", 12, 12, 10, "art1", 0.85),
            MockSpan("span3", "line 3", "file.py", 14, 14, 10, "art1", 0.8),
            MockSpan("span4", "other", "other.py", 20, 20, 10, "art2", 0.75),
        ]
        working_set = MockWorkingSet("test", adjacent_spans, [])
        mock_avocadodb.compile.return_value = working_set

        reader = AvocadoDBReader(combine_adjacent=True)
        reader.client = mock_avocadodb
        documents = reader.load_data("test")

        # Should combine first 3 spans into one document
        assert len(documents) == 2

        # First document should have combined text
        combined_doc = documents[0]
        assert "line 1" in combined_doc.text
        assert "line 2" in combined_doc.text
        assert "line 3" in combined_doc.text
        assert combined_doc.metadata['span_count'] == 3
        assert combined_doc.metadata['start_line'] == 10
        assert combined_doc.metadata['end_line'] == 14

    def test_lazy_load_data(self, mock_avocadodb, sample_working_set):
        """Test lazy loading (generator) functionality."""
        mock_avocadodb.compile.return_value = sample_working_set

        reader = AvocadoDBReader()
        reader.client = mock_avocadodb

        documents = list(reader.lazy_load_data("test"))

        assert len(documents) == 3
        assert all(isinstance(doc, Document) for doc in documents)

    def test_lazy_load_with_score_filter(self, mock_avocadodb, sample_working_set):
        """Test lazy loading with score filtering."""
        mock_avocadodb.compile.return_value = sample_working_set

        reader = AvocadoDBReader(min_score=0.8)
        reader.client = mock_avocadodb

        documents = list(reader.lazy_load_data("test"))

        # Only high-scoring documents
        assert len(documents) == 2
        assert all(doc.metadata['score'] >= 0.8 for doc in documents)

    def test_metadata_completeness(self, mock_avocadodb, sample_working_set):
        """Test that all expected metadata is present."""
        mock_avocadodb.compile.return_value = sample_working_set

        reader = AvocadoDBReader(include_citations=True, include_scores=True)
        reader.client = mock_avocadodb
        documents = reader.load_data("test")

        doc = documents[0]
        metadata = doc.metadata

        # Required metadata fields
        assert 'file_path' in metadata
        assert 'start_line' in metadata
        assert 'end_line' in metadata
        assert 'token_count' in metadata
        assert 'span_id' in metadata
        assert 'artifact_id' in metadata
        assert 'query' in metadata
        assert 'deterministic_hash' in metadata
        assert 'compilation_time_ms' in metadata
        assert 'score' in metadata
        assert 'citations' in metadata

    def test_deterministic_hash_presence(self, mock_avocadodb, sample_working_set):
        """Test that deterministic hash is included in metadata."""
        mock_avocadodb.compile.return_value = sample_working_set

        reader = AvocadoDBReader()
        reader.client = mock_avocadodb
        documents = reader.load_data("test")

        for doc in documents:
            assert 'deterministic_hash' in doc.metadata
            assert doc.metadata['deterministic_hash'] == "abc123def456"


class TestLlamaIndexIntegration:
    """Test integration with LlamaIndex components."""

    @pytest.mark.skipif(
        True,
        reason="Requires llama-index-embeddings-openai package"
    )
    def test_create_vector_index(self, mock_avocadodb, sample_working_set):
        """Test creating a VectorStoreIndex from loaded documents."""
        mock_avocadodb.compile.return_value = sample_working_set

        reader = AvocadoDBReader()
        reader.client = mock_avocadodb
        documents = reader.load_data("test")

        # This should not raise an error
        index = VectorStoreIndex.from_documents(documents)
        assert index is not None

    @pytest.mark.skipif(
        True,
        reason="Requires llama-index-embeddings-openai package"
    )
    def test_create_index_from_nodes(self, mock_avocadodb, sample_working_set):
        """Test creating index from TextNodes."""
        mock_avocadodb.compile.return_value = sample_working_set

        reader = AvocadoDBReader(create_nodes=True)
        reader.client = mock_avocadodb
        nodes = reader.load_data("test")

        # Create index from nodes
        index = VectorStoreIndex(nodes)
        assert index is not None

    @pytest.mark.integration
    @pytest.mark.skipif(
        True,
        reason="Requires llama-index-embeddings-openai package"
    )
    def test_query_engine_basic(self, mock_avocadodb, sample_working_set):
        """Test basic query engine functionality."""
        mock_avocadodb.compile.return_value = sample_working_set

        reader = AvocadoDBReader()
        reader.client = mock_avocadodb
        documents = reader.load_data("authentication")

        # Create index and query engine
        index = VectorStoreIndex.from_documents(documents)
        query_engine = index.as_query_engine()

        assert query_engine is not None

    @pytest.mark.integration
    @pytest.mark.skipif(
        True,
        reason="Requires llama-index-embeddings-openai package"
    )
    def test_retriever_creation(self, mock_avocadodb, sample_working_set):
        """Test creating a retriever from the index."""
        mock_avocadodb.compile.return_value = sample_working_set

        reader = AvocadoDBReader()
        reader.client = mock_avocadodb
        documents = reader.load_data("test")

        index = VectorStoreIndex.from_documents(documents)
        retriever = index.as_retriever(similarity_top_k=3)

        assert retriever is not None


class TestEdgeCases:
    """Test edge cases and error conditions."""

    def test_should_combine_same_file_adjacent_lines(self):
        """Test span combination logic for adjacent lines."""
        reader = AvocadoDBReader()

        span1 = MockSpan("s1", "text", "file.py", 10, 12, 10, "a1")
        span2 = MockSpan("s2", "text", "file.py", 13, 15, 10, "a1")

        # Lines 12 and 13 are adjacent (gap = 1)
        assert reader._should_combine(span1, span2) is True

    def test_should_not_combine_different_files(self):
        """Test that spans from different files are not combined."""
        reader = AvocadoDBReader()

        span1 = MockSpan("s1", "text", "file1.py", 10, 12, 10, "a1")
        span2 = MockSpan("s2", "text", "file2.py", 13, 15, 10, "a2")

        assert reader._should_combine(span1, span2) is False

    def test_should_not_combine_distant_lines(self):
        """Test that distant spans are not combined."""
        reader = AvocadoDBReader()

        span1 = MockSpan("s1", "text", "file.py", 10, 12, 10, "a1")
        span2 = MockSpan("s2", "text", "file.py", 20, 22, 10, "a1")

        # Gap of 8 lines is too large
        assert reader._should_combine(span1, span2) is False

    def test_combined_document_metadata_aggregation(self, mock_avocadodb):
        """Test metadata aggregation in combined documents."""
        spans = [
            MockSpan("s1", "text1", "file.py", 10, 11, 10, "a1", 0.9),
            MockSpan("s2", "text2", "file.py", 12, 13, 15, "a1", 0.8),
        ]
        working_set = MockWorkingSet("test", spans, [])
        mock_avocadodb.compile.return_value = working_set

        reader = AvocadoDBReader(combine_adjacent=True, include_scores=True)
        reader.client = mock_avocadodb
        documents = reader.load_data("test")

        doc = documents[0]
        assert doc.metadata['span_count'] == 2
        assert doc.metadata['token_count'] == 25  # 10 + 15
        assert abs(doc.metadata['avg_score'] - 0.85) < 0.001  # Floating point comparison
        assert doc.metadata['max_score'] == 0.9
        assert doc.metadata['min_score'] == 0.8

    def test_empty_query_string(self, mock_avocadodb, sample_working_set):
        """Test handling of empty query string."""
        mock_avocadodb.compile.return_value = sample_working_set

        reader = AvocadoDBReader()
        reader.client = mock_avocadodb
        documents = reader.load_data("")

        # Should still call compile
        mock_avocadodb.compile.assert_called_once()

    def test_very_large_budget(self, mock_avocadodb, sample_working_set):
        """Test handling of very large token budgets."""
        mock_avocadodb.compile.return_value = sample_working_set

        reader = AvocadoDBReader()
        reader.client = mock_avocadodb
        documents = reader.load_data("test", budget=1000000)

        call_kwargs = mock_avocadodb.compile.call_args[1]
        assert call_kwargs['budget'] == 1000000

    def test_weights_normalization(self, mock_avocadodb):
        """Test that weights are passed correctly even if not normalized."""
        reader = AvocadoDBReader(
            semantic_weight=0.9,
            lexical_weight=0.5  # Sum > 1.0
        )
        reader.client = mock_avocadodb

        # Should still pass weights as-is
        assert reader.semantic_weight == 0.9
        assert reader.lexical_weight == 0.5


# Mark tests that require actual server
@pytest.mark.integration
class TestRealServerIntegration:
    """Tests that require an actual AvocadoDB server running."""

    def test_real_server_connection(self):
        """Test connection to real AvocadoDB server (requires server running)."""
        pytest.skip("Requires AvocadoDB server running")

        reader = AvocadoDBReader(url="http://localhost:8765")
        documents = reader.load_data("test query", budget=1000)

        # If server is running, should return documents
        assert isinstance(documents, list)

    def test_real_server_query_engine(self):
        """Test full pipeline with real server (requires server running)."""
        pytest.skip("Requires AvocadoDB server running")

        reader = AvocadoDBReader()
        documents = reader.load_data("authentication", budget=5000)

        index = VectorStoreIndex.from_documents(documents)
        query_engine = index.as_query_engine()
        response = query_engine.query("How does auth work?")

        assert response is not None


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
