"""
Advanced QA Chain patterns with AvocadoDB and LangChain.

This example demonstrates:
1. Different chain types (stuff, map_reduce, refine)
2. Custom prompts for better answers
3. Source attribution and citation formatting
4. Batch processing multiple questions
5. Chain customization and optimization

This goes beyond basic RAG to show advanced QA patterns.
"""

from langchain_avocadodb import AvocadoDBRetriever
from langchain.chains import RetrievalQA
from langchain.chains.question_answering import load_qa_chain
from langchain_openai import ChatOpenAI
from langchain.prompts import PromptTemplate
from langchain.callbacks import StdOutCallbackHandler
from typing import List, Dict
import json


def create_custom_prompt() -> PromptTemplate:
    """
    Create a custom prompt template for better QA answers.

    This prompt emphasizes:
    - Using provided context accurately
    - Citing sources with line numbers
    - Admitting when information is not in context
    - Structured, clear answers

    Returns:
        PromptTemplate for QA chain
    """
    template = """You are a technical documentation assistant. Use the following pieces of context to answer the question at the end.

Context from codebase:
{context}

Important guidelines:
1. Answer based ONLY on the provided context
2. Cite specific files and line numbers when referencing code
3. If the context doesn't contain enough information, say so
4. Be precise and technical - this is for developers
5. Format code snippets using markdown code blocks

Question: {question}

Detailed Answer:"""

    return PromptTemplate(template=template, input_variables=["context", "question"])


def example_basic_qa():
    """
    Basic QA chain with default settings.

    This is the simplest way to use AvocadoDB for Q&A.
    """
    print("=" * 60)
    print("Example 1: Basic QA Chain")
    print("=" * 60)

    # Create retriever
    retriever = AvocadoDBRetriever(
        url="http://localhost:8765",
        budget=8000,
        include_citations=True,
    )

    # Create LLM
    llm = ChatOpenAI(model="gpt-3.5-turbo", temperature=0)

    # Create QA chain (uses "stuff" strategy by default)
    qa_chain = RetrievalQA.from_chain_type(
        llm=llm,
        chain_type="stuff",  # Stuffs all context into one prompt
        retriever=retriever,
        return_source_documents=True,
    )

    # Ask a question
    question = "How does the authentication system work?"
    print(f"\nQuestion: {question}")

    result = qa_chain.invoke({"query": question})

    print(f"\nAnswer:\n{result['answer']}")
    print(f"\nSources: {len(result['source_documents'])} documents")

    return result


def example_custom_prompt_qa():
    """
    QA chain with custom prompt for better answers.

    Custom prompts allow you to:
    - Control answer format
    - Emphasize specific aspects
    - Improve citation quality
    """
    print("\n" + "=" * 60)
    print("Example 2: QA with Custom Prompt")
    print("=" * 60)

    retriever = AvocadoDBRetriever(
        url="http://localhost:8765",
        budget=8000,
        include_citations=True,
        include_scores=True,
    )

    llm = ChatOpenAI(model="gpt-3.5-turbo", temperature=0)

    # Create QA chain with custom prompt
    qa_chain = RetrievalQA.from_chain_type(
        llm=llm,
        chain_type="stuff",
        retriever=retriever,
        return_source_documents=True,
        chain_type_kwargs={"prompt": create_custom_prompt()},
    )

    question = "What database does the system use and how is it configured?"
    print(f"\nQuestion: {question}")

    result = qa_chain.invoke({"query": question})

    print(f"\nAnswer with Citations:\n{result['answer']}")

    # Show detailed source information
    print("\n" + "-" * 60)
    print("DETAILED SOURCES")
    print("-" * 60)
    for i, doc in enumerate(result["source_documents"], 1):
        meta = doc.metadata
        print(f"\n[{i}] {meta['source']}")
        print(f"    Lines: {meta['start_line']}-{meta['end_line']}")
        print(f"    Relevance: {meta.get('score', 'N/A')}")
        print(f"    Tokens: {meta['token_count']}")

    return result


def example_batch_questions():
    """
    Process multiple questions efficiently.

    Useful for:
    - Documentation generation
    - FAQ creation
    - Systematic code analysis
    """
    print("\n" + "=" * 60)
    print("Example 3: Batch Question Processing")
    print("=" * 60)

    retriever = AvocadoDBRetriever(
        url="http://localhost:8765",
        budget=6000,  # Lower budget for batch processing
        include_citations=True,
    )

    llm = ChatOpenAI(model="gpt-3.5-turbo", temperature=0)

    qa_chain = RetrievalQA.from_chain_type(
        llm=llm,
        chain_type="stuff",
        retriever=retriever,
        return_source_documents=True,
    )

    # List of questions to process
    questions = [
        "What is the main authentication mechanism?",
        "How is the database connection managed?",
        "What API endpoints are available?",
        "How is error handling implemented?",
    ]

    results = []

    print(f"\nProcessing {len(questions)} questions...\n")

    for i, question in enumerate(questions, 1):
        print(f"[{i}/{len(questions)}] {question}")

        try:
            result = qa_chain.invoke({"query": question})
            results.append(
                {
                    "question": question,
                    "answer": result["answer"],
                    "source_count": len(result["source_documents"]),
                    "success": True,
                }
            )
            print(f"    ✓ Answered ({len(result['source_documents'])} sources)")

        except Exception as e:
            print(f"    ✗ Error: {e}")
            results.append({"question": question, "error": str(e), "success": False})

    # Summary
    print("\n" + "=" * 60)
    print("BATCH RESULTS SUMMARY")
    print("=" * 60)
    successful = sum(1 for r in results if r.get("success"))
    print(f"Successful: {successful}/{len(results)}")

    # Show all answers
    for i, result in enumerate(results, 1):
        if result.get("success"):
            print(f"\n[Q{i}] {result['question']}")
            print(f"[A{i}] {result['answer'][:200]}...")

    return results


def example_map_reduce_chain():
    """
    Use map-reduce chain for handling large contexts.

    Map-reduce:
    - Processes documents independently (map)
    - Combines results (reduce)
    - Better for very large contexts that don't fit in one prompt
    """
    print("\n" + "=" * 60)
    print("Example 4: Map-Reduce Chain for Large Contexts")
    print("=" * 60)

    retriever = AvocadoDBRetriever(
        url="http://localhost:8765",
        budget=15000,  # Larger budget
        include_citations=True,
    )

    llm = ChatOpenAI(model="gpt-3.5-turbo", temperature=0)

    # Use map_reduce chain type
    qa_chain = RetrievalQA.from_chain_type(
        llm=llm,
        chain_type="map_reduce",  # Process docs separately, then combine
        retriever=retriever,
        return_source_documents=True,
    )

    question = "Summarize all the different components and their responsibilities"
    print(f"\nQuestion: {question}")
    print("(This question benefits from map-reduce approach)\n")

    result = qa_chain.invoke({"query": question})

    print(f"Answer:\n{result['answer']}")
    print(f"\nProcessed {len(result['source_documents'])} documents")

    return result


def example_with_filtering():
    """
    QA with score filtering for high-quality results.

    Only uses the most relevant context for answers.
    """
    print("\n" + "=" * 60)
    print("Example 5: QA with Score Filtering")
    print("=" * 60)

    # Configure retriever with minimum score threshold
    retriever = AvocadoDBRetriever(
        url="http://localhost:8765",
        budget=8000,
        include_citations=True,
        include_scores=True,
        min_score=0.75,  # Only highly relevant results
    )

    llm = ChatOpenAI(model="gpt-3.5-turbo", temperature=0)

    qa_chain = RetrievalQA.from_chain_type(
        llm=llm,
        chain_type="stuff",
        retriever=retriever,
        return_source_documents=True,
    )

    question = "How are JWT tokens generated?"
    print(f"\nQuestion: {question}")
    print("(Using only high-relevance context with score >= 0.75)\n")

    result = qa_chain.invoke({"query": question})

    print(f"Answer:\n{result['answer']}")

    # Show scores
    print("\n" + "-" * 60)
    print("HIGH-RELEVANCE SOURCES")
    print("-" * 60)
    for doc in result["source_documents"]:
        meta = doc.metadata
        print(f"• {meta['source']} (score: {meta.get('score', 'N/A')})")

    return result


def example_export_results(results: List[Dict]):
    """
    Export QA results to JSON for documentation.

    Args:
        results: List of QA results to export
    """
    print("\n" + "=" * 60)
    print("Example 6: Export Results")
    print("=" * 60)

    output_file = "qa_results.json"

    # Prepare export data
    export_data = {
        "questions_answered": len(results),
        "timestamp": "2024-01-01",  # In production, use actual timestamp
        "results": [],
    }

    for result in results:
        if result.get("success"):
            export_data["results"].append(
                {
                    "question": result["question"],
                    "answer": result["answer"],
                    "source_count": result["source_count"],
                }
            )

    # Save to file
    with open(output_file, "w") as f:
        json.dump(export_data, f, indent=2)

    print(f"\nExported {len(results)} results to {output_file}")


def main():
    """
    Run all examples.

    Demonstrates the full range of QA patterns with AvocadoDB.
    """
    print("Advanced QA Chain Patterns with AvocadoDB")
    print("=" * 60)
    print("\nThis script demonstrates various QA chain configurations.")
    print("Make sure AvocadoDB server is running with ingested data.\n")

    try:
        # Run examples
        example_basic_qa()

        input("\nPress Enter to continue to next example...")
        example_custom_prompt_qa()

        input("\nPress Enter to continue to batch processing...")
        batch_results = example_batch_questions()

        input("\nPress Enter to continue to map-reduce...")
        example_map_reduce_chain()

        input("\nPress Enter to continue to filtering example...")
        example_with_filtering()

        # Export results
        example_export_results(batch_results)

        print("\n" + "=" * 60)
        print("All examples completed successfully!")
        print("=" * 60)

    except KeyboardInterrupt:
        print("\n\nExamples interrupted by user.")
    except Exception as e:
        print(f"\nError running examples: {e}")
        print("\nTroubleshooting:")
        print("1. Is AvocadoDB server running? (avocado-server)")
        print("2. Have you ingested documents? (avocado ingest . --recursive)")
        print("3. Is OPENAI_API_KEY set?")


if __name__ == "__main__":
    main()
