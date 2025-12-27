"""
minimemory SDK Client
"""

from __future__ import annotations

from typing import Any

import httpx

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
)


class KnowledgeAPI:
    """Knowledge Bank API operations."""

    def __init__(self, client: "MiniMemoryClient") -> None:
        self._client = client

    async def ingest(
        self,
        content: str,
        name: str,
        *,
        type: KnowledgeSourceType = KnowledgeSourceType.DOCUMENT,
        url: str | None = None,
        mime_type: str | None = None,
        metadata: dict[str, Any] | None = None,
        chunking: ChunkingOptions | None = None,
        generate_embeddings: bool = True,
    ) -> IngestResult:
        """Ingest a document into the knowledge bank."""
        body: dict[str, Any] = {
            "content": content,
            "name": name,
            "type": type.value,
            "generateEmbeddings": generate_embeddings,
        }
        if url:
            body["url"] = url
        if mime_type:
            body["mimeType"] = mime_type
        if metadata:
            body["metadata"] = metadata
        if chunking:
            body["chunking"] = chunking.model_dump(by_alias=True, exclude_none=True)

        data = await self._client._request("POST", "/knowledge/ingest", body)
        return IngestResult.model_validate(data)

    async def list_sources(
        self,
        *,
        type: KnowledgeSourceType | None = None,
        limit: int = 100,
        offset: int = 0,
    ) -> tuple[list[KnowledgeSource], int, bool]:
        """List knowledge sources."""
        params: dict[str, Any] = {"limit": limit, "offset": offset}
        if type:
            params["type"] = type.value

        data = await self._client._request("GET", "/knowledge/sources", params=params)
        sources = [KnowledgeSource.model_validate(s) for s in data.get("sources", [])]
        return sources, data.get("total", 0), data.get("hasMore", False)

    async def get_source(self, source_id: str) -> KnowledgeSource:
        """Get a specific knowledge source."""
        data = await self._client._request("GET", f"/knowledge/sources/{source_id}")
        return KnowledgeSource.model_validate(data["source"])

    async def delete_source(self, source_id: str) -> bool:
        """Delete a knowledge source and all its chunks."""
        data = await self._client._request("DELETE", f"/knowledge/sources/{source_id}")
        return data.get("success", False)

    async def get_chunks(
        self,
        source_id: str,
        *,
        limit: int = 100,
        offset: int = 0,
    ) -> tuple[list[KnowledgeChunk], int, bool]:
        """Get chunks for a specific source."""
        params = {"limit": limit, "offset": offset}
        data = await self._client._request(
            "GET", f"/knowledge/sources/{source_id}/chunks", params=params
        )
        chunks = [KnowledgeChunk.model_validate(c) for c in data.get("chunks", [])]
        return chunks, data.get("total", 0), data.get("hasMore", False)

    async def stats(self) -> KnowledgeStats:
        """Get knowledge bank statistics."""
        data = await self._client._request("GET", "/knowledge/stats")
        return KnowledgeStats.model_validate(data["stats"])

    async def preview_chunking(
        self,
        content: str,
        chunking: ChunkingOptions | None = None,
    ) -> dict[str, Any]:
        """Preview how content will be chunked."""
        body: dict[str, Any] = {"content": content}
        if chunking:
            body["chunking"] = chunking.model_dump(by_alias=True, exclude_none=True)

        return await self._client._request("POST", "/knowledge/chunk-preview", body)


class EmbedAPI:
    """Embedding API operations."""

    def __init__(self, client: "MiniMemoryClient") -> None:
        self._client = client

    async def single(
        self,
        text: str,
        *,
        dimensions: EmbeddingDimensions = 768,
    ) -> EmbedResult:
        """Generate embedding for a single text."""
        data = await self._client._request(
            "POST", "/embed", {"text": text, "dimensions": dimensions}
        )
        return EmbedResult.model_validate(data)

    async def batch(
        self,
        texts: list[str],
        *,
        dimensions: EmbeddingDimensions = 768,
    ) -> EmbedBatchResult:
        """Generate embeddings for multiple texts."""
        data = await self._client._request(
            "POST", "/embed", {"texts": texts, "dimensions": dimensions}
        )
        return EmbedBatchResult.model_validate(data)

    async def info(self) -> EmbedInfo:
        """Get embedding service information."""
        data = await self._client._request("GET", "/embed/info")
        return EmbedInfo.model_validate(data)


class AgentTokensAPI:
    """Agent Token API operations for MCP access control.

    Requires JWT authentication (use set_access_token).
    """

    def __init__(self, client: "MiniMemoryClient") -> None:
        self._client = client

    async def list(
        self,
        *,
        active: bool | None = None,
        limit: int = 100,
        offset: int = 0,
    ) -> tuple[list[AgentToken], int, bool]:
        """
        List all agent tokens for the authenticated user.

        Args:
            active: Filter by active status
            limit: Max results (default: 100)
            offset: Pagination offset

        Returns:
            Tuple of (tokens, total, has_more)
        """
        params: dict[str, Any] = {"limit": limit, "offset": offset}
        if active is not None:
            params["active"] = str(active).lower()

        data = await self._client._request("GET", "/agent-tokens", params=params)
        tokens = [AgentToken.model_validate(t) for t in data.get("tokens", [])]
        return tokens, data.get("total", 0), data.get("hasMore", False)

    async def create(
        self,
        name: str,
        *,
        description: str | None = None,
        allowed_memories: list[str] | None = None,
        permissions: list[AgentPermission] | None = None,
        expires_at: int | None = None,
    ) -> AgentToken:
        """
        Create a new agent token.

        Args:
            name: Token name (max 100 chars)
            description: Optional description
            allowed_memories: Memory IDs or ["*"] for all (default: ["*"])
            permissions: ["read"], ["write"], or both (default: both)
            expires_at: Optional expiration timestamp in ms

        Returns:
            The created agent token
        """
        body: dict[str, Any] = {"name": name}
        if description:
            body["description"] = description
        if allowed_memories:
            body["allowedMemories"] = allowed_memories
        if permissions:
            body["permissions"] = [p.value for p in permissions]
        if expires_at:
            body["expiresAt"] = expires_at

        data = await self._client._request("POST", "/agent-tokens", body)
        return AgentToken.model_validate(data["token"])

    async def get(self, token_id: str) -> AgentToken:
        """Get a specific agent token by ID."""
        data = await self._client._request("GET", f"/agent-tokens/{token_id}")
        return AgentToken.model_validate(data["token"])

    async def update(
        self,
        token_id: str,
        *,
        name: str | None = None,
        description: str | None = None,
        allowed_memories: list[str] | None = None,
        permissions: list[AgentPermission] | None = None,
        is_active: bool | None = None,
        expires_at: int | None = None,
    ) -> AgentToken:
        """
        Update an agent token.

        Args:
            token_id: Agent token ID
            name: New name
            description: New description
            allowed_memories: New allowed memories list
            permissions: New permissions
            is_active: Active status
            expires_at: New expiration (use 0 or None to clear)

        Returns:
            The updated agent token
        """
        body: dict[str, Any] = {}
        if name is not None:
            body["name"] = name
        if description is not None:
            body["description"] = description
        if allowed_memories is not None:
            body["allowedMemories"] = allowed_memories
        if permissions is not None:
            body["permissions"] = [p.value for p in permissions]
        if is_active is not None:
            body["isActive"] = is_active
        if expires_at is not None:
            body["expiresAt"] = expires_at if expires_at > 0 else None

        data = await self._client._request("PATCH", f"/agent-tokens/{token_id}", body)
        return AgentToken.model_validate(data["token"])

    async def delete(self, token_id: str) -> bool:
        """Delete an agent token."""
        data = await self._client._request("DELETE", f"/agent-tokens/{token_id}")
        return data.get("success", False)

    async def toggle(self, token_id: str) -> AgentToken:
        """Toggle agent token active status."""
        data = await self._client._request("POST", f"/agent-tokens/{token_id}/toggle")
        return AgentToken.model_validate(data["token"])

    async def add_memory(self, token_id: str, memory_id: str) -> AgentToken:
        """Add a memory ID to the token's allowed list."""
        data = await self._client._request(
            "POST", f"/agent-tokens/{token_id}/add-memory", {"memoryId": memory_id}
        )
        return AgentToken.model_validate(data["token"])

    async def remove_memory(self, token_id: str, memory_id: str) -> AgentToken:
        """Remove a memory ID from the token's allowed list."""
        data = await self._client._request(
            "POST", f"/agent-tokens/{token_id}/remove-memory", {"memoryId": memory_id}
        )
        return AgentToken.model_validate(data["token"])

    async def stats(self) -> AgentTokenStats:
        """Get usage statistics for the user's tokens."""
        data = await self._client._request("GET", "/agent-tokens/stats")
        return AgentTokenStats.model_validate(data["stats"])

    async def validate(self, api_key: str, agent_token: str) -> AgentValidationResult:
        """
        Validate an API key + agent token combination.

        Used by MCP servers to verify agent credentials.

        Args:
            api_key: The API key
            agent_token: The agent token ID

        Returns:
            Validation result with permissions
        """
        data = await self._client._request(
            "POST", "/auth/validate-agent", {"apiKey": api_key, "agentToken": agent_token}
        )
        return AgentValidationResult.model_validate(data)


class MiniMemoryClient:
    """
    Async client for the minimemory vector database service.

    Example:
        ```python
        from minimemory import MiniMemoryClient

        async with MiniMemoryClient(
            base_url="https://your-worker.workers.dev/api/v1",
            api_key="mm_your_api_key"
        ) as client:
            # Store a memory
            result = await client.remember("User prefers dark mode")

            # Search memories
            results = await client.recall("user preferences")
        ```
    """

    def __init__(
        self,
        base_url: str,
        *,
        api_key: str | None = None,
        access_token: str | None = None,
        namespace: str = "default",
        timeout: float = 30.0,
        headers: dict[str, str] | None = None,
    ) -> None:
        """
        Initialize the client.

        Args:
            base_url: Base URL of the minimemory service API
            api_key: API key for authentication
            access_token: JWT access token for authentication
            namespace: Default namespace for operations
            timeout: Request timeout in seconds
            headers: Additional headers for all requests
        """
        self.base_url = base_url.rstrip("/")
        self.api_key = api_key
        self.access_token = access_token
        self.namespace = namespace
        self.timeout = timeout
        self._headers = headers or {}
        self._client: httpx.AsyncClient | None = None

        # Sub-APIs
        self.knowledge = KnowledgeAPI(self)
        self.embed = EmbedAPI(self)
        self.agent_tokens = AgentTokensAPI(self)

    async def __aenter__(self) -> "MiniMemoryClient":
        await self._ensure_client()
        return self

    async def __aexit__(self, *args: Any) -> None:
        await self.close()

    async def _ensure_client(self) -> httpx.AsyncClient:
        if self._client is None:
            self._client = httpx.AsyncClient(timeout=self.timeout)
        return self._client

    async def close(self) -> None:
        """Close the HTTP client."""
        if self._client:
            await self._client.aclose()
            self._client = None

    def set_namespace(self, namespace: str) -> "MiniMemoryClient":
        """Set the active namespace."""
        self.namespace = namespace
        return self

    def set_api_key(self, api_key: str) -> "MiniMemoryClient":
        """Set the API key."""
        self.api_key = api_key
        return self

    def set_access_token(self, token: str) -> "MiniMemoryClient":
        """Set the JWT access token."""
        self.access_token = token
        return self

    def _build_headers(self) -> dict[str, str]:
        headers = {
            "Content-Type": "application/json",
            "X-Namespace": self.namespace,
            **self._headers,
        }
        if self.api_key:
            headers["X-API-Key"] = self.api_key
        if self.access_token:
            headers["Authorization"] = f"Bearer {self.access_token}"
        return headers

    async def _request(
        self,
        method: str,
        path: str,
        body: dict[str, Any] | None = None,
        params: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        client = await self._ensure_client()
        url = f"{self.base_url}{path}"
        headers = self._build_headers()

        try:
            if method == "GET":
                response = await client.get(url, headers=headers, params=params)
            elif method == "POST":
                response = await client.post(url, headers=headers, json=body)
            elif method == "PATCH":
                response = await client.patch(url, headers=headers, json=body)
            elif method == "DELETE":
                response = await client.delete(url, headers=headers)
            else:
                raise ValueError(f"Unsupported method: {method}")

            data = response.json()

            if not response.is_success:
                error_msg = data.get("error", f"HTTP {response.status_code}")
                self._raise_for_status(response.status_code, error_msg)

            return data

        except httpx.TimeoutException as e:
            raise TimeoutError(str(e)) from e
        except httpx.NetworkError as e:
            raise NetworkError(str(e)) from e

    def _raise_for_status(self, status: int, message: str) -> None:
        if status == 400:
            raise ValidationError(message)
        elif status == 401:
            raise AuthenticationError(message)
        elif status == 404:
            raise NotFoundError(message)
        elif status == 429:
            raise RateLimitError(message)
        else:
            raise MiniMemoryError(message, status=status)

    # ============ Memory Operations ============

    async def remember(
        self,
        content: str,
        *,
        type: MemoryType = MemoryType.SEMANTIC,
        importance: float = 0.5,
        metadata: dict[str, Any] | None = None,
        embedding: list[float] | None = None,
        generate_embedding: bool = True,
        session_id: str | None = None,
        ttl: int | None = None,
    ) -> tuple[Memory, bool, bool]:
        """
        Store a new memory.

        Args:
            content: The text content to remember
            type: Memory type (episodic, semantic, working, knowledge)
            importance: Importance score (0-1)
            metadata: Additional metadata
            embedding: Pre-computed embedding vector
            generate_embedding: Auto-generate embedding
            session_id: Session ID (for working memory)
            ttl: Time-to-live in ms (for working memory)

        Returns:
            Tuple of (Memory, embedding_generated, persisted)
        """
        body: dict[str, Any] = {
            "content": content,
            "type": type.value,
            "importance": importance,
            "generateEmbedding": generate_embedding,
        }
        if metadata:
            body["metadata"] = metadata
        if embedding:
            body["embedding"] = embedding
        if session_id:
            body["sessionId"] = session_id
        if ttl:
            body["ttl"] = ttl

        data = await self._request("POST", "/remember", body)
        memory = Memory.model_validate(data["memory"])
        return memory, data.get("embeddingGenerated", False), data.get("persisted", False)

    async def recall(
        self,
        query: str | None = None,
        *,
        keywords: str | None = None,
        embedding: list[float] | None = None,
        type: MemoryType | None = None,
        limit: int = 10,
        min_importance: float | None = None,
        min_similarity: float | None = None,
        session_id: str | None = None,
        mode: SearchMode = SearchMode.HYBRID,
        alpha: float = 0.5,
    ) -> tuple[list[RecallResult], bool]:
        """
        Search for relevant memories.

        Args:
            query: Search query (auto-generates embedding)
            keywords: Keywords for keyword search
            embedding: Pre-computed embedding for vector search
            type: Filter by memory type
            limit: Maximum results
            min_importance: Minimum importance threshold
            min_similarity: Minimum similarity score
            session_id: Filter by session
            mode: Search mode (vector, keyword, hybrid)
            alpha: Hybrid weight (0=keyword, 1=vector)

        Returns:
            Tuple of (results, embedding_generated)
        """
        body: dict[str, Any] = {
            "limit": limit,
            "mode": mode.value,
            "alpha": alpha,
        }
        if query:
            body["query"] = query
        if keywords:
            body["keywords"] = keywords
        if embedding:
            body["embedding"] = embedding
        if type:
            body["type"] = type.value
        if min_importance is not None:
            body["minImportance"] = min_importance
        if min_similarity is not None:
            body["minSimilarity"] = min_similarity
        if session_id:
            body["sessionId"] = session_id

        data = await self._request("POST", "/recall", body)
        results = [RecallResult.model_validate(r) for r in data.get("results", [])]
        return results, data.get("embeddingGenerated", False)

    async def get(self, memory_id: str) -> Memory:
        """Get a specific memory by ID."""
        data = await self._request("GET", f"/memory/{memory_id}")
        return Memory.model_validate(data["memory"])

    async def update(
        self,
        memory_id: str,
        *,
        content: str | None = None,
        importance: float | None = None,
        metadata: dict[str, Any] | None = None,
        embedding: list[float] | None = None,
    ) -> Memory:
        """Update an existing memory."""
        body: dict[str, Any] = {}
        if content is not None:
            body["content"] = content
        if importance is not None:
            body["importance"] = importance
        if metadata is not None:
            body["metadata"] = metadata
        if embedding is not None:
            body["embedding"] = embedding

        data = await self._request("PATCH", f"/memory/{memory_id}", body)
        return Memory.model_validate(data["memory"])

    async def forget(self, memory_id: str) -> bool:
        """Delete a specific memory."""
        data = await self._request("DELETE", f"/forget/{memory_id}")
        return data.get("success", False)

    async def forget_by_filter(self, filter: dict[str, Any]) -> int:
        """Delete memories matching a filter."""
        data = await self._request("POST", "/forget", {"filter": filter})
        return data.get("count", 0)

    async def stats(self) -> MemoryStats:
        """Get memory statistics for the namespace."""
        data = await self._request("GET", "/stats")
        return MemoryStats.model_validate(data["stats"])

    async def cleanup(self) -> int:
        """Remove expired working memories."""
        data = await self._request("POST", "/cleanup")
        return data.get("count", 0)

    async def decay(self) -> None:
        """Apply importance decay to all memories."""
        await self._request("POST", "/decay")

    async def export_memories(self) -> list[dict[str, Any]]:
        """Export all memories from the namespace."""
        data = await self._request("POST", "/export")
        return data.get("data", {}).get("memories", [])

    async def import_memories(self, memories: list[dict[str, Any]]) -> int:
        """Import memories into the namespace."""
        data = await self._request("POST", "/import", {"memories": memories})
        return data.get("count", 0)

    async def clear(self) -> None:
        """Delete all memories in the namespace."""
        await self._request("DELETE", "/clear")
