"""
Basic RAG example with LlamaIndex and AvocadoDB.

This example demonstrates:
1. Setting up AvocadoDBReader
2. Creating an index from loaded documents
3. Querying with citation tracking
"""

from llama_index_avocadodb import AvocadoDBReader
from llama_index.core import VectorStoreIndex, Settings
from llama_index.llms.openai import OpenAI
from llama_index.core.response.pprint_utils import pprint_response


def main():
    # Initialize AvocadoDB reader
    print("Initializing AvocadoDB reader...")
    reader = AvocadoDBReader(
        url="http://localhost:8765",  # AvocadoDB server URL
        budget=8000,  # Token budget
        include_citations=True,  # Include source citations
        semantic_weight=0.7,  # Semantic search weight
        lexical_weight=0.3,  # Keyword search weight
    )

    # Configure LLM
    Settings.llm = OpenAI(
        model="gpt-3.5-turbo",
        temperature=0,  # Deterministic
    )

    # Example queries
    queries = [
        "How does the authentication system work?",
        "What database architecture is used?",
        "Explain the error handling strategy",
    ]

    for query in queries:
        print(f"\n{'='*60}")
        print(f"Query: {query}")
        print('='*60)

        # Load relevant documents from AvocadoDB
        documents = reader.load_data(query, budget=8000)

        print(f"\nLoaded {len(documents)} relevant documents")

        # Show document sources
        for doc in documents[:3]:  # Show first 3
            metadata = doc.metadata
            print(f"  - {metadata['file_path']}:{metadata['start_line']}-{metadata['end_line']}")
            if "score" in metadata:
                print(f"    Score: {metadata['score']:.3f}")

        # Create index from documents
        index = VectorStoreIndex.from_documents(documents)

        # Create query engine
        query_engine = index.as_query_engine(
            response_mode="tree_summarize",  # Summarize across documents
            verbose=True,
        )

        # Query the index
        response = query_engine.query(query)

        # Print response
        print(f"\nAnswer: {response}")

        # Show source nodes with citations
        print("\nSource Citations:")
        for node in response.source_nodes:
            metadata = node.metadata
            print(f"  - {metadata.get('file_path', 'Unknown')}:")
            print(f"    Lines: {metadata.get('start_line', '?')}-{metadata.get('end_line', '?')}")
            if "citations" in metadata:
                for citation in metadata["citations"]:
                    print(f"    Referenced: {citation['file']}:{citation['start_line']}-{citation['end_line']}")

        # Show deterministic hash
        if documents:
            hash_value = documents[0].metadata.get("deterministic_hash")
            print(f"\nDeterministic hash: {hash_value}")
            print("(Same query always returns same context)")


def example_with_nodes():
    """
    Example using TextNodes for more control.
    """
    print("\n" + "="*60)
    print("TextNode Example - Fine-grained control")
    print("="*60)

    # Configure reader to return nodes
    reader = AvocadoDBReader(
        url="http://localhost:8765",
        create_nodes=True,  # Return TextNodes instead of Documents
        budget=10000,
    )

    # Load as nodes
    nodes = reader.load_data("API design patterns", budget=10000)

    print(f"Loaded {len(nodes)} nodes")

    # Create index from nodes
    index = VectorStoreIndex(nodes)

    # Query with custom settings
    query_engine = index.as_query_engine(
        similarity_top_k=5,  # Return top 5 most similar
        response_mode="compact",  # Compact response
    )

    response = query_engine.query("What are the REST API best practices?")

    print(f"\nAnswer: {response}")

    # Nodes have more detailed metadata
    for i, node in enumerate(response.source_nodes):
        print(f"\nNode {i+1}:")
        print(f"  ID: {node.id_}")
        print(f"  Score: {node.score:.3f}")
        print(f"  Tokens: {node.metadata.get('token_count', 'N/A')}")


def example_batch_loading():
    """
    Example loading multiple queries at once.
    """
    print("\n" + "="*60)
    print("Batch Loading Example - Multiple queries")
    print("="*60)

    reader = AvocadoDBReader(url="http://localhost:8765")

    # Multiple queries to process
    queries = [
        "database indexing strategies",
        "caching implementation",
        "security best practices",
    ]

    # Batch load
    all_documents = reader.load_data_batch(queries, budget=5000)

    for query, docs in zip(queries, all_documents):
        print(f"\nQuery: '{query}'")
        print(f"  Found {len(docs)} documents")
        if docs:
            print(f"  Top source: {docs[0].metadata['file_path']}")


def example_combined_documents():
    """
    Example combining adjacent spans for better context.
    """
    print("\n" + "="*60)
    print("Combined Documents Example - Consolidated context")
    print("="*60)

    # Configure to combine adjacent spans
    reader = AvocadoDBReader(
        url="http://localhost:8765",
        combine_adjacent=True,  # Combine nearby spans
        budget=12000,
    )

    documents = reader.load_data("complete authentication flow")

    for doc in documents:
        metadata = doc.metadata
        span_count = metadata.get("span_count", 1)

        if span_count > 1:
            print(f"\nCombined {span_count} adjacent spans:")
            print(f"  File: {metadata['file_path']}")
            print(f"  Lines: {metadata['start_line']}-{metadata['end_line']}")
            print(f"  Total tokens: {metadata['token_count']}")
            if "avg_score" in metadata:
                print(f"  Average score: {metadata['avg_score']:.3f}")


def example_lazy_loading():
    """
    Example using lazy loading for memory efficiency.
    """
    print("\n" + "="*60)
    print("Lazy Loading Example - Memory efficient")
    print("="*60)

    reader = AvocadoDBReader(
        url="http://localhost:8765",
        min_score=0.6,  # Only high-quality matches
    )

    # Lazy load documents (generator)
    print("Processing documents lazily...")
    doc_count = 0
    total_tokens = 0

    for doc in reader.lazy_load_data("system architecture", budget=15000):
        doc_count += 1
        total_tokens += doc.metadata.get("token_count", 0)

        # Process each document as it's loaded
        if doc_count <= 3:
            print(f"\nDocument {doc_count}:")
            print(f"  File: {doc.metadata['file_path']}")
            print(f"  Tokens: {doc.metadata['token_count']}")
            print(f"  Score: {doc.metadata.get('score', 0):.3f}")

    print(f"\nTotal documents processed: {doc_count}")
    print(f"Total tokens: {total_tokens}")


def example_with_chat():
    """
    Example using AvocadoDB in a chat context.
    """
    print("\n" + "="*60)
    print("Chat Example - Context-aware conversations")
    print("="*60)

    from llama_index.core import ChatPromptTemplate
    from llama_index.core.chat_engine import ContextChatEngine

    reader = AvocadoDBReader(url="http://localhost:8765")

    # Initial context load
    context_docs = reader.load_data("project overview and architecture", budget=10000)

    # Create index
    index = VectorStoreIndex.from_documents(context_docs)

    # Create chat engine
    chat_engine = index.as_chat_engine(
        chat_mode="context",
        verbose=True,
    )

    # Simulate conversation
    questions = [
        "What is the main purpose of this project?",
        "How does it handle scalability?",
        "What are the main components?",
    ]

    for question in questions:
        print(f"\nUser: {question}")
        response = chat_engine.chat(question)
        print(f"Assistant: {response}")


if __name__ == "__main__":
    print("AvocadoDB + LlamaIndex RAG Examples")
    print("===================================")
    print("\nPrerequisites:")
    print("1. AvocadoDB server running: avocado-server")
    print("2. Documents ingested: avocado ingest . --recursive")
    print("3. OpenAI API key set: export OPENAI_API_KEY=...")
    print()

    try:
        # Run basic example
        main()

        # Run additional examples
        example_with_nodes()
        example_batch_loading()
        example_combined_documents()
        example_lazy_loading()
        # example_with_chat()  # Uncomment for chat example

    except Exception as e:
        print(f"\nError: {e}")
        print("\nTroubleshooting:")
        print("1. Check AvocadoDB server: curl http://localhost:8765/stats")
        print("2. Verify ingestion: avocado stats")
        print("3. Check OpenAI API key: echo $OPENAI_API_KEY")
        print("4. Install dependencies: pip install llama-index-avocadodb llama-index-llms-openai")