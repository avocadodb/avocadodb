#!/usr/bin/env python3
"""
Advanced Example: Agent with Session Memory

This example demonstrates how to build an AI agent that maintains
conversation context using AvocadoDB sessions. The agent:

1. Maintains conversation history across multiple interactions
2. Uses context compilation to retrieve relevant information
3. Demonstrates multi-turn conversations with memory
4. Shows how to handle agent responses and update session state

Use case: Building a chatbot or AI assistant that remembers context.
"""

from avocado import AvocadoDB
import sys


def create_agent_with_memory():
    """Create an agent that maintains conversation memory using sessions."""

    # Initialize AvocadoDB in HTTP mode (required for sessions)
    db = AvocadoDB(mode="http", base_url="http://localhost:8765")

    # Create a new session for this conversation
    print("🤖 Creating new agent session...")
    session = db.create_session(
        user_id="demo_user",
        title="Agent Memory Demo"
    )
    print(f"✓ Session created: {session.id}\n")

    return db, session


def agent_respond(session, user_query, budget=8000):
    """
    Simulate an agent response using session context.

    Args:
        session: The conversation session
        user_query: User's question
        budget: Token budget for context compilation

    Returns:
        dict: Contains compiled context and metadata
    """
    print(f"👤 User: {user_query}")

    # Compile context for the query (automatically adds user message)
    result = session.compile(user_query, budget=budget)

    # Extract compiled context
    working_set = result['working_set']
    context_text = working_set['text']
    tokens_used = working_set['tokens_used']

    print(f"📚 Context compiled: {tokens_used} tokens")

    # In a real agent, you would:
    # 1. Take the context_text
    # 2. Add conversation history: session.get_history(max_tokens=2000)
    # 3. Send to LLM (OpenAI, Claude, etc.)
    # 4. Get response
    # 5. Add response to session

    # Simulate agent response (in real app, this would be LLM-generated)
    agent_response = f"""Based on the context (using {tokens_used} tokens),
I can help you with: {user_query}

[In a real implementation, this would be an LLM response using the compiled context]"""

    # Add agent's response to session
    session.add_message("assistant", agent_response)

    print(f"🤖 Agent: {agent_response}\n")
    print("-" * 80 + "\n")

    return {
        'query': user_query,
        'context': context_text,
        'tokens_used': tokens_used,
        'response': agent_response
    }


def demo_multi_turn_conversation():
    """Demonstrate a multi-turn conversation with memory."""

    print("=" * 80)
    print("Agent Memory Demo: Multi-Turn Conversation")
    print("=" * 80 + "\n")

    # Create agent with session
    db, session = create_agent_with_memory()

    # Simulate a multi-turn conversation
    conversation_turns = [
        "What is AvocadoDB?",
        "How does the compiler work?",
        "Can you explain the caching system?",
        "What are the performance characteristics?"
    ]

    results = []
    for query in conversation_turns:
        result = agent_respond(session, query)
        results.append(result)

    # Show conversation history
    print("\n" + "=" * 80)
    print("Full Conversation History")
    print("=" * 80 + "\n")

    history = session.get_history()
    print(history)

    # Show session replay (for debugging)
    print("\n" + "=" * 80)
    print("Session Replay (Debug View)")
    print("=" * 80 + "\n")

    replay = session.replay()
    print(f"Session: {replay['session']['id']}")
    print(f"Total turns: {len(replay['turns'])}\n")

    for i, turn in enumerate(replay['turns'], 1):
        print(f"Turn {i}:")
        print(f"  User: {turn['user_message']['content'][:60]}...")

        if turn.get('working_set'):
            ws = turn['working_set']
            print(f"  Context: {ws['tokens_used']} tokens, {len(ws['spans'])} spans")

        if turn.get('assistant_message'):
            print(f"  Agent: {turn['assistant_message']['content'][:60]}...")

        print()

    return session


def demo_token_limited_history():
    """Demonstrate token-limited history for large conversations."""

    print("=" * 80)
    print("Agent Memory Demo: Token-Limited History")
    print("=" * 80 + "\n")

    db, session = create_agent_with_memory()

    # Simulate a long conversation
    print("Simulating long conversation...")
    for i in range(20):
        query = f"Question {i+1}: Tell me about topic {i+1}"
        session.compile(query, budget=4000)
        session.add_message("assistant", f"Response to question {i+1}")

    print("✓ Added 20 turns (40 messages)\n")

    # Get full history
    full_history = session.get_history()
    print(f"Full history length: {len(full_history)} characters\n")

    # Get token-limited history (recent messages only)
    limited_history = session.get_history(max_tokens=1000)
    print(f"Limited history (1000 tokens): {len(limited_history)} characters")
    print("\nLimited History Preview:")
    print("-" * 80)
    print(limited_history[:500] + "...")
    print("-" * 80 + "\n")


def demo_session_persistence():
    """Demonstrate session persistence across restarts."""

    print("=" * 80)
    print("Agent Memory Demo: Session Persistence")
    print("=" * 80 + "\n")

    db = AvocadoDB(mode="http")

    # Create a session
    print("Creating session...")
    session = db.create_session(user_id="persistent_user", title="Persistent Session")
    session_id = session.id
    print(f"✓ Created session: {session_id}\n")

    # Add some messages
    session.compile("First question", budget=4000)
    session.add_message("assistant", "First answer")

    print("Added first conversation turn")

    # Simulate app restart by getting session again
    print("\n--- Simulating app restart ---\n")

    # Retrieve existing session
    retrieved_session = db.get_session(session_id)
    print(f"✓ Retrieved session: {retrieved_session.id}")

    # Continue conversation
    retrieved_session.compile("Second question after restart", budget=4000)
    retrieved_session.add_message("assistant", "Second answer")

    print("✓ Continued conversation after restart\n")

    # Show full history
    history = retrieved_session.get_history()
    print("Full conversation history:")
    print("-" * 80)
    print(history)
    print("-" * 80 + "\n")


def demo_multiple_sessions():
    """Demonstrate managing multiple concurrent sessions."""

    print("=" * 80)
    print("Agent Memory Demo: Multiple Sessions")
    print("=" * 80 + "\n")

    db = AvocadoDB(mode="http")

    # Create multiple sessions for different users
    sessions = {}

    for user in ["alice", "bob", "charlie"]:
        session = db.create_session(user_id=user, title=f"{user}'s conversation")
        sessions[user] = session
        print(f"✓ Created session for {user}: {session.id}")

    print()

    # Each user asks questions
    print("Users asking questions...")
    sessions["alice"].compile("Alice's question about feature X", budget=4000)
    sessions["bob"].compile("Bob's question about feature Y", budget=4000)
    sessions["charlie"].compile("Charlie's question about feature Z", budget=4000)

    print("✓ All users sent queries\n")

    # List all sessions
    print("Listing all sessions:")
    all_sessions = db.list_sessions(limit=10)

    for sess_info in all_sessions:
        print(f"  - {sess_info.user_id}: {sess_info.id} (created: {sess_info.created_at})")

    print()

    # List sessions for specific user
    print("Listing sessions for 'alice':")
    alice_sessions = db.list_sessions(user_id="alice")

    for sess_info in alice_sessions:
        print(f"  - {sess_info.id}: {sess_info.title}")

    print()


def main():
    """Run all demos."""

    print("\n🥑 AvocadoDB Session Memory Examples\n")

    demos = [
        ("Multi-Turn Conversation", demo_multi_turn_conversation),
        ("Token-Limited History", demo_token_limited_history),
        ("Session Persistence", demo_session_persistence),
        ("Multiple Sessions", demo_multiple_sessions),
    ]

    print("Available demos:")
    for i, (name, _) in enumerate(demos, 1):
        print(f"  {i}. {name}")
    print(f"  {len(demos) + 1}. Run all demos")
    print()

    try:
        choice = input("Select demo (1-{}): ".format(len(demos) + 1))
        choice = int(choice)

        if choice == len(demos) + 1:
            # Run all demos
            for name, demo_func in demos:
                print("\n\n")
                demo_func()
                input("\nPress Enter to continue to next demo...")
        elif 1 <= choice <= len(demos):
            # Run selected demo
            demos[choice - 1][1]()
        else:
            print("Invalid choice")
            sys.exit(1)

    except KeyboardInterrupt:
        print("\n\nDemo interrupted")
        sys.exit(0)
    except Exception as e:
        print(f"\n❌ Error: {e}")
        print("\nMake sure AvocadoDB server is running:")
        print("  cargo run --bin avocado-server")
        sys.exit(1)


if __name__ == "__main__":
    main()
