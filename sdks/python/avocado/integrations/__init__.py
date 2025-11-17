"""Framework integrations for AvocadoDB.

Provides ready-to-use integrations for popular agent frameworks:
- LangChain / DeepAgents
- AutoGen
- CrewAI
- LlamaIndex

Each integration provides:
- Tool wrapper (avocado_compile_context)
- Framework-specific configuration
- Optional middleware (for blocking parallel tools)

Example:
    >>> # LangChain
    >>> from avocado.integrations.langchain import AvocadoDBTool, AvocadoDBMiddleware
    >>> agent = create_agent(tools=[AvocadoDBTool()], middleware=[AvocadoDBMiddleware()])

    >>> # AutoGen
    >>> from avocado.integrations.autogen import avocado_compile_context
    >>> agent = AssistantAgent(tools=[avocado_compile_context])
"""

__all__ = []
