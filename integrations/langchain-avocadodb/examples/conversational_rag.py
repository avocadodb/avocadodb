"""
Conversational RAG with session management using AvocadoDB and LangChain.

This example demonstrates:
1. Building a conversational interface with memory
2. Using AvocadoDB for deterministic context retrieval
3. Maintaining conversation history across multiple turns
4. Combining chat memory with retrieval for contextual answers

The conversational chain remembers previous questions and answers,
allowing for follow-up questions and contextual understanding.
"""

from langchain_avocadodb import AvocadoDBRetriever
from langchain.chains import ConversationalRetrievalChain
from langchain.memory import ConversationBufferMemory
from langchain_openai import ChatOpenAI
from langchain.prompts import PromptTemplate
import sys


def create_conversational_chain():
    """
    Create a conversational RAG chain with AvocadoDB retriever.

    Returns:
        ConversationalRetrievalChain configured with memory and retrieval
    """
    # Initialize AvocadoDB retriever for deterministic context
    print("Initializing AvocadoDB retriever...")
    retriever = AvocadoDBRetriever(
        url="http://localhost:8765",
        budget=8000,  # 8K tokens of context
        include_citations=True,
        semantic_weight=0.7,  # Balanced semantic and keyword search
        lexical_weight=0.3,
        enable_mmr=True,  # Enable diversity in results
        mmr_lambda=0.5,  # Balance relevance and diversity
    )

    # Initialize LLM with low temperature for consistent answers
    llm = ChatOpenAI(
        model="gpt-3.5-turbo",
        temperature=0.1,  # Low temperature for consistent responses
        streaming=True,  # Enable streaming for better UX
    )

    # Create conversation memory to track chat history
    # This stores the last N messages and provides them as context
    memory = ConversationBufferMemory(
        memory_key="chat_history",
        return_messages=True,  # Return as message objects
        output_key="answer",  # Key for the answer in the result
    )

    # Create the conversational chain
    # This combines retrieval with chat memory
    chain = ConversationalRetrievalChain.from_llm(
        llm=llm,
        retriever=retriever,
        memory=memory,
        return_source_documents=True,  # Include retrieved documents in response
        verbose=False,  # Set to True to see chain internals
        # Custom prompt can be added here if needed
    )

    return chain


def print_sources(documents):
    """
    Print source citations from retrieved documents.

    Args:
        documents: List of LangChain Document objects with metadata
    """
    print("\n" + "=" * 60)
    print("SOURCES")
    print("=" * 60)

    for i, doc in enumerate(documents, 1):
        metadata = doc.metadata
        print(f"\n[{i}] {metadata['source']}")
        print(f"    Lines: {metadata['start_line']}-{metadata['end_line']}")

        # Show relevance score if available
        if "score" in metadata:
            print(f"    Relevance: {metadata['score']:.3f}")

        # Show token count
        print(f"    Tokens: {metadata['token_count']}")

        # Show snippet of content
        snippet = doc.page_content[:100].replace("\n", " ")
        print(f"    Preview: {snippet}...")


def run_conversation():
    """
    Run an interactive conversational RAG session.

    Users can ask follow-up questions that reference previous context.
    The chain maintains conversation history and uses AvocadoDB for
    deterministic context retrieval.
    """
    print("=" * 60)
    print("Conversational RAG with AvocadoDB")
    print("=" * 60)
    print("\nThis is an interactive conversational RAG system.")
    print("Ask questions about your codebase, and follow up with")
    print("related questions. The system remembers your conversation.")
    print("\nType 'quit' or 'exit' to end the conversation.")
    print("Type 'clear' to reset the conversation history.")
    print("=" * 60)

    # Create the chain
    try:
        chain = create_conversational_chain()
    except Exception as e:
        print(f"\nError initializing chain: {e}")
        print("\nMake sure:")
        print("1. AvocadoDB server is running (avocado-server)")
        print("2. You've ingested documents (avocado ingest . --recursive)")
        print("3. OPENAI_API_KEY environment variable is set")
        sys.exit(1)

    # Keep track of turn number for display
    turn = 0

    # Main conversation loop
    while True:
        print("\n" + "-" * 60)

        # Get user input
        try:
            question = input("\nYou: ").strip()
        except (EOFError, KeyboardInterrupt):
            print("\n\nGoodbye!")
            break

        # Handle special commands
        if question.lower() in ["quit", "exit"]:
            print("\nGoodbye!")
            break

        if question.lower() == "clear":
            # Reset conversation memory
            chain.memory.clear()
            turn = 0
            print("\nConversation history cleared.")
            continue

        if not question:
            continue

        turn += 1
        print(f"\n[Turn {turn}]")

        # Query the chain
        try:
            # The chain will:
            # 1. Use the question + chat history to reformulate the query
            # 2. Retrieve relevant context from AvocadoDB
            # 3. Generate an answer using the LLM
            # 4. Store the Q&A in memory for future turns
            result = chain.invoke({"question": question})

            # Print the answer
            print(f"\nAssistant: {result['answer']}")

            # Print source documents if available
            if result.get("source_documents"):
                print_sources(result["source_documents"])

            # Show deterministic hash for reproducibility
            if result["source_documents"]:
                hash_val = result["source_documents"][0].metadata.get(
                    "deterministic_hash"
                )
                if hash_val:
                    print(f"\nContext Hash: {hash_val}")
                    print(
                        "(Asking the same question again will retrieve identical context)"
                    )

        except Exception as e:
            print(f"\nError: {e}")
            print("Please try rephrasing your question.")


def example_programmatic():
    """
    Example of using conversational chain programmatically.

    This shows how to use the chain in code rather than interactively.
    """
    print("\n" + "=" * 60)
    print("Programmatic Conversational RAG Example")
    print("=" * 60)

    chain = create_conversational_chain()

    # Example conversation with follow-up questions
    questions = [
        "How does the authentication system work?",
        "What algorithm does it use for tokens?",  # Follow-up referring to "it"
        "Are there any security concerns with this approach?",  # Follow-up
    ]

    for i, question in enumerate(questions, 1):
        print(f"\n[Question {i}]: {question}")
        result = chain.invoke({"question": question})
        print(f"[Answer {i}]: {result['answer']}")

        # Show how many sources were used
        num_sources = len(result.get("source_documents", []))
        print(f"[Sources]: {num_sources} documents retrieved")


def main():
    """Main entry point."""
    import sys

    # Check if we should run interactive or programmatic example
    if len(sys.argv) > 1 and sys.argv[1] == "--demo":
        example_programmatic()
    else:
        run_conversation()


if __name__ == "__main__":
    main()
