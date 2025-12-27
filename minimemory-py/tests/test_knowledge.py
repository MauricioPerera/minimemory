"""
Unit tests for KnowledgeAPI.
"""

import pytest
from pytest_httpx import HTTPXMock

from minimemory import (
    ChunkingOptions,
    KnowledgeSourceType,
    MiniMemoryClient,
)


class TestKnowledgeIngest:
    """Tests for knowledge.ingest() method."""

    @pytest.mark.asyncio
    async def test_ingest_document(self, httpx_mock: HTTPXMock) -> None:
        """Should ingest a document successfully."""
        httpx_mock.add_response(
            method="POST",
            url="https://api.example.com/knowledge/ingest",
            json={
                "sourceId": "src_123",
                "sourceName": "test.txt",
                "chunksCreated": 5,
                "embeddingsGenerated": True,
                "totalCharacters": 5000,
                "averageChunkSize": 1000,
                "durationMs": 150,
            },
        )

        async with MiniMemoryClient(
            base_url="https://api.example.com",
            api_key="test_key",
        ) as client:
            result = await client.knowledge.ingest(
                content="Long document content...",
                name="test.txt",
            )

        assert result.source_id == "src_123"
        assert result.chunks_created == 5
        assert result.embeddings_generated is True

    @pytest.mark.asyncio
    async def test_ingest_with_options(self, httpx_mock: HTTPXMock) -> None:
        """Should ingest with all options."""
        httpx_mock.add_response(
            method="POST",
            url="https://api.example.com/knowledge/ingest",
            json={
                "sourceId": "src_456",
                "sourceName": "article.md",
                "chunksCreated": 10,
                "embeddingsGenerated": True,
                "totalCharacters": 10000,
                "averageChunkSize": 1000,
                "durationMs": 200,
            },
        )

        async with MiniMemoryClient(
            base_url="https://api.example.com",
            api_key="test_key",
        ) as client:
            result = await client.knowledge.ingest(
                content="# Article\n\nContent here...",
                name="article.md",
                type=KnowledgeSourceType.DOCUMENT,
                url="https://example.com/article",
                mime_type="text/markdown",
                metadata={"author": "Test"},
                chunking=ChunkingOptions(chunk_size=500, chunk_overlap=50),
            )

        assert result.source_id == "src_456"
        assert result.chunks_created == 10


class TestKnowledgeSources:
    """Tests for knowledge source operations."""

    @pytest.mark.asyncio
    async def test_list_sources(self, httpx_mock: HTTPXMock) -> None:
        """Should list knowledge sources."""
        httpx_mock.add_response(
            method="GET",
            url="https://api.example.com/knowledge/sources?limit=100&offset=0",
            json={
                "sources": [
                    {
                        "id": "src_1",
                        "name": "doc1.txt",
                        "type": "document",
                        "chunkCount": 5,
                        "namespace": "default",
                        "metadata": {},
                        "createdAt": 1700000000000,
                        "updatedAt": 1700000000000,
                    }
                ],
                "total": 1,
                "hasMore": False,
            },
        )

        async with MiniMemoryClient(
            base_url="https://api.example.com",
            api_key="test_key",
        ) as client:
            sources, total, has_more = await client.knowledge.list_sources()

        assert len(sources) == 1
        assert sources[0].id == "src_1"
        assert sources[0].name == "doc1.txt"
        assert total == 1
        assert has_more is False

    @pytest.mark.asyncio
    async def test_list_sources_with_filter(self, httpx_mock: HTTPXMock) -> None:
        """Should list sources with type filter."""
        httpx_mock.add_response(
            method="GET",
            url="https://api.example.com/knowledge/sources?limit=50&offset=10&type=url",
            json={
                "sources": [],
                "total": 0,
                "hasMore": False,
            },
        )

        async with MiniMemoryClient(
            base_url="https://api.example.com",
            api_key="test_key",
        ) as client:
            sources, total, _ = await client.knowledge.list_sources(
                type=KnowledgeSourceType.URL,
                limit=50,
                offset=10,
            )

        assert sources == []
        assert total == 0

    @pytest.mark.asyncio
    async def test_get_source(self, httpx_mock: HTTPXMock) -> None:
        """Should get a specific source."""
        httpx_mock.add_response(
            method="GET",
            url="https://api.example.com/knowledge/sources/src_123",
            json={
                "source": {
                    "id": "src_123",
                    "name": "document.pdf",
                    "type": "document",
                    "mimeType": "application/pdf",
                    "chunkCount": 20,
                    "namespace": "default",
                    "metadata": {"pages": 10},
                    "createdAt": 1700000000000,
                    "updatedAt": 1700000000000,
                }
            },
        )

        async with MiniMemoryClient(
            base_url="https://api.example.com",
            api_key="test_key",
        ) as client:
            source = await client.knowledge.get_source("src_123")

        assert source.id == "src_123"
        assert source.name == "document.pdf"
        assert source.mime_type == "application/pdf"
        assert source.chunk_count == 20

    @pytest.mark.asyncio
    async def test_delete_source(self, httpx_mock: HTTPXMock) -> None:
        """Should delete a source."""
        httpx_mock.add_response(
            method="DELETE",
            url="https://api.example.com/knowledge/sources/src_123",
            json={"success": True},
        )

        async with MiniMemoryClient(
            base_url="https://api.example.com",
            api_key="test_key",
        ) as client:
            success = await client.knowledge.delete_source("src_123")

        assert success is True


class TestKnowledgeChunks:
    """Tests for knowledge chunk operations."""

    @pytest.mark.asyncio
    async def test_get_chunks(self, httpx_mock: HTTPXMock) -> None:
        """Should get chunks for a source."""
        httpx_mock.add_response(
            method="GET",
            url="https://api.example.com/knowledge/sources/src_123/chunks?limit=100&offset=0",
            json={
                "chunks": [
                    {
                        "id": "chunk_1",
                        "content": "First chunk content",
                        "chunkIndex": 0,
                        "startOffset": 0,
                        "endOffset": 500,
                        "createdAt": 1700000000000,
                    },
                    {
                        "id": "chunk_2",
                        "content": "Second chunk content",
                        "chunkIndex": 1,
                        "startOffset": 450,
                        "endOffset": 950,
                        "createdAt": 1700000000000,
                    },
                ],
                "total": 2,
                "hasMore": False,
            },
        )

        async with MiniMemoryClient(
            base_url="https://api.example.com",
            api_key="test_key",
        ) as client:
            chunks, total, has_more = await client.knowledge.get_chunks("src_123")

        assert len(chunks) == 2
        assert chunks[0].chunk_index == 0
        assert chunks[1].chunk_index == 1
        assert total == 2
        assert has_more is False


class TestKnowledgeStats:
    """Tests for knowledge.stats() method."""

    @pytest.mark.asyncio
    async def test_stats(self, httpx_mock: HTTPXMock) -> None:
        """Should get knowledge statistics."""
        httpx_mock.add_response(
            method="GET",
            url="https://api.example.com/knowledge/stats",
            json={
                "stats": {
                    "totalSources": 10,
                    "totalChunks": 150,
                    "byType": {"document": 8, "url": 2},
                    "averageChunksPerSource": 15.0,
                }
            },
        )

        async with MiniMemoryClient(
            base_url="https://api.example.com",
            api_key="test_key",
        ) as client:
            stats = await client.knowledge.stats()

        assert stats.total_sources == 10
        assert stats.total_chunks == 150
        assert stats.by_type == {"document": 8, "url": 2}
        assert stats.average_chunks_per_source == 15.0


class TestPreviewChunking:
    """Tests for knowledge.preview_chunking() method."""

    @pytest.mark.asyncio
    async def test_preview_chunking(self, httpx_mock: HTTPXMock) -> None:
        """Should preview chunking."""
        httpx_mock.add_response(
            method="POST",
            url="https://api.example.com/knowledge/chunk-preview",
            json={
                "chunks": [
                    {"content": "First chunk", "startOffset": 0, "endOffset": 100},
                    {"content": "Second chunk", "startOffset": 80, "endOffset": 180},
                ],
                "totalChunks": 2,
                "averageSize": 100,
            },
        )

        async with MiniMemoryClient(
            base_url="https://api.example.com",
            api_key="test_key",
        ) as client:
            result = await client.knowledge.preview_chunking(
                content="Content to chunk...",
                chunking=ChunkingOptions(chunk_size=100, chunk_overlap=20),
            )

        assert result["totalChunks"] == 2
        assert len(result["chunks"]) == 2
