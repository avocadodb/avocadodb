"""
Chat Engine with AvocadoDB Memory Persistence.

This example demonstrates building chat engines with AvocadoDB
that maintain conversation context and memory across sessions.

Features:
- Different chat modes (context, condense, react)
- Memory persistence
- Session management
- Context-aware responses
"""

from llama_index_avocadodb import AvocadoDBReader
from llama_index.core import VectorStoreIndex, Settings
from llama_index.core.memory import ChatMemoryBuffer
from llama_index.llms.openai import OpenAI
import json
from typing import List, Dict
from pathlib import Path


def example_context_chat_engine():
    """
    Context chat mode: Uses retrieved context with each message.

    Best for: Technical Q&A where context needs to be referenced.
    """
    print("="*70)
    print("Example 1: Context Chat Engine")
    print("="*70)

    reader = AvocadoDBReader(
        url="http://localhost:8765",
        budget=10000,
        include_citations=True
    )

    # Load initial context
    print("\nLoading context about authentication...")
    documents = reader.load_data("authentication and security implementation")
    print(f"Loaded {len(documents)} documents\n")

    # Create index
    index = VectorStoreIndex.from_documents(documents)

    # Configure LLM
    Settings.llm = OpenAI(model="gpt-3.5-turbo", temperature=0.7)

    # Create chat engine
    chat_engine = index.as_chat_engine(
        chat_mode="context",
        memory=ChatMemoryBuffer.from_defaults(token_limit=3000),
        verbose=True
    )

    # Simulate conversation
    conversation = [
        "What authentication methods are available?",
        "How is JWT token validation handled?",
        "Are there any security vulnerabilities I should know about?",
        "What's the token expiration policy?"
    ]

    for message in conversation:
        print(f"\nUser: {message}")
        print("-" * 70)

        response = chat_engine.chat(message)
        print(f"Assistant: {response}\n")

        # Show sources used
        if hasattr(response, 'source_nodes') and response.source_nodes:
            print("Sources:")
            for node in response.source_nodes[:2]:
                metadata = node.metadata
                print(f"  - {metadata.get('file_path')}:{metadata.get('start_line')}")


def example_condense_chat_engine():
    """
    Condense chat mode: Condenses conversation history into queries.

    Best for: Multi-turn conversations with evolving context needs.
    """
    print("\n" + "="*70)
    print("Example 2: Condense Chat Engine")
    print("="*70)

    reader = AvocadoDBReader(
        url="http://localhost:8765",
        budget=12000
    )

    # Load broad context
    print("\nLoading system architecture context...")
    documents = reader.load_data("system architecture and components")
    print(f"Loaded {len(documents)} documents\n")

    index = VectorStoreIndex.from_documents(documents)

    # Create condense chat engine
    chat_engine = index.as_chat_engine(
        chat_mode="condense_question",
        memory=ChatMemoryBuffer.from_defaults(token_limit=2000),
        verbose=False
    )

    # Progressive conversation
    conversation = [
        "What's the overall architecture?",
        "How do the components communicate?",
        "What about the database layer?",
        "How is that secured?"
    ]

    for message in conversation:
        print(f"\nUser: {message}")
        response = chat_engine.chat(message)
        print(f"Assistant: {response}")
        print("-" * 70)


def example_chat_with_memory_persistence():
    """
    Chat engine with persistent memory across sessions.

    Demonstrates saving and loading conversation state.
    """
    print("\n" + "="*70)
    print("Example 3: Chat with Memory Persistence")
    print("="*70)

    reader = AvocadoDBReader(url="http://localhost:8765")

    # Session management
    session_file = "/tmp/chat_session.json"
    session_data = {
        "session_id": "demo-session-001",
        "messages": []
    }

    # Load context
    documents = reader.load_data("API and web services", budget=8000)
    index = VectorStoreIndex.from_documents(documents)

    # Create chat engine
    chat_engine = index.as_chat_engine(
        chat_mode="context",
        memory=ChatMemoryBuffer.from_defaults(token_limit=2000)
    )

    # First part of conversation
    print("\n--- Session Start ---")
    messages_part1 = [
        "What APIs are available?",
        "How do I authenticate?"
    ]

    for msg in messages_part1:
        print(f"\nUser: {msg}")
        response = chat_engine.chat(msg)
        print(f"Assistant: {response}")

        # Save to session
        session_data["messages"].append({
            "role": "user",
            "content": msg
        })
        session_data["messages"].append({
            "role": "assistant",
            "content": str(response)
        })

    # Save session
    print(f"\n--- Saving session to {session_file} ---")
    with open(session_file, 'w') as f:
        json.dump(session_data, f, indent=2)

    # Simulate session break
    print("\n--- Session Break (simulating app restart) ---\n")

    # Load session
    print(f"--- Loading session from {session_file} ---")
    with open(session_file, 'r') as f:
        loaded_session = json.load(f)

    print(f"Restored {len(loaded_session['messages'])} messages")

    # Create new chat engine and restore context
    new_chat_engine = index.as_chat_engine(
        chat_mode="context",
        memory=ChatMemoryBuffer.from_defaults(token_limit=2000)
    )

    # Restore conversation history
    for i in range(0, len(loaded_session['messages']), 2):
        user_msg = loaded_session['messages'][i]['content']
        assistant_msg = loaded_session['messages'][i+1]['content']

        # Re-inject into memory
        new_chat_engine.chat_history.append({
            "role": "user",
            "content": user_msg
        })
        new_chat_engine.chat_history.append({
            "role": "assistant",
            "content": assistant_msg
        })

    # Continue conversation
    print("\n--- Continuing Session ---")
    messages_part2 = [
        "What were we just talking about?",
        "Can you give me more details on that?"
    ]

    for msg in messages_part2:
        print(f"\nUser: {msg}")
        response = new_chat_engine.chat(msg)
        print(f"Assistant: {response}")


def example_multi_index_chat():
    """
    Chat across multiple knowledge bases.

    Demonstrates switching contexts within a conversation.
    """
    print("\n" + "="*70)
    print("Example 4: Multi-Index Chat")
    print("="*70)

    reader = AvocadoDBReader(url="http://localhost:8765")

    # Create multiple indexes for different topics
    print("\nCreating specialized indexes...")

    topics = {
        "frontend": "frontend UI components and React",
        "backend": "backend API and services",
        "database": "database schema and queries"
    }

    indexes = {}
    for topic, query in topics.items():
        docs = reader.load_data(query, budget=5000)
        indexes[topic] = VectorStoreIndex.from_documents(docs)
        print(f"  {topic}: {len(docs)} documents")

    # Start with frontend index
    current_topic = "frontend"
    chat_engine = indexes[current_topic].as_chat_engine(
        chat_mode="context",
        verbose=False
    )

    print(f"\nStarting conversation (context: {current_topic})")
    print("="*70)

    # Conversation that switches topics
    conversations = [
        ("frontend", "What UI components are available?"),
        ("frontend", "How is state managed?"),
        ("backend", "Now tell me about the backend APIs"),
        ("backend", "How do they handle errors?"),
        ("database", "What about the database schema?")
    ]

    for topic, message in conversations:
        # Switch index if topic changed
        if topic != current_topic:
            print(f"\n>>> Switching context to: {topic} <<<\n")
            current_topic = topic
            chat_engine = indexes[topic].as_chat_engine(
                chat_mode="context",
                verbose=False
            )

        print(f"User: {message}")
        response = chat_engine.chat(message)
        print(f"Assistant: {response}")
        print("-" * 70)


def example_chat_with_citations():
    """
    Chat engine that includes citations in responses.

    Shows how to track and display sources for each answer.
    """
    print("\n" + "="*70)
    print("Example 5: Chat with Citation Tracking")
    print("="*70)

    reader = AvocadoDBReader(
        url="http://localhost:8765",
        include_citations=True,
        include_scores=True,
        budget=10000
    )

    documents = reader.load_data("error handling and logging")
    index = VectorStoreIndex.from_documents(documents)

    chat_engine = index.as_chat_engine(
        chat_mode="context",
        similarity_top_k=3
    )

    questions = [
        "How are errors caught and handled?",
        "What logging framework is used?",
        "How are critical errors reported?"
    ]

    for question in questions:
        print(f"\nUser: {question}")
        print("="*70)

        response = chat_engine.chat(question)
        print(f"\nAssistant: {response}")

        # Extract and display citations
        if hasattr(response, 'source_nodes'):
            print("\nCitations:")
            for i, node in enumerate(response.source_nodes, 1):
                metadata = node.metadata
                file_path = metadata.get('file_path', 'Unknown')
                lines = f"{metadata.get('start_line')}-{metadata.get('end_line')}"
                score = metadata.get('score', 0)

                print(f"  [{i}] {file_path}:{lines} (relevance: {score:.3f})")

                # Show referenced citations
                if 'citations' in metadata:
                    for citation in metadata['citations'][:2]:
                        ref_file = citation['file']
                        ref_lines = f"{citation['start_line']}-{citation['end_line']}"
                        print(f"      References: {ref_file}:{ref_lines}")

        print("-" * 70)


def example_smart_context_loading():
    """
    Intelligently load more context as conversation progresses.

    Demonstrates dynamic context expansion based on conversation.
    """
    print("\n" + "="*70)
    print("Example 6: Smart Context Loading")
    print("="*70)

    reader = AvocadoDBReader(url="http://localhost:8765")

    # Start with minimal context
    print("\nLoading initial context...")
    initial_docs = reader.load_data("system overview", budget=3000)
    print(f"Initial: {len(initial_docs)} documents")

    index = VectorStoreIndex.from_documents(initial_docs)
    chat_engine = index.as_chat_engine(chat_mode="context")

    # Track topics discussed
    topics_discussed = ["overview"]

    conversation = [
        ("What is this system about?", None),
        ("Tell me about the authentication", "authentication implementation"),
        ("How does the database work?", "database architecture"),
        ("What about error handling?", "error handling strategies")
    ]

    for message, expansion_query in conversation:
        print(f"\nUser: {message}")

        # Expand context if needed
        if expansion_query and expansion_query not in topics_discussed:
            print(f"  -> Loading more context: {expansion_query}")
            new_docs = reader.load_data(expansion_query, budget=3000)

            for doc in new_docs:
                index.insert(doc)

            print(f"  -> Added {len(new_docs)} documents")
            topics_discussed.append(expansion_query)

        response = chat_engine.chat(message)
        print(f"Assistant: {response}")
        print("-" * 70)

    print(f"\nTotal topics covered: {len(topics_discussed)}")
    print(f"Topics: {', '.join(topics_discussed)}")


def main():
    """Run all chat engine examples."""
    print("\n" + "="*70)
    print("Chat Engine Examples with AvocadoDB")
    print("="*70)
    print("\nDemonstrates various chat patterns with persistent context")
    print("\nPrerequisites:")
    print("  1. AvocadoDB server: avocado-server")
    print("  2. Ingested docs: avocado ingest . --recursive")
    print("  3. OpenAI API key: export OPENAI_API_KEY=...")
    print()

    try:
        example_context_chat_engine()
        example_condense_chat_engine()
        example_chat_with_memory_persistence()
        example_multi_index_chat()
        example_chat_with_citations()
        example_smart_context_loading()

        print("\n" + "="*70)
        print("All chat examples completed!")
        print("="*70)

    except Exception as e:
        print(f"\nError: {e}")
        print("\nTroubleshooting:")
        print("  - Check server: curl http://localhost:8765/stats")
        print("  - Verify ingestion: avocado stats")
        print("  - Check API key: echo $OPENAI_API_KEY")


if __name__ == "__main__":
    main()
