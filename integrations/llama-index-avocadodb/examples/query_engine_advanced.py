"""
Advanced Query Engine Examples with AvocadoDB.

This example demonstrates advanced query engine patterns
that leverage AvocadoDB's deterministic retrieval and
citation capabilities.

Features:
- Multiple response modes
- Custom retrieval strategies
- Citation tracking and verification
- Query engine composition
- Performance optimization
"""

from llama_index_avocadodb import AvocadoDBReader
from llama_index.core import (
    VectorStoreIndex,
    Settings,
    get_response_synthesizer
)
from llama_index.core.query_engine import RetrieverQueryEngine
from llama_index.core.retrievers import VectorIndexRetriever
from llama_index.core.postprocessor import (
    SimilarityPostprocessor,
    KeywordNodePostprocessor
)
from llama_index.llms.openai import OpenAI
from llama_index.embeddings.openai import OpenAIEmbedding
from typing import List, Dict, Any
import time


def example_response_modes():
    """
    Demonstrate different response modes with AvocadoDB.

    Response modes control how the query engine synthesizes
    answers from retrieved context.
    """
    print("="*70)
    print("Example 1: Response Modes Comparison")
    print("="*70)

    # Initialize reader
    reader = AvocadoDBReader(
        url="http://localhost:8765",
        budget=10000,
        include_citations=True
    )

    # Load documents
    query = "authentication and authorization implementation"
    print(f"\nQuery: {query}")
    documents = reader.load_data(query)
    print(f"Loaded: {len(documents)} documents\n")

    # Create index
    index = VectorStoreIndex.from_documents(documents)

    # Test different response modes
    modes = [
        ("compact", "Combine text chunks into larger context"),
        ("tree_summarize", "Build summary tree from chunks"),
        ("simple_summarize", "Truncate and summarize chunks"),
        ("no_text", "Return source nodes only"),
    ]

    question = "How is user authentication validated?"

    for mode, description in modes:
        print(f"\n{mode.upper()}")
        print(f"  Description: {description}")
        print("-" * 70)

        try:
            # Create query engine with specific mode
            query_engine = index.as_query_engine(
                response_mode=mode,
                verbose=False
            )

            # Time the query
            start = time.time()
            response = query_engine.query(question)
            elapsed = time.time() - start

            print(f"  Response ({elapsed:.2f}s):")
            print(f"  {str(response)[:200]}...")

            # Show sources
            if hasattr(response, 'source_nodes'):
                print(f"\n  Sources: {len(response.source_nodes)} nodes")

        except Exception as e:
            print(f"  Error: {e}")


def example_custom_retriever():
    """
    Create a custom retriever with post-processing.

    This example shows how to build a query engine with
    custom retrieval and filtering logic.
    """
    print("\n" + "="*70)
    print("Example 2: Custom Retriever with Post-Processing")
    print("="*70)

    reader = AvocadoDBReader(
        url="http://localhost:8765",
        budget=12000,
        include_citations=True,
        semantic_weight=0.8,
        lexical_weight=0.2
    )

    # Load documents
    documents = reader.load_data("database operations and queries")
    index = VectorStoreIndex.from_documents(documents)

    # Create custom retriever
    retriever = VectorIndexRetriever(
        index=index,
        similarity_top_k=10,
    )

    # Add post-processors for filtering
    node_postprocessors = [
        SimilarityPostprocessor(similarity_cutoff=0.7),
        KeywordNodePostprocessor(
            required_keywords=["database", "query"],
            exclude_keywords=["test", "mock"]
        )
    ]

    # Create response synthesizer
    response_synthesizer = get_response_synthesizer(
        response_mode="tree_summarize"
    )

    # Build query engine
    query_engine = RetrieverQueryEngine(
        retriever=retriever,
        response_synthesizer=response_synthesizer,
        node_postprocessors=node_postprocessors
    )

    # Test queries
    queries = [
        "How are database queries optimized?",
        "What indexing strategies are used?",
        "How is query caching implemented?"
    ]

    for query in queries:
        print(f"\nQuery: {query}")
        print("-" * 70)

        response = query_engine.query(query)
        print(f"Answer: {response}\n")

        # Show filtered sources
        if hasattr(response, 'source_nodes'):
            print(f"Sources after filtering: {len(response.source_nodes)}")
            for i, node in enumerate(response.source_nodes[:3], 1):
                print(f"  {i}. {node.metadata.get('file_path')} "
                      f"(score: {node.score:.3f})")


def example_citation_verification():
    """
    Verify and display citations for query responses.
    """
    print("\n" + "="*70)
    print("Example 3: Citation Verification")
    print("="*70)

    reader = AvocadoDBReader(
        url="http://localhost:8765",
        budget=8000,
        include_citations=True,
        include_scores=True
    )

    documents = reader.load_data("API endpoint implementation")
    print(f"Loaded {len(documents)} documents with citations\n")

    index = VectorStoreIndex.from_documents(documents)
    query_engine = index.as_query_engine(
        similarity_top_k=5,
        verbose=False
    )

    query = "How are REST API endpoints defined?"
    print(f"Query: {query}")
    print("-" * 70)

    response = query_engine.query(query)
    print(f"\nAnswer:\n{response}\n")

    print("Citations (with line-level precision):")
    print("=" * 70)

    citation_map: Dict[str, List[Dict[str, Any]]] = {}

    for node in response.source_nodes:
        metadata = node.metadata

        file_path = metadata.get('file_path', 'Unknown')
        start_line = metadata.get('start_line', 0)
        end_line = metadata.get('end_line', 0)
        score = metadata.get('score', 0)

        if file_path not in citation_map:
            citation_map[file_path] = []

        citation_map[file_path].append({
            'lines': f"{start_line}-{end_line}",
            'score': score,
            'type': 'primary'
        })

        if 'citations' in metadata:
            for citation in metadata['citations']:
                ref_file = citation['file']
                ref_lines = f"{citation['start_line']}-{citation['end_line']}"
                ref_score = citation['score']

                if ref_file not in citation_map:
                    citation_map[ref_file] = []

                citation_map[ref_file].append({
                    'lines': ref_lines,
                    'score': ref_score,
                    'type': 'reference'
                })

    for file_path, citations in sorted(citation_map.items()):
        print(f"\n{file_path}")

        citations.sort(key=lambda x: x['score'], reverse=True)

        for citation in citations:
            citation_type = "Primary" if citation['type'] == 'primary' else "Reference"
            print(f"  {citation_type}: Lines {citation['lines']} "
                  f"(relevance: {citation['score']:.3f})")


def main():
    """Run all advanced query engine examples."""
    print("\n" + "="*70)
    print("Advanced Query Engine Examples with AvocadoDB")
    print("="*70)

    Settings.llm = OpenAI(model="gpt-3.5-turbo", temperature=0)
    Settings.embed_model = OpenAIEmbedding(model="text-embedding-3-small")

    try:
        example_response_modes()
        example_custom_retriever()
        example_citation_verification()

        print("\n" + "="*70)
        print("All examples completed successfully!")
        print("="*70)

    except Exception as e:
        print(f"\nError: {e}")


if __name__ == "__main__":
    main()
