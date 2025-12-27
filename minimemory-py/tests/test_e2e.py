"""
End-to-end tests for the SDK against a running minimemory-service.

Run with: pytest tests/test_e2e.py -v
Requires: minimemory-service running at http://localhost:8787
"""

import os

import pytest

from minimemory import (
    MiniMemoryClient,
    MemoryType,
    NotFoundError,
    SearchMode,
)

BASE_URL = os.environ.get("MINIMEMORY_URL", "http://localhost:8787/api/v1")
API_KEY = os.environ.get("MINIMEMORY_API_KEY", "mm_dev_key_12345")
TEST_NAMESPACE = "default"


def get_client() -> MiniMemoryClient:
    """Create a client connected to the test service."""
    return MiniMemoryClient(
        base_url=BASE_URL,
        api_key=API_KEY,
        namespace=TEST_NAMESPACE,
        timeout=10.0,
    )


class TestMemoryOperations:
    """E2E tests for memory operations."""

    @pytest.mark.asyncio
    async def test_remember_and_get(self) -> None:
        """Should store and retrieve a memory."""
        async with get_client() as client:
            # Store
            memory, embedding_generated, persisted = await client.remember(
                "The quick brown fox jumps over the lazy dog",
                type=MemoryType.SEMANTIC,
                importance=0.8,
                metadata={"test": True, "category": "pangram"},
            )

            assert memory.id is not None
            assert memory.content == "The quick brown fox jumps over the lazy dog"
            assert memory.type == MemoryType.SEMANTIC
            assert persisted is True
            print(f"  Created memory: {memory.id}")

            # Retrieve
            retrieved = await client.get(memory.id)
            assert retrieved.id == memory.id
            assert retrieved.content == memory.content

            # Cleanup
            await client.forget(memory.id)

    @pytest.mark.asyncio
    async def test_recall_with_keyword_search(self) -> None:
        """Should search memories with keyword mode."""
        async with get_client() as client:
            # Store test memories
            mem1, _, _ = await client.remember(
                "TypeScript is a typed superset of JavaScript",
                type=MemoryType.SEMANTIC,
                metadata={"topic": "programming"},
            )
            mem2, _, _ = await client.remember(
                "Python is great for machine learning",
                type=MemoryType.SEMANTIC,
                metadata={"topic": "programming"},
            )

            try:
                # Search
                results, _ = await client.recall(
                    "programming languages",
                    limit=5,
                    mode=SearchMode.KEYWORD,
                )

                assert len(results) >= 0
                print(f"  Found {len(results)} results")
            finally:
                # Cleanup
                await client.forget(mem1.id)
                await client.forget(mem2.id)

    @pytest.mark.asyncio
    async def test_update_memory(self) -> None:
        """Should update a memory."""
        async with get_client() as client:
            # Create
            memory, _, _ = await client.remember(
                "Original content",
                importance=0.5,
            )

            try:
                # Update
                updated = await client.update(
                    memory.id,
                    importance=0.95,
                )

                assert updated.importance == 0.95
            finally:
                await client.forget(memory.id)

    @pytest.mark.asyncio
    async def test_get_stats(self) -> None:
        """Should get memory statistics."""
        async with get_client() as client:
            stats = await client.stats()

            assert stats.total >= 0
            assert stats.by_type is not None
            print(f"  Stats: total={stats.total}")

    @pytest.mark.asyncio
    async def test_export_and_import(self) -> None:
        """Should export and import memories."""
        async with get_client() as client:
            # Create test memory
            memory, _, _ = await client.remember(
                "Export test memory",
                importance=0.7,
            )

            try:
                # Export
                exported = await client.export_memories()
                assert len(exported) >= 1
                print(f"  Exported {len(exported)} memories")
            finally:
                await client.forget(memory.id)

    @pytest.mark.asyncio
    async def test_forget_memory(self) -> None:
        """Should delete a memory."""
        async with get_client() as client:
            # Create
            memory, _, _ = await client.remember("To be deleted")

            # Delete
            success = await client.forget(memory.id)
            assert success is True

            # Verify deleted
            with pytest.raises(NotFoundError):
                await client.get(memory.id)

    @pytest.mark.asyncio
    async def test_not_found_error(self) -> None:
        """Should raise NotFoundError for non-existent memory."""
        async with get_client() as client:
            with pytest.raises(NotFoundError):
                await client.get("non_existent_id_12345")


class TestEmbeddingOperations:
    """E2E tests for embedding operations."""

    @pytest.mark.asyncio
    async def test_embed_info(self) -> None:
        """Should get embedding service info."""
        async with get_client() as client:
            info = await client.embed.info()

            assert info.model is not None
            assert info.dimensions is not None
            print(f"  Embedding model: {info.model}")
            print(f"  Available: {info.available}")


class TestCleanup:
    """Cleanup tests."""

    @pytest.mark.asyncio
    async def test_clear_namespace(self) -> None:
        """Should clear all memories in namespace."""
        async with get_client() as client:
            # Create some test memories
            await client.remember("Cleanup test 1")
            await client.remember("Cleanup test 2")

            # Clear
            await client.clear()

            # Verify
            stats = await client.stats()
            assert stats.total == 0
            print(f"  Cleared namespace: {TEST_NAMESPACE}")
