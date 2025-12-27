"""
Unit tests for EmbedAPI.
"""

import pytest
from pytest_httpx import HTTPXMock

from minimemory import MiniMemoryClient


class TestEmbedSingle:
    """Tests for embed.single() method."""

    @pytest.mark.asyncio
    async def test_embed_single(self, httpx_mock: HTTPXMock) -> None:
        """Should generate embedding for single text."""
        httpx_mock.add_response(
            method="POST",
            url="https://api.example.com/embed",
            json={
                "embedding": [0.1, 0.2, 0.3, 0.4, 0.5],
                "dimensions": 768,
                "model": "bge-base-en-v1.5",
                "truncated": False,
            },
        )

        async with MiniMemoryClient(
            base_url="https://api.example.com",
            api_key="test_key",
        ) as client:
            result = await client.embed.single("Hello world")

        assert len(result.embedding) == 5
        assert result.dimensions == 768
        assert result.model == "bge-base-en-v1.5"
        assert result.truncated is False

    @pytest.mark.asyncio
    async def test_embed_single_with_dimensions(self, httpx_mock: HTTPXMock) -> None:
        """Should generate embedding with custom dimensions."""
        httpx_mock.add_response(
            method="POST",
            url="https://api.example.com/embed",
            json={
                "embedding": [0.1, 0.2, 0.3],
                "dimensions": 256,
                "model": "bge-base-en-v1.5",
            },
        )

        async with MiniMemoryClient(
            base_url="https://api.example.com",
            api_key="test_key",
        ) as client:
            result = await client.embed.single("Hello", dimensions=256)

        assert result.dimensions == 256


class TestEmbedBatch:
    """Tests for embed.batch() method."""

    @pytest.mark.asyncio
    async def test_embed_batch(self, httpx_mock: HTTPXMock) -> None:
        """Should generate embeddings for multiple texts."""
        httpx_mock.add_response(
            method="POST",
            url="https://api.example.com/embed",
            json={
                "embeddings": [
                    [0.1, 0.2, 0.3],
                    [0.4, 0.5, 0.6],
                    [0.7, 0.8, 0.9],
                ],
                "dimensions": 768,
                "model": "bge-base-en-v1.5",
                "count": 3,
            },
        )

        async with MiniMemoryClient(
            base_url="https://api.example.com",
            api_key="test_key",
        ) as client:
            result = await client.embed.batch(["Text 1", "Text 2", "Text 3"])

        assert len(result.embeddings) == 3
        assert result.count == 3
        assert result.dimensions == 768


class TestEmbedInfo:
    """Tests for embed.info() method."""

    @pytest.mark.asyncio
    async def test_embed_info(self, httpx_mock: HTTPXMock) -> None:
        """Should get embedding service info."""
        httpx_mock.add_response(
            method="GET",
            url="https://api.example.com/embed/info",
            json={
                "available": True,
                "model": "bge-base-en-v1.5",
                "dimensions": {"default": 768, "available": [768, 512, 256, 128]},
                "matryoshka": True,
                "pricing": {"perToken": 0.00001},
                "estimatedCosts": {"1000 embeddings": "$0.01"},
            },
        )

        async with MiniMemoryClient(
            base_url="https://api.example.com",
            api_key="test_key",
        ) as client:
            info = await client.embed.info()

        assert info.available is True
        assert info.model == "bge-base-en-v1.5"
        assert info.matryoshka is True
        assert info.dimensions["default"] == 768
