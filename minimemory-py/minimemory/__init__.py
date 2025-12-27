"""
minimemory SDK - Python client for minimemory vector database service.

Example:
    ```python
    from minimemory import MiniMemoryClient

    async with MiniMemoryClient(
        base_url="https://your-worker.workers.dev/api/v1",
        api_key="mm_your_api_key"
    ) as client:
        # Store a memory
        memory, _, _ = await client.remember("User prefers dark mode")

        # Search memories
        results, _ = await client.recall("user preferences")
    ```
"""

from __future__ import annotations

from .client import AgentTokensAPI, EmbedAPI, KnowledgeAPI, MiniMemoryClient
from .exceptions import (
    AuthenticationError,
    MiniMemoryError,
    NetworkError,
    NotFoundError,
    RateLimitError,
    TimeoutError,
    ValidationError,
)
from .types import (
    AgentPermission,
    AgentToken,
    AgentTokenStats,
    AgentValidationResult,
    ChunkingOptions,
    EmbedBatchResult,
    EmbedInfo,
    EmbedResult,
    EmbeddingDimensions,
    IngestResult,
    KnowledgeChunk,
    KnowledgeSource,
    KnowledgeSourceType,
    KnowledgeStats,
    Memory,
    MemoryStats,
    MemoryType,
    RecallResult,
    SearchMode,
    SourceCitation,
)

__version__ = "0.1.0"

__all__ = [
    # Client
    "MiniMemoryClient",
    "KnowledgeAPI",
    "EmbedAPI",
    "AgentTokensAPI",
    # Exceptions
    "MiniMemoryError",
    "AuthenticationError",
    "NotFoundError",
    "RateLimitError",
    "ValidationError",
    "TimeoutError",
    "NetworkError",
    # Types - Enums
    "MemoryType",
    "KnowledgeSourceType",
    "SearchMode",
    "EmbeddingDimensions",
    "AgentPermission",
    # Types - Models
    "Memory",
    "RecallResult",
    "MemoryStats",
    "SourceCitation",
    "KnowledgeSource",
    "KnowledgeChunk",
    "KnowledgeStats",
    "ChunkingOptions",
    "IngestResult",
    "EmbedResult",
    "EmbedBatchResult",
    "EmbedInfo",
    # Agent Token types
    "AgentToken",
    "AgentTokenStats",
    "AgentValidationResult",
]
