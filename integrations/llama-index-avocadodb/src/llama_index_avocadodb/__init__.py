"""
LlamaIndex integration for AvocadoDB.

Provides deterministic, citation-backed document loading for RAG applications.
"""

from llama_index_avocadodb.reader import AvocadoDBReader
from llama_index_avocadodb.index import AvocadoDBIndex

__version__ = "1.0.0"
__all__ = [
    "AvocadoDBReader",
    "AvocadoDBIndex",
]