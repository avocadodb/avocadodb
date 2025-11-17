"""
LangChain integration for AvocadoDB.

Provides deterministic, citation-backed retrieval for RAG applications.
"""

from langchain_avocadodb.retriever import AvocadoDBRetriever
from langchain_avocadodb.vectorstore import AvocadoDBVectorStore

__version__ = "1.0.0"
__all__ = [
    "AvocadoDBRetriever",
    "AvocadoDBVectorStore",
]