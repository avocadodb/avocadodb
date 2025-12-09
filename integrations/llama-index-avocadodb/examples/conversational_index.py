"""
Conversational Index with AvocadoDB Session Management.

This example demonstrates how to use AvocadoDB's session management
capabilities with LlamaIndex to build a conversational agent with
persistent memory across interactions.

Features:
- Session-based context loading
- Conversation history tracking
- Context evolution over time
- Deterministic retrieval per session
"""

from llama_index_avocadodb import AvocadoDBReader
from llama_index.core import VectorStoreIndex, Settings
from llama_index.llms.openai import OpenAI
from llama_index.core.memory import ChatMemoryBuffer
from typing import List, Dict, Any
import json


class ConversationalAvocadoIndex:
    """
    A conversational index that uses AvocadoDB for session-based retrieval.

    This class manages conversation sessions with context that evolves
    based on the conversation history.
    """

    def __init__(
        self,
        reader: AvocadoDBReader,
        initial_query: str,
        session_id: str = "default",
        budget_per_turn: int = 5000
    ):
        """
        Initialize conversational index.

        Args:
            reader: AvocadoDB reader instance
            initial_query: Initial query to load base context
            session_id: Unique session identifier
            budget_per_turn: Token budget for each conversation turn
        """
        self.reader = reader
        self.session_id = session_id
        self.budget_per_turn = budget_per_turn
        self.conversation_history: List[Dict[str, str]] = []

        # Configure LLM
        Settings.llm = OpenAI(model="gpt-3.5-turbo", temperature=0.7)

        # Load initial context
        print(f"Loading initial context for session: {session_id}")
        print(f"Query: {initial_query}")
        initial_docs = reader.load_data(initial_query, budget=budget_per_turn)
        print(f"Loaded {len(initial_docs)} initial documents\n")

        # Create initial index
        self.index = VectorStoreIndex.from_documents(initial_docs)
        self.chat_engine = self.index.as_chat_engine(
            chat_mode="context",
            memory=ChatMemoryBuffer.from_defaults(token_limit=3000),
            verbose=False
        )

    def chat(self, user_message: str) -> str:
        """
        Send a message and get a response.

        This method:
        1. Records the user message
        2. Optionally expands context based on the conversation
        3. Generates a response using the chat engine
        4. Records the assistant response

        Args:
            user_message: The user's message

        Returns:
            The assistant's response
        """
        print(f"\nUser: {user_message}")

        # Check if we need to expand context based on the conversation
        if self._should_expand_context(user_message):
            self._expand_context(user_message)

        # Get response from chat engine
        response = self.chat_engine.chat(user_message)
        response_text = str(response)

        # Record conversation
        self.conversation_history.append({
            "role": "user",
            "content": user_message
        })
        self.conversation_history.append({
            "role": "assistant",
            "content": response_text
        })

        print(f"Assistant: {response_text}")

        # Show sources if available
        if hasattr(response, 'source_nodes') and response.source_nodes:
            print("\nSources used:")
            for i, node in enumerate(response.source_nodes[:3], 1):
                metadata = node.metadata
                print(f"  {i}. {metadata.get('file_path', 'Unknown')}")
                print(f"     Lines: {metadata.get('start_line')}-{metadata.get('end_line')}")
                if 'score' in metadata:
                    print(f"     Relevance: {metadata['score']:.2f}")

        return response_text

    def _should_expand_context(self, message: str) -> bool:
        """
        Determine if we should load more context.

        Context is expanded when:
        - User asks about a new topic
        - User requests more details
        - Conversation shifts direction

        Args:
            message: Current user message

        Returns:
            True if context should be expanded
        """
        # Keywords that suggest need for more context
        expansion_keywords = [
            "more about", "tell me about", "what about",
            "how does", "explain", "details", "also",
            "additionally", "what else", "expand on"
        ]

        message_lower = message.lower()
        return any(keyword in message_lower for keyword in expansion_keywords)

    def _expand_context(self, query: str):
        """
        Expand the index with new relevant documents.

        Args:
            query: Query to use for finding new context
        """
        print(f"  → Expanding context based on: '{query}'")

        # Load additional documents
        new_docs = self.reader.load_data(
            query,
            budget=self.budget_per_turn // 2  # Use half budget for expansion
        )

        if new_docs:
            print(f"  → Added {len(new_docs)} new documents to context")

            # Insert new documents into the index
            for doc in new_docs:
                self.index.insert(doc)
        else:
            print("  → No new documents found")

    def get_conversation_summary(self) -> Dict[str, Any]:
        """
        Get a summary of the conversation session.

        Returns:
            Dictionary with session statistics
        """
        return {
            "session_id": self.session_id,
            "turns": len(self.conversation_history) // 2,
            "messages": len(self.conversation_history),
            "history": self.conversation_history
        }

    def save_session(self, filename: str):
        """
        Save the conversation session to a file.

        Args:
            filename: File path to save session
        """
        session_data = self.get_conversation_summary()
        with open(filename, 'w') as f:
            json.dump(session_data, f, indent=2)
        print(f"\nSession saved to: {filename}")


def example_technical_qa_session():
    """
    Example: Technical Q&A session about a codebase.

    Demonstrates how context evolves as the conversation
    explores different aspects of the code.
    """
    print("="*70)
    print("Example 1: Technical Q&A Session")
    print("="*70)

    # Initialize reader
    reader = AvocadoDBReader(
        url="http://localhost:8765",
        budget=8000,
        include_citations=True
    )

    # Create conversational index with initial topic
    conv = ConversationalAvocadoIndex(
        reader=reader,
        initial_query="authentication system overview",
        session_id="tech-qa-001",
        budget_per_turn=5000
    )

    # Simulate a conversation
    conversations = [
        "What authentication methods are supported?",
        "How is JWT validation implemented?",
        "Tell me more about the token refresh mechanism",
        "What security measures are in place?",
        "How does the system handle authentication failures?"
    ]

    for message in conversations:
        conv.chat(message)
        print("-" * 70)

    # Show session summary
    summary = conv.get_conversation_summary()
    print(f"\nSession Summary:")
    print(f"  Total turns: {summary['turns']}")
    print(f"  Messages: {summary['messages']}")

    # Save session
    conv.save_session("/tmp/session_tech_qa.json")


def example_code_exploration_session():
    """
    Example: Interactive code exploration session.

    Shows how to progressively explore a codebase,
    drilling down into specific components.
    """
    print("\n" + "="*70)
    print("Example 2: Code Exploration Session")
    print("="*70)

    reader = AvocadoDBReader(
        url="http://localhost:8765",
        budget=10000,
        combine_adjacent=True  # Get more complete context
    )

    conv = ConversationalAvocadoIndex(
        reader=reader,
        initial_query="project architecture and main components",
        session_id="exploration-001",
        budget_per_turn=6000
    )

    # Progressive exploration
    explorations = [
        "What are the main architectural components?",
        "How do these components interact with each other?",
        "What about the database layer?",
        "Explain the API design patterns used",
        "How is error handling implemented across components?"
    ]

    for message in explorations:
        conv.chat(message)
        print("-" * 70)

    # Session statistics
    summary = conv.get_conversation_summary()
    print(f"\nExploration complete!")
    print(f"  Topics covered: {summary['turns']}")


def example_debugging_session():
    """
    Example: Debugging session with targeted context.

    Demonstrates using conversation to narrow down
    to specific code areas for debugging.
    """
    print("\n" + "="*70)
    print("Example 3: Debugging Session")
    print("="*70)

    reader = AvocadoDBReader(
        url="http://localhost:8765",
        budget=8000,
        min_score=0.7  # High-quality matches only
    )

    conv = ConversationalAvocadoIndex(
        reader=reader,
        initial_query="error handling and exception management",
        session_id="debug-001",
        budget_per_turn=5000
    )

    # Debugging conversation
    debug_questions = [
        "Show me how errors are caught and handled",
        "What happens when a database connection fails?",
        "Are there any retry mechanisms?",
        "How are errors logged?",
        "What error information is returned to users?"
    ]

    for question in debug_questions:
        conv.chat(question)
        print("-" * 70)


def example_with_context_refresh():
    """
    Example: Session with periodic context refresh.

    Shows how to refresh context periodically to keep
    the most relevant information in the index.
    """
    print("\n" + "="*70)
    print("Example 4: Context Refresh Strategy")
    print("="*70)

    reader = AvocadoDBReader(url="http://localhost:8765")

    conv = ConversationalAvocadoIndex(
        reader=reader,
        initial_query="API endpoints and routing",
        session_id="refresh-001"
    )

    # After several turns, manually refresh context
    conv.chat("What are the main API endpoints?")
    conv.chat("How is routing configured?")

    # Refresh with new focus
    print("\n→ Refreshing context with new focus...")
    new_docs = reader.load_data(
        "authentication and authorization endpoints",
        budget=5000
    )

    # Update index
    for doc in new_docs:
        conv.index.insert(doc)
    print(f"→ Added {len(new_docs)} documents\n")

    conv.chat("How are protected endpoints secured?")
    conv.chat("What authentication is required?")


def example_multi_session():
    """
    Example: Multiple sessions for different topics.

    Demonstrates managing multiple conversation sessions
    for different purposes.
    """
    print("\n" + "="*70)
    print("Example 5: Multi-Session Management")
    print("="*70)

    reader = AvocadoDBReader(url="http://localhost:8765")

    # Session 1: Frontend
    print("\nSession 1: Frontend Discussion")
    print("-" * 70)
    frontend_session = ConversationalAvocadoIndex(
        reader=reader,
        initial_query="frontend components and UI",
        session_id="frontend-001"
    )
    frontend_session.chat("What UI components are available?")
    frontend_session.chat("How is state managed?")

    # Session 2: Backend
    print("\nSession 2: Backend Discussion")
    print("-" * 70)
    backend_session = ConversationalAvocadoIndex(
        reader=reader,
        initial_query="backend services and API",
        session_id="backend-001"
    )
    backend_session.chat("What backend services exist?")
    backend_session.chat("How do they communicate?")

    # Save both sessions
    frontend_session.save_session("/tmp/session_frontend.json")
    backend_session.save_session("/tmp/session_backend.json")

    print("\n✓ Both sessions saved independently")


def main():
    """Run all conversational examples."""
    print("\n" + "="*70)
    print("AvocadoDB Conversational Index Examples")
    print("="*70)
    print("\nThese examples demonstrate session-based conversations")
    print("with evolving context using AvocadoDB and LlamaIndex.")
    print("\nPrerequisites:")
    print("  1. AvocadoDB server running: avocado-server")
    print("  2. Documents ingested: avocado ingest . --recursive")
    print("  3. OpenAI API key: export OPENAI_API_KEY=...")
    print()

    try:
        # Run examples
        example_technical_qa_session()
        example_code_exploration_session()
        example_debugging_session()
        example_with_context_refresh()
        example_multi_session()

        print("\n" + "="*70)
        print("All examples completed successfully!")
        print("="*70)

    except Exception as e:
        print(f"\n❌ Error: {e}")
        print("\nTroubleshooting:")
        print("  1. Check server: curl http://localhost:8765/stats")
        print("  2. Check ingestion: avocado stats")
        print("  3. Verify API key: echo $OPENAI_API_KEY")


if __name__ == "__main__":
    main()
