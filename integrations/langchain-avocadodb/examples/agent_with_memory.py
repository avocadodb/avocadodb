"""
LangChain Agent with AvocadoDB for persistent memory and tool use.

This example demonstrates:
1. Creating an agent that can use AvocadoDB as a search tool
2. Combining retrieval with other tools (calculator, etc.)
3. Agent reasoning with access to your codebase
4. Multi-turn conversations with tool use

Agents can decide when to search the codebase vs. use other tools,
making them more flexible than simple retrieval chains.
"""

from langchain_avocadodb import AvocadoDBRetriever
from langchain.agents import create_openai_tools_agent, AgentExecutor
from langchain.tools.retriever import create_retriever_tool
from langchain_openai import ChatOpenAI
from langchain.prompts import ChatPromptTemplate, MessagesPlaceholder
from langchain.memory import ConversationBufferMemory
from langchain_core.messages import HumanMessage, AIMessage
import sys


def create_codebase_search_tool():
    """
    Create a retriever tool for searching the codebase.

    Returns:
        LangChain Tool that uses AvocadoDB for search
    """
    # Initialize AvocadoDB retriever
    retriever = AvocadoDBRetriever(
        url="http://localhost:8765",
        budget=6000,  # Moderate budget for agent use
        include_citations=True,
        enable_mmr=True,  # Diverse results for better agent decisions
        mmr_lambda=0.4,  # Favor diversity
    )

    # Create a tool from the retriever
    # The agent will use this tool when it needs to search the codebase
    tool = create_retriever_tool(
        retriever,
        name="codebase_search",
        description=(
            "Searches the codebase for information about implementation details, "
            "code structure, APIs, functions, classes, and documentation. "
            "Use this when you need to find specific code or understand how "
            "something is implemented. Input should be a search query."
        ),
    )

    return tool


def create_agent_with_tools():
    """
    Create an agent with AvocadoDB search and other tools.

    Returns:
        Configured AgentExecutor
    """
    # Initialize LLM for the agent
    llm = ChatOpenAI(
        model="gpt-4",  # GPT-4 is better for agent reasoning
        temperature=0,  # Deterministic for consistent behavior
    )

    # Create tools for the agent
    tools = [
        create_codebase_search_tool(),
        # You can add more tools here:
        # - Calculator for math
        # - API calling tools
        # - Database query tools
        # - etc.
    ]

    # Create prompt template for the agent
    # This instructs the agent on how to use the tools
    prompt = ChatPromptTemplate.from_messages(
        [
            (
                "system",
                """You are a helpful AI assistant with access to a codebase search tool.

You can:
1. Search the codebase for implementation details, code examples, and documentation
2. Answer questions about code structure and architecture
3. Help debug issues by finding relevant code
4. Explain how different parts of the system work together

When searching the codebase:
- Be specific in your search queries
- Use the tool multiple times if needed to gather complete information
- Cite the sources (files and line numbers) in your answers

Always provide clear, accurate answers with proper attribution to source code.""",
            ),
            MessagesPlaceholder(variable_name="chat_history", optional=True),
            ("user", "{input}"),
            MessagesPlaceholder(variable_name="agent_scratchpad"),
        ]
    )

    # Create the agent
    agent = create_openai_tools_agent(llm, tools, prompt)

    # Create agent executor with memory
    memory = ConversationBufferMemory(
        memory_key="chat_history",
        return_messages=True,
    )

    agent_executor = AgentExecutor(
        agent=agent,
        tools=tools,
        memory=memory,
        verbose=True,  # Shows agent reasoning
        max_iterations=5,  # Limit iterations to prevent infinite loops
        handle_parsing_errors=True,  # Gracefully handle errors
    )

    return agent_executor


def run_interactive_agent():
    """
    Run an interactive agent session.

    The agent can search the codebase and answer questions,
    maintaining conversation history across turns.
    """
    print("=" * 60)
    print("LangChain Agent with AvocadoDB Memory")
    print("=" * 60)
    print("\nThis agent can search your codebase and answer questions.")
    print("It will show its reasoning and tool use as it works.")
    print("\nType 'quit' or 'exit' to end.")
    print("Type 'clear' to reset conversation history.")
    print("=" * 60)

    # Create the agent
    try:
        agent_executor = create_agent_with_tools()
    except Exception as e:
        print(f"\nError initializing agent: {e}")
        print("\nMake sure:")
        print("1. AvocadoDB server is running (avocado-server)")
        print("2. You've ingested documents (avocado ingest . --recursive)")
        print("3. OPENAI_API_KEY environment variable is set")
        sys.exit(1)

    # Main interaction loop
    turn = 0

    while True:
        print("\n" + "-" * 60)

        # Get user input
        try:
            user_input = input("\nYou: ").strip()
        except (EOFError, KeyboardInterrupt):
            print("\n\nGoodbye!")
            break

        # Handle special commands
        if user_input.lower() in ["quit", "exit"]:
            print("\nGoodbye!")
            break

        if user_input.lower() == "clear":
            agent_executor.memory.clear()
            turn = 0
            print("\nConversation history cleared.")
            continue

        if not user_input:
            continue

        turn += 1
        print(f"\n[Turn {turn}]\n")

        # Execute the agent
        try:
            result = agent_executor.invoke({"input": user_input})

            print(f"\n{'='*60}")
            print("FINAL ANSWER")
            print("=" * 60)
            print(f"\n{result['output']}")

        except Exception as e:
            print(f"\nError: {e}")
            print("Please try rephrasing your question.")


def example_agent_tasks():
    """
    Demonstrate the agent with example tasks.

    Shows different types of questions the agent can handle.
    """
    print("\n" + "=" * 60)
    print("Agent Task Examples")
    print("=" * 60)

    agent_executor = create_agent_with_tools()

    # Example tasks that benefit from agent reasoning
    tasks = [
        {
            "task": "Find the authentication implementation and explain how it works",
            "description": "Agent will search codebase and synthesize explanation",
        },
        {
            "task": "What database is used and how is connection pooling configured?",
            "description": "Multi-part question requiring multiple searches",
        },
        {
            "task": "Are there any error handling patterns I should follow?",
            "description": "Open-ended question requiring code analysis",
        },
    ]

    for i, example in enumerate(tasks, 1):
        print(f"\n{'='*60}")
        print(f"Example {i}: {example['task']}")
        print(f"Goal: {example['description']}")
        print("=" * 60)

        try:
            result = agent_executor.invoke({"input": example["task"]})
            print(f"\nAgent's Answer:\n{result['output']}")

        except Exception as e:
            print(f"\nError: {e}")

        # Pause between examples
        if i < len(tasks):
            input("\nPress Enter to continue to next example...")


def example_with_custom_tools():
    """
    Example showing how to add custom tools alongside AvocadoDB search.

    This demonstrates the flexibility of the agent approach.
    """
    print("\n" + "=" * 60)
    print("Agent with Custom Tools")
    print("=" * 60)

    from langchain.tools import Tool

    # Create a custom tool (example: code analysis)
    def analyze_complexity(code_snippet: str) -> str:
        """Simple code complexity analyzer (mock example)."""
        lines = code_snippet.split("\n")
        return f"Analysis: {len(lines)} lines, appears to be {'complex' if len(lines) > 20 else 'simple'}"

    complexity_tool = Tool(
        name="code_complexity_analyzer",
        func=analyze_complexity,
        description="Analyzes code complexity. Input should be a code snippet.",
    )

    # Create LLM and tools
    llm = ChatOpenAI(model="gpt-4", temperature=0)

    tools = [
        create_codebase_search_tool(),
        complexity_tool,
        # Add more custom tools as needed
    ]

    # Create prompt
    prompt = ChatPromptTemplate.from_messages(
        [
            (
                "system",
                "You are a code analysis assistant with search and analysis tools.",
            ),
            MessagesPlaceholder(variable_name="chat_history", optional=True),
            ("user", "{input}"),
            MessagesPlaceholder(variable_name="agent_scratchpad"),
        ]
    )

    # Create agent
    agent = create_openai_tools_agent(llm, tools, prompt)
    agent_executor = AgentExecutor(agent=agent, tools=tools, verbose=True)

    # Example task
    task = "Find the main database connection code and analyze its complexity"
    print(f"\nTask: {task}\n")

    result = agent_executor.invoke({"input": task})
    print(f"\nResult:\n{result['output']}")


def main():
    """Main entry point."""
    import sys

    if len(sys.argv) > 1:
        if sys.argv[1] == "--demo":
            example_agent_tasks()
        elif sys.argv[1] == "--custom":
            example_with_custom_tools()
        else:
            print("Usage: python agent_with_memory.py [--demo|--custom]")
            sys.exit(1)
    else:
        run_interactive_agent()


if __name__ == "__main__":
    main()
