"""
Basic RAG (Retrieval-Augmented Generation) example with LangChain and AvocadoDB.

This example demonstrates:
1. Setting up AvocadoDBRetriever with custom configuration
2. Using it with RetrievalQA chain for question answering
3. Getting deterministic, citation-backed answers with source tracking
4. Different retrieval strategies (MMR, filtering, span combination)

Prerequisites:
- AvocadoDB server running (avocado-server)
- Documents ingested (avocado ingest . --recursive)
- OpenAI API key set (export OPENAI_API_KEY=...)

The key benefit of AvocadoDB is determinism: the same query will always
return the exact same context, making your RAG applications reproducible
and testable.
"""

from langchain_avocadodb import AvocadoDBRetriever
from langchain.chains import RetrievalQA
from langchain_openai import ChatOpenAI
from langchain.callbacks import StdOutCallbackHandler


def main():
    # Initialize AvocadoDB retriever
    print("Initializing AvocadoDB retriever...")
    retriever = AvocadoDBRetriever(
        url="http://localhost:8765",  # AvocadoDB server URL
        budget=8000,  # Token budget for context
        include_citations=True,  # Include source citations
        semantic_weight=0.7,  # Balance semantic search
        lexical_weight=0.3,  # with keyword search
    )

    # Create LLM
    llm = ChatOpenAI(
        model="gpt-3.5-turbo",
        temperature=0,  # Deterministic generation
    )

    # Create QA chain with retriever
    qa_chain = RetrievalQA.from_chain_type(
        llm=llm,
        chain_type="stuff",
        retriever=retriever,
        return_source_documents=True,
        callbacks=[StdOutCallbackHandler()],
    )

    # Example questions
    questions = [
        "How does the authentication system work?",
        "What database does the system use?",
        "How is error handling implemented?",
    ]

    for question in questions:
        print(f"\n{'='*60}")
        print(f"Question: {question}")
        print("=" * 60)

        # Get answer with sources
        result = qa_chain.invoke({"query": question})

        print(f"\nAnswer: {result['answer']}")

        # Show citations
        print("\nSources:")
        for doc in result["source_documents"]:
            metadata = doc.metadata
            print(
                f"  - {metadata['source']}:{metadata['start_line']}-{metadata['end_line']}"
            )
            if "score" in metadata:
                print(f"    Score: {metadata['score']:.3f}")
            if "citations" in metadata:
                for citation in metadata["citations"]:
                    print(f"    Citation: {citation['file']}:{citation['lines']}")

        # Show deterministic hash
        if result["source_documents"]:
            hash_value = result["source_documents"][0].metadata.get(
                "deterministic_hash"
            )
            print(f"\nDeterministic hash: {hash_value}")
            print("(Same query will always return same context)")


def example_with_mmr():
    """
    Example using MMR (Maximal Marginal Relevance) for diverse results.
    """
    print("\n" + "=" * 60)
    print("MMR Example - Getting diverse results")
    print("=" * 60)

    # Configure for diversity
    retriever = AvocadoDBRetriever(
        url="http://localhost:8765",
        budget=10000,  # Higher budget for more candidates
        enable_mmr=True,  # Enable MMR
        mmr_lambda=0.3,  # Lower = more diverse (0.0-1.0)
    )

    llm = ChatOpenAI(model="gpt-3.5-turbo", temperature=0)

    qa_chain = RetrievalQA.from_chain_type(
        llm=llm,
        retriever=retriever,
        return_source_documents=True,
    )

    # Query that benefits from diverse sources
    result = qa_chain.invoke(
        {"query": "What are all the different components of the system?"}
    )

    print(f"\nAnswer: {result['answer']}")
    print(f"\nDiverse sources found: {len(result['source_documents'])}")

    # Show unique files
    unique_files = set(doc.metadata["source"] for doc in result["source_documents"])
    print(f"Unique files: {', '.join(unique_files)}")


def example_with_filtering():
    """
    Example with score filtering for high-quality results.
    """
    print("\n" + "=" * 60)
    print("Filtered Example - Only high-quality matches")
    print("=" * 60)

    # Configure with minimum score threshold
    retriever = AvocadoDBRetriever(
        url="http://localhost:8765",
        budget=8000,
        min_score=0.7,  # Only return highly relevant results
        include_scores=True,
    )

    # Direct retrieval without chain
    docs = retriever.get_relevant_documents("database schema design")

    print(f"Found {len(docs)} high-quality matches")
    for doc in docs:
        print(f"\n- {doc.metadata['source']}")
        print(f"  Score: {doc.metadata.get('score', 0):.3f}")
        print(f"  Lines: {doc.metadata['start_line']}-{doc.metadata['end_line']}")
        print(f"  Preview: {doc.page_content[:100]}...")


def example_combined_spans():
    """
    Example combining adjacent spans for better context.
    """
    print("\n" + "=" * 60)
    print("Combined Spans Example - Better context windows")
    print("=" * 60)

    # Configure to combine adjacent spans
    retriever = AvocadoDBRetriever(
        url="http://localhost:8765",
        budget=8000,
        combine_spans=True,  # Combine adjacent spans
    )

    docs = retriever.get_relevant_documents("API endpoints implementation")

    for doc in docs:
        metadata = doc.metadata
        span_count = metadata.get("span_count", 1)

        if span_count > 1:
            print(f"\nCombined {span_count} adjacent spans:")
            print(f"  File: {metadata['source']}")
            print(f"  Lines: {metadata['start_line']}-{metadata['end_line']}")
            print(f"  Total tokens: {metadata['token_count']}")


if __name__ == "__main__":
    print("AvocadoDB + LangChain RAG Examples")
    print("==================================")
    print("\nMake sure AvocadoDB server is running:")
    print("  avocado-server")
    print("\nAnd you've ingested your codebase:")
    print("  avocado ingest . --recursive")
    print()

    try:
        # Run basic example
        main()

        # Run additional examples
        example_with_mmr()
        example_with_filtering()
        example_combined_spans()

    except Exception as e:
        print(f"\nError: {e}")
        print("\nTroubleshooting:")
        print("1. Is AvocadoDB server running? (avocado-server)")
        print("2. Have you ingested documents? (avocado ingest . --recursive)")
        print("3. Is the server URL correct? (default: http://localhost:8765)")
        print("4. Do you have OpenAI API key set? (export OPENAI_API_KEY=...)")
