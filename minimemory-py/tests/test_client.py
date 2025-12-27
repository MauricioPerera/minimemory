"""
Unit tests for MiniMemoryClient.
"""

import pytest
from pytest_httpx import HTTPXMock

from minimemory import (
    AuthenticationError,
    MiniMemoryClient,
    MemoryType,
    NotFoundError,
    RateLimitError,
    SearchMode,
    ValidationError,
)


class TestMiniMemoryClient:
    """Tests for MiniMemoryClient initialization and configuration."""

    def test_client_creation(self) -> None:
        """Should create client with required parameters."""
        client = MiniMemoryClient(
            base_url="https://api.example.com",
            api_key="test_key",
        )
        assert client.base_url == "https://api.example.com"
        assert client.api_key == "test_key"
        assert client.namespace == "default"

    def test_client_with_custom_namespace(self) -> None:
        """Should create client with custom namespace."""
        client = MiniMemoryClient(
            base_url="https://api.example.com",
            namespace="custom",
        )
        assert client.namespace == "custom"

    def test_client_strips_trailing_slash(self) -> None:
        """Should strip trailing slash from base URL."""
        client = MiniMemoryClient(base_url="https://api.example.com/")
        assert client.base_url == "https://api.example.com"

    def test_set_namespace(self) -> None:
        """Should update namespace with fluent API."""
        client = MiniMemoryClient(base_url="https://api.example.com")
        result = client.set_namespace("new_ns")
        assert client.namespace == "new_ns"
        assert result is client

    def test_set_api_key(self) -> None:
        """Should update API key with fluent API."""
        client = MiniMemoryClient(base_url="https://api.example.com")
        result = client.set_api_key("new_key")
        assert client.api_key == "new_key"
        assert result is client

    def test_set_access_token(self) -> None:
        """Should update access token with fluent API."""
        client = MiniMemoryClient(base_url="https://api.example.com")
        result = client.set_access_token("jwt_token")
        assert client.access_token == "jwt_token"
        assert result is client


class TestRemember:
    """Tests for remember() method."""

    @pytest.mark.asyncio
    async def test_remember_basic(self, httpx_mock: HTTPXMock) -> None:
        """Should store a memory successfully."""
        httpx_mock.add_response(
            method="POST",
            url="https://api.example.com/remember",
            json={
                "memory": {
                    "id": "mem_123",
                    "type": "semantic",
                    "content": "Test content",
                    "importance": 0.5,
                    "createdAt": 1700000000000,
                },
                "embeddingGenerated": True,
                "persisted": True,
            },
        )

        async with MiniMemoryClient(
            base_url="https://api.example.com",
            api_key="test_key",
        ) as client:
            memory, embedding_generated, persisted = await client.remember("Test content")

        assert memory.id == "mem_123"
        assert memory.content == "Test content"
        assert memory.type == MemoryType.SEMANTIC
        assert embedding_generated is True
        assert persisted is True

    @pytest.mark.asyncio
    async def test_remember_with_options(self, httpx_mock: HTTPXMock) -> None:
        """Should store a memory with all options."""
        httpx_mock.add_response(
            method="POST",
            url="https://api.example.com/remember",
            json={
                "memory": {
                    "id": "mem_456",
                    "type": "episodic",
                    "content": "User event",
                    "importance": 0.9,
                    "metadata": {"event": "login"},
                    "createdAt": 1700000000000,
                },
                "embeddingGenerated": True,
                "persisted": True,
            },
        )

        async with MiniMemoryClient(
            base_url="https://api.example.com",
            api_key="test_key",
        ) as client:
            memory, _, _ = await client.remember(
                "User event",
                type=MemoryType.EPISODIC,
                importance=0.9,
                metadata={"event": "login"},
            )

        assert memory.type == MemoryType.EPISODIC
        assert memory.importance == 0.9
        assert memory.metadata == {"event": "login"}


class TestRecall:
    """Tests for recall() method."""

    @pytest.mark.asyncio
    async def test_recall_basic(self, httpx_mock: HTTPXMock) -> None:
        """Should search memories successfully."""
        httpx_mock.add_response(
            method="POST",
            url="https://api.example.com/recall",
            json={
                "results": [
                    {
                        "id": "mem_123",
                        "type": "semantic",
                        "content": "Related content",
                        "score": 0.85,
                        "importance": 0.5,
                        "createdAt": 1700000000000,
                    }
                ],
                "embeddingGenerated": True,
            },
        )

        async with MiniMemoryClient(
            base_url="https://api.example.com",
            api_key="test_key",
        ) as client:
            results, embedding_generated = await client.recall("search query")

        assert len(results) == 1
        assert results[0].id == "mem_123"
        assert results[0].score == 0.85
        assert embedding_generated is True

    @pytest.mark.asyncio
    async def test_recall_with_options(self, httpx_mock: HTTPXMock) -> None:
        """Should search with all options."""
        httpx_mock.add_response(
            method="POST",
            url="https://api.example.com/recall",
            json={"results": [], "embeddingGenerated": False},
        )

        async with MiniMemoryClient(
            base_url="https://api.example.com",
            api_key="test_key",
        ) as client:
            results, _ = await client.recall(
                "query",
                type=MemoryType.SEMANTIC,
                limit=5,
                mode=SearchMode.KEYWORD,
                min_importance=0.3,
            )

        assert results == []


class TestMemoryOperations:
    """Tests for get, update, forget operations."""

    @pytest.mark.asyncio
    async def test_get_memory(self, httpx_mock: HTTPXMock) -> None:
        """Should get a memory by ID."""
        httpx_mock.add_response(
            method="GET",
            url="https://api.example.com/memory/mem_123",
            json={
                "memory": {
                    "id": "mem_123",
                    "type": "semantic",
                    "content": "Test content",
                    "importance": 0.5,
                    "createdAt": 1700000000000,
                }
            },
        )

        async with MiniMemoryClient(
            base_url="https://api.example.com",
            api_key="test_key",
        ) as client:
            memory = await client.get("mem_123")

        assert memory.id == "mem_123"
        assert memory.content == "Test content"

    @pytest.mark.asyncio
    async def test_update_memory(self, httpx_mock: HTTPXMock) -> None:
        """Should update a memory."""
        httpx_mock.add_response(
            method="PATCH",
            url="https://api.example.com/memory/mem_123",
            json={
                "memory": {
                    "id": "mem_123",
                    "type": "semantic",
                    "content": "Updated content",
                    "importance": 0.8,
                    "createdAt": 1700000000000,
                }
            },
        )

        async with MiniMemoryClient(
            base_url="https://api.example.com",
            api_key="test_key",
        ) as client:
            memory = await client.update("mem_123", content="Updated content", importance=0.8)

        assert memory.content == "Updated content"
        assert memory.importance == 0.8

    @pytest.mark.asyncio
    async def test_forget_memory(self, httpx_mock: HTTPXMock) -> None:
        """Should delete a memory."""
        httpx_mock.add_response(
            method="DELETE",
            url="https://api.example.com/forget/mem_123",
            json={"success": True},
        )

        async with MiniMemoryClient(
            base_url="https://api.example.com",
            api_key="test_key",
        ) as client:
            success = await client.forget("mem_123")

        assert success is True

    @pytest.mark.asyncio
    async def test_forget_by_filter(self, httpx_mock: HTTPXMock) -> None:
        """Should delete memories by filter."""
        httpx_mock.add_response(
            method="POST",
            url="https://api.example.com/forget",
            json={"count": 5},
        )

        async with MiniMemoryClient(
            base_url="https://api.example.com",
            api_key="test_key",
        ) as client:
            count = await client.forget_by_filter({"type": "working"})

        assert count == 5


class TestStats:
    """Tests for stats() method."""

    @pytest.mark.asyncio
    async def test_stats(self, httpx_mock: HTTPXMock) -> None:
        """Should get memory statistics."""
        httpx_mock.add_response(
            method="GET",
            url="https://api.example.com/stats",
            json={
                "stats": {
                    "total": 100,
                    "byType": {"semantic": 50, "episodic": 50},
                    "averageImportance": 0.6,
                    "oldestMemory": 1699000000000,
                    "newestMemory": 1700000000000,
                }
            },
        )

        async with MiniMemoryClient(
            base_url="https://api.example.com",
            api_key="test_key",
        ) as client:
            stats = await client.stats()

        assert stats.total == 100
        assert stats.by_type == {"semantic": 50, "episodic": 50}
        assert stats.average_importance == 0.6


class TestExportImport:
    """Tests for export and import operations."""

    @pytest.mark.asyncio
    async def test_export_memories(self, httpx_mock: HTTPXMock) -> None:
        """Should export all memories."""
        httpx_mock.add_response(
            method="POST",
            url="https://api.example.com/export",
            json={
                "data": {
                    "memories": [
                        {"id": "mem_1", "content": "Memory 1"},
                        {"id": "mem_2", "content": "Memory 2"},
                    ]
                }
            },
        )

        async with MiniMemoryClient(
            base_url="https://api.example.com",
            api_key="test_key",
        ) as client:
            memories = await client.export_memories()

        assert len(memories) == 2

    @pytest.mark.asyncio
    async def test_import_memories(self, httpx_mock: HTTPXMock) -> None:
        """Should import memories."""
        httpx_mock.add_response(
            method="POST",
            url="https://api.example.com/import",
            json={"count": 2},
        )

        async with MiniMemoryClient(
            base_url="https://api.example.com",
            api_key="test_key",
        ) as client:
            count = await client.import_memories([
                {"id": "mem_1", "content": "Memory 1"},
                {"id": "mem_2", "content": "Memory 2"},
            ])

        assert count == 2


class TestCleanupAndDecay:
    """Tests for cleanup and decay operations."""

    @pytest.mark.asyncio
    async def test_cleanup(self, httpx_mock: HTTPXMock) -> None:
        """Should cleanup expired memories."""
        httpx_mock.add_response(
            method="POST",
            url="https://api.example.com/cleanup",
            json={"count": 3},
        )

        async with MiniMemoryClient(
            base_url="https://api.example.com",
            api_key="test_key",
        ) as client:
            count = await client.cleanup()

        assert count == 3

    @pytest.mark.asyncio
    async def test_decay(self, httpx_mock: HTTPXMock) -> None:
        """Should apply importance decay."""
        httpx_mock.add_response(
            method="POST",
            url="https://api.example.com/decay",
            json={"success": True},
        )

        async with MiniMemoryClient(
            base_url="https://api.example.com",
            api_key="test_key",
        ) as client:
            await client.decay()

    @pytest.mark.asyncio
    async def test_clear(self, httpx_mock: HTTPXMock) -> None:
        """Should clear all memories."""
        httpx_mock.add_response(
            method="DELETE",
            url="https://api.example.com/clear",
            json={"success": True},
        )

        async with MiniMemoryClient(
            base_url="https://api.example.com",
            api_key="test_key",
        ) as client:
            await client.clear()


class TestErrorHandling:
    """Tests for error handling."""

    @pytest.mark.asyncio
    async def test_validation_error(self, httpx_mock: HTTPXMock) -> None:
        """Should raise ValidationError on 400."""
        httpx_mock.add_response(
            method="POST",
            url="https://api.example.com/remember",
            status_code=400,
            json={"error": "Content is required"},
        )

        async with MiniMemoryClient(
            base_url="https://api.example.com",
            api_key="test_key",
        ) as client:
            with pytest.raises(ValidationError) as exc_info:
                await client.remember("")

        assert "Content is required" in str(exc_info.value)

    @pytest.mark.asyncio
    async def test_authentication_error(self, httpx_mock: HTTPXMock) -> None:
        """Should raise AuthenticationError on 401."""
        httpx_mock.add_response(
            method="GET",
            url="https://api.example.com/stats",
            status_code=401,
            json={"error": "Invalid API key"},
        )

        async with MiniMemoryClient(
            base_url="https://api.example.com",
            api_key="invalid_key",
        ) as client:
            with pytest.raises(AuthenticationError) as exc_info:
                await client.stats()

        assert "Invalid API key" in str(exc_info.value)

    @pytest.mark.asyncio
    async def test_not_found_error(self, httpx_mock: HTTPXMock) -> None:
        """Should raise NotFoundError on 404."""
        httpx_mock.add_response(
            method="GET",
            url="https://api.example.com/memory/nonexistent",
            status_code=404,
            json={"error": "Memory not found"},
        )

        async with MiniMemoryClient(
            base_url="https://api.example.com",
            api_key="test_key",
        ) as client:
            with pytest.raises(NotFoundError) as exc_info:
                await client.get("nonexistent")

        assert "Memory not found" in str(exc_info.value)

    @pytest.mark.asyncio
    async def test_rate_limit_error(self, httpx_mock: HTTPXMock) -> None:
        """Should raise RateLimitError on 429."""
        httpx_mock.add_response(
            method="POST",
            url="https://api.example.com/remember",
            status_code=429,
            json={"error": "Rate limit exceeded"},
        )

        async with MiniMemoryClient(
            base_url="https://api.example.com",
            api_key="test_key",
        ) as client:
            with pytest.raises(RateLimitError):
                await client.remember("Test")


class TestHeaders:
    """Tests for request headers."""

    @pytest.mark.asyncio
    async def test_api_key_header(self, httpx_mock: HTTPXMock) -> None:
        """Should include X-API-Key header."""
        httpx_mock.add_response(
            method="GET",
            url="https://api.example.com/stats",
            json={"stats": {"total": 0, "byType": {}, "averageImportance": 0}},
        )

        async with MiniMemoryClient(
            base_url="https://api.example.com",
            api_key="my_api_key",
        ) as client:
            await client.stats()

        request = httpx_mock.get_requests()[0]
        assert request.headers["X-API-Key"] == "my_api_key"

    @pytest.mark.asyncio
    async def test_namespace_header(self, httpx_mock: HTTPXMock) -> None:
        """Should include X-Namespace header."""
        httpx_mock.add_response(
            method="GET",
            url="https://api.example.com/stats",
            json={"stats": {"total": 0, "byType": {}, "averageImportance": 0}},
        )

        async with MiniMemoryClient(
            base_url="https://api.example.com",
            namespace="custom_ns",
        ) as client:
            await client.stats()

        request = httpx_mock.get_requests()[0]
        assert request.headers["X-Namespace"] == "custom_ns"

    @pytest.mark.asyncio
    async def test_bearer_token_header(self, httpx_mock: HTTPXMock) -> None:
        """Should include Authorization header with access token."""
        httpx_mock.add_response(
            method="GET",
            url="https://api.example.com/stats",
            json={"stats": {"total": 0, "byType": {}, "averageImportance": 0}},
        )

        async with MiniMemoryClient(
            base_url="https://api.example.com",
            access_token="jwt_token_here",
        ) as client:
            await client.stats()

        request = httpx_mock.get_requests()[0]
        assert request.headers["Authorization"] == "Bearer jwt_token_here"

    @pytest.mark.asyncio
    async def test_custom_headers(self, httpx_mock: HTTPXMock) -> None:
        """Should include custom headers."""
        httpx_mock.add_response(
            method="GET",
            url="https://api.example.com/stats",
            json={"stats": {"total": 0, "byType": {}, "averageImportance": 0}},
        )

        async with MiniMemoryClient(
            base_url="https://api.example.com",
            headers={"X-Custom-Header": "custom_value"},
        ) as client:
            await client.stats()

        request = httpx_mock.get_requests()[0]
        assert request.headers["X-Custom-Header"] == "custom_value"
