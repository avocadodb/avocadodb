#!/usr/bin/env python3
"""
Example: Using AvocadoDB with LangChain DeepAgents

This example demonstrates how to integrate AvocadoDB's deterministic
context compilation with DeepAgents for reliable, citation-backed responses.

Requirements:
    pip install deepagents avocadodb

Setup:
    1. Start AvocadoDB server: ./target/release/avocado-server
    2. Ingest documents: ./target/release/avocado ingest test-docs/ --recursive
    3. Set ANTHROPIC_API_KEY or OPENAI_API_KEY
    4. Run: python examples/deepagent_example.py
"""

import os
from deepagents import create_deep_agent
from avocado import avocado_compile_context


def main():
    print("🥑 AvocadoDB + DeepAgents Integration Demo\n")
    print("=" * 60)

    # System prompt for a code documentation assistant
    system_prompt = """You are an expert code documentation assistant.

You have access to a deterministic knowledge base through AvocadoDB.

## `avocado_compile_context`

Use this tool to retrieve relevant context from the codebase. It provides:
- **100% Deterministic**: Same query always returns same context
- **Citation-backed**: Every piece of information has exact line numbers
- **Token efficient**: Optimized to use your token budget effectively

When you receive context from this tool:
1. Read the 'context' field carefully - this contains the relevant information
2. Use the context to provide accurate, detailed answers
3. Cite your sources using the 'citations' field (file path and line numbers)
4. Always synthesize the information naturally - never show raw JSON

The knowledge base contains ingested documentation and code. Use it to answer
questions about the codebase, architecture, APIs, and implementation details.
"""

    # Create deep agent with AvocadoDB tool
    print("\n📦 Creating DeepAgent with AvocadoDB integration...")

    agent = create_deep_agent(
        tools=[avocado_compile_context],
        system_prompt=system_prompt,
    )

    print("✅ Agent created successfully!")

    # Example queries
    queries = [
        "How does authentication work in this system?",
        "Explain the API endpoints available",
        "What testing strategies are documented?",
    ]

    print("\n" + "=" * 60)
    print("🤖 Running example queries...\n")

    for i, query in enumerate(queries, 1):
        print(f"\n{'─' * 60}")
        print(f"Query {i}: {query}")
        print("─" * 60)

        try:
            # Invoke the agent
            result = agent.invoke({"messages": [{"role": "user", "content": query}]})

            # Extract the final response
            messages = result.get("messages", [])
            if messages:
                final_message = messages[-1]
                response = final_message.content if hasattr(final_message, 'content') else str(final_message)
                print(f"\n🤖 Response:\n{response}")
            else:
                print("\n⚠️  No response received")

        except Exception as e:
            print(f"\n❌ Error: {e}")

        print()

    print("\n" + "=" * 60)
    print("✨ Demo Complete!")
    print("\nKey Benefits:")
    print("  ✅ Deterministic responses (same query → same context)")
    print("  ✅ Citation-backed answers (exact line numbers)")
    print("  ✅ Token efficient (90-95% budget utilization)")
    print("  ✅ Seamless DeepAgents integration")
    print()


def simple_example():
    """Minimal example showing the integration."""
    from deepagents import create_deep_agent
    from avocado import avocado_compile_context

    # Create agent with AvocadoDB tool
    agent = create_deep_agent(
        tools=[avocado_compile_context],
        system_prompt="You are a helpful assistant with access to a deterministic knowledge base.",
    )

    # Use it
    result = agent.invoke({
        "messages": [{"role": "user", "content": "What documentation is available?"}]
    })

    return result


if __name__ == "__main__":
    # Check if server is running
    from avocado import AvocadoDB

    try:
        db = AvocadoDB()
        stats = db.stats()
        print(f"✅ AvocadoDB server is running ({stats['spans']} spans available)\n")
    except Exception as e:
        print("⚠️  AvocadoDB server not reachable!")
        print("\nMake sure to:")
        print("  1. Start server: ./target/release/avocado-server")
        print("  2. Ingest docs: ./target/release/avocado ingest test-docs/ --recursive")
        print()
        exit(1)

    # Run the demo
    main()
