"""
minimemory SDK Types
"""

from __future__ import annotations

from enum import Enum
from typing import Any, Literal

from pydantic import BaseModel, ConfigDict, Field


class MemoryType(str, Enum):
    """Memory type enumeration."""

    EPISODIC = "episodic"
    SEMANTIC = "semantic"
    WORKING = "working"
    KNOWLEDGE = "knowledge"


class KnowledgeSourceType(str, Enum):
    """Knowledge source type enumeration."""

    DOCUMENT = "document"
    URL = "url"
    API = "api"
    MANUAL = "manual"


class SearchMode(str, Enum):
    """Search mode enumeration."""

    VECTOR = "vector"
    KEYWORD = "keyword"
    HYBRID = "hybrid"


# Type aliases
EmbeddingDimensions = Literal[768, 512, 256, 128]


class Memory(BaseModel):
    """Memory object."""

    id: str
    type: MemoryType
    content: str
    importance: float
    metadata: dict[str, Any] | None = None
    created_at: int = Field(alias="createdAt")
    last_accessed: int | None = Field(default=None, alias="lastAccessed")
    access_count: int | None = Field(default=None, alias="accessCount")

    model_config = ConfigDict(populate_by_name=True)


class SourceCitation(BaseModel):
    """Source citation for knowledge memories."""

    id: str
    name: str
    type: KnowledgeSourceType
    url: str | None = None
    chunk_index: int = Field(alias="chunkIndex")
    total_chunks: int = Field(alias="totalChunks")

    model_config = ConfigDict(populate_by_name=True)


class RecallResult(BaseModel):
    """Search result item."""

    id: str
    type: MemoryType
    content: str
    score: float
    vector_similarity: float | None = Field(default=None, alias="vectorSimilarity")
    keyword_score: float | None = Field(default=None, alias="keywordScore")
    importance: float
    metadata: dict[str, Any] | None = None
    created_at: int = Field(alias="createdAt")
    source: SourceCitation | None = None

    model_config = ConfigDict(populate_by_name=True)


class MemoryStats(BaseModel):
    """Memory statistics."""

    total: int
    by_type: dict[str, int] = Field(alias="byType")
    average_importance: float = Field(alias="averageImportance")
    oldest_memory: int | None = Field(default=None, alias="oldestMemory")
    newest_memory: int | None = Field(default=None, alias="newestMemory")
    knowledge_sources: int | None = Field(default=None, alias="knowledgeSources")

    model_config = ConfigDict(populate_by_name=True)


class KnowledgeSource(BaseModel):
    """Knowledge source metadata."""

    id: str
    name: str
    type: KnowledgeSourceType
    url: str | None = None
    mime_type: str | None = Field(default=None, alias="mimeType")
    size: int | None = None
    chunk_count: int = Field(alias="chunkCount")
    namespace: str
    metadata: dict[str, Any]
    created_at: int = Field(alias="createdAt")
    updated_at: int = Field(alias="updatedAt")

    model_config = ConfigDict(populate_by_name=True)


class KnowledgeChunk(BaseModel):
    """Knowledge chunk."""

    id: str
    content: str
    chunk_index: int = Field(alias="chunkIndex")
    start_offset: int | None = Field(default=None, alias="startOffset")
    end_offset: int | None = Field(default=None, alias="endOffset")
    created_at: int = Field(alias="createdAt")

    model_config = ConfigDict(populate_by_name=True)


class KnowledgeStats(BaseModel):
    """Knowledge bank statistics."""

    total_sources: int = Field(alias="totalSources")
    total_chunks: int = Field(alias="totalChunks")
    by_type: dict[str, int] = Field(alias="byType")
    average_chunks_per_source: float = Field(alias="averageChunksPerSource")

    model_config = ConfigDict(populate_by_name=True)


class ChunkingOptions(BaseModel):
    """Chunking options for document ingestion."""

    chunk_size: int | None = Field(default=None, alias="chunkSize")
    chunk_overlap: int | None = Field(default=None, alias="chunkOverlap")
    separators: list[str] | None = None
    preserve_paragraphs: bool | None = Field(default=None, alias="preserveParagraphs")

    model_config = ConfigDict(populate_by_name=True)


class IngestResult(BaseModel):
    """Document ingestion result."""

    source_id: str = Field(alias="sourceId")
    source_name: str = Field(alias="sourceName")
    chunks_created: int = Field(alias="chunksCreated")
    embeddings_generated: bool = Field(alias="embeddingsGenerated")
    total_characters: int = Field(alias="totalCharacters")
    average_chunk_size: int = Field(alias="averageChunkSize")
    duration_ms: int = Field(alias="durationMs")

    model_config = ConfigDict(populate_by_name=True)


class EmbedResult(BaseModel):
    """Single embedding result."""

    embedding: list[float]
    dimensions: int
    model: str
    truncated: bool | None = None


class EmbedBatchResult(BaseModel):
    """Batch embedding result."""

    embeddings: list[list[float]]
    dimensions: int
    model: str
    count: int


class EmbedInfo(BaseModel):
    """Embedding service information."""

    available: bool
    model: str
    dimensions: dict[str, Any]
    matryoshka: bool
    pricing: dict[str, Any] | None = None
    estimated_costs: dict[str, str] | None = Field(default=None, alias="estimatedCosts")

    model_config = ConfigDict(populate_by_name=True)


# ============ Agent Token Types ============


class AgentPermission(str, Enum):
    """Agent permission enumeration."""

    READ = "read"
    WRITE = "write"


class AgentToken(BaseModel):
    """Agent token for MCP access control."""

    id: str
    user_id: str = Field(alias="userId")
    tenant_id: str | None = Field(default=None, alias="tenantId")
    name: str
    description: str | None = None
    allowed_memories: list[str] = Field(alias="allowedMemories")
    permissions: list[AgentPermission]
    is_active: bool = Field(alias="isActive")
    last_used_at: int | None = Field(default=None, alias="lastUsedAt")
    use_count: int = Field(alias="useCount")
    expires_at: int | None = Field(default=None, alias="expiresAt")
    created_at: int = Field(alias="createdAt")
    updated_at: int = Field(alias="updatedAt")

    model_config = ConfigDict(populate_by_name=True)


class AgentTokenStats(BaseModel):
    """Agent token usage statistics."""

    total: int
    active: int
    inactive: int
    expired: int
    total_use_count: int = Field(alias="totalUseCount")

    model_config = ConfigDict(populate_by_name=True)


class AgentValidationResult(BaseModel):
    """Result of agent token validation."""

    valid: bool
    user_id: str | None = Field(default=None, alias="userId")
    tenant_id: str | None = Field(default=None, alias="tenantId")
    agent_token_id: str | None = Field(default=None, alias="agentTokenId")
    agent_name: str | None = Field(default=None, alias="agentName")
    allowed_memories: list[str] | None = Field(default=None, alias="allowedMemories")
    permissions: list[AgentPermission] | None = None
    expires_at: int | None = Field(default=None, alias="expiresAt")
    error: str | None = None

    model_config = ConfigDict(populate_by_name=True)
