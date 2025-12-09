"""
Example of session replay for debugging agent behavior.

This demonstrates how to use session replay to understand:
- What queries were made
- What context was retrieved
- How the agent responded
"""

from avocado import AvocadoDB
import json

def main():
    # Initialize client
    db = AvocadoDB(mode="http")

    print("=== Session Replay Example ===\n")

    # Create a session and have a conversation
    print("1. Creating a debugging session...")
    session = db.create_session(
        user_id="debug_user",
        title="Debugging Agent Behavior"
    )

    # Simulate a conversation
    queries = [
        "How does vector search work?",
        "What about hybrid search?",
        "Compare semantic vs lexical search"
    ]

    print("2. Simulating a conversation...")
    for i, query in enumerate(queries, 1):
        print(f"   Query {i}: {query}")

        # Compile and get context
        result = session.compile(query, budget=8000)

        # Simulate assistant response
        session.add_message(
            "assistant",
            f"Response to: {query}"
        )

    print()

    # Now replay the session for debugging
    print("3. Replaying session to debug agent behavior...\n")
    replay = session.replay()

    print("=" * 70)
    print(f"Session: {replay['session']['id']}")
    print(f"User: {replay['session']['user_id']}")
    print(f"Title: {replay['session']['title']}")
    print(f"Created: {replay['session']['created_at']}")
    print("=" * 70)

    # Analyze each turn
    for i, turn in enumerate(replay['turns'], 1):
        print(f"\n{'─' * 70}")
        print(f"TURN {i}")
        print(f"{'─' * 70}")

        # User query
        user_msg = turn['user_message']
        print(f"\nUser Query (seq {user_msg['sequence_number']}):")
        print(f"  \"{user_msg['content']}\"")

        # Context retrieved
        if turn.get('working_set'):
            ws = turn['working_set']
            print("\nContext Retrieved:")
            print(f"  - Token budget: {ws['tokens_used']} tokens")
            print(f"  - Spans retrieved: {len(ws['spans'])}")
            print(f"  - Compilation time: {ws['compilation_time_ms']}ms")
            print(f"  - Query: {ws['query']}")

            # Show top citations
            if ws.get('citations'):
                print("\n  Top Citations:")
                for j, citation in enumerate(ws['citations'][:3], 1):
                    print(f"    {j}. {citation['artifact_path']}:{citation['start_line']}-{citation['end_line']}")
                    print(f"       Score: {citation['score']:.4f}")

            # Show context preview
            print("\n  Context Preview:")
            preview = ws['text'][:200].replace("\n", "\n    ")
            print(f"    {preview}...")

        # Assistant response
        if turn.get('assistant_message'):
            asst_msg = turn['assistant_message']
            print(f"\nAssistant Response (seq {asst_msg['sequence_number']}):")
            print(f"  \"{asst_msg['content']}\"")

    print(f"\n{'=' * 70}")
    print("Replay Complete")
    print(f"{'=' * 70}")

    # Export replay to JSON for further analysis
    print("\n4. Exporting replay to JSON...")
    with open("session_replay.json", "w") as f:
        json.dump(replay, f, indent=2, default=str)
    print("   Saved to session_replay.json")

    # Cleanup
    print("\n5. Cleaning up session...")
    session.delete()
    print("   Done!")


if __name__ == "__main__":
    main()
