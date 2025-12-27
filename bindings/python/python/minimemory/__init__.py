"""
minimemory - Embedded vector database for Python

Like SQLite, but for vector similarity search.

Example:
    >>> from minimemory import VectorDB
    >>> db = VectorDB(dimensions=384)
    >>> db.insert("doc1", [0.1] * 384)
    >>> results = db.search([0.1] * 384, k=10)
"""

from minimemory._minimemory import VectorDB, SearchResult

__all__ = ["VectorDB", "SearchResult"]
__version__ = "0.1.0"
