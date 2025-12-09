"""
Example of using AvocadoDB sessions for multi-turn conversations.

This demonstrates:
- Creating a new session
- Adding messages and compiling context
- Retrieving conversation history
- Session replay for debugging
"""

from avocado import AvocadoDB

def main():
    # Initialize client in HTTP mode (sessions require HTTP server)
    # Make sure the AvocadoDB server is running: `avocado-server`
    db = AvocadoDB(mode="http")

    print("=== AvocadoDB Session Example ===\n")

    # Create a new session
    print("1. Creating a new session...")
    session = db.create_session(
        user_id="alice",
        title="Learning about Rust"
    )
    print(f"   Created session: {session.session_id}")
    print(f"   User: {session.info.user_id}")
    print(f"   Title: {session.info.title}\n")

    # First query - compile context and add user message
    print("2. First query: What is Rust?")
    result1 = session.compile("What is Rust?", budget=8000)

    print(f"   User message added: {result1['message'].id}")
    print(f"   Context compiled: {len(result1['working_set']['spans'])} spans")
    print(f"   Context preview: {result1['working_set']['text'][:200]}...\n")

    # Add assistant response
    print("3. Adding assistant response...")
    assistant_msg1 = session.add_message(
        "assistant",
        "Rust is a systems programming language that runs blazingly fast, "
        "prevents segfaults, and guarantees thread safety."
    )
    print(f"   Assistant message added: {assistant_msg1.id}\n")

    # Second query
    print("4. Second query: Tell me about ownership")
    result2 = session.compile("Tell me about ownership in Rust", budget=8000)

    print(f"   User message added: {result2['message'].id}")
    print(f"   Context compiled: {len(result2['working_set']['spans'])} spans\n")

    # Add another assistant response
    assistant_msg2 = session.add_message(
        "assistant",
        "Ownership is Rust's most unique feature. It enables Rust to make "
        "memory safety guarantees without needing a garbage collector."
    )
    print(f"   Assistant message added: {assistant_msg2.id}\n")

    # Get conversation history
    print("5. Retrieving conversation history...")
    history = session.get_history()
    print("   Full conversation history:")
    print("   " + "-" * 60)
    for line in history.split("\n"):
        print(f"   {line}")
    print("   " + "-" * 60 + "\n")

    # Get messages directly
    print("6. Getting all messages...")
    messages = session.get_messages()
    print(f"   Total messages: {len(messages)}")
    for msg in messages:
        print(f"   [{msg.sequence_number}] {msg.role}: {msg.content[:50]}...")
    print()

    # Replay session for debugging
    print("7. Replaying session (for debugging)...")
    replay = session.replay()
    print(f"   Session ID: {replay['session']['id']}")
    print(f"   Total turns: {len(replay['turns'])}")

    for i, turn in enumerate(replay['turns'], 1):
        print(f"\n   Turn {i}:")
        print(f"   - User: {turn['user_message']['content']}")

        if turn.get('working_set'):
            ws = turn['working_set']
            print(f"   - Context: {ws['tokens_used']} tokens, {len(ws['spans'])} spans")

        if turn.get('assistant_message'):
            print(f"   - Assistant: {turn['assistant_message']['content'][:60]}...")

    print("\n8. Listing all sessions...")
    sessions = db.list_sessions(user_id="alice")
    print(f"   Found {len(sessions)} sessions for user 'alice'")
    for s in sessions:
        print(f"   - {s['id']}: {s.get('title', '(no title)')}")

    # Cleanup (optional)
    print(f"\n9. Deleting session {session.session_id}...")
    success = session.delete()
    print(f"   Deleted: {success}")

    print("\n=== Example Complete ===")


if __name__ == "__main__":
    main()
