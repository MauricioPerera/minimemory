"""
Pytest configuration and fixtures.
"""

import pytest

from minimemory import MiniMemoryClient


@pytest.fixture
def service_url() -> str:
    """Base URL for the minimemory service."""
    return "http://localhost:8787/api/v1"


@pytest.fixture
def test_api_key() -> str:
    """Dev API key for testing."""
    return "mm_dev_key_12345"


@pytest.fixture
def test_namespace() -> str:
    """Test namespace."""
    return "default"


@pytest.fixture
async def client(service_url: str, test_api_key: str, test_namespace: str) -> MiniMemoryClient:
    """Create a MiniMemoryClient instance."""
    async with MiniMemoryClient(
        base_url=service_url,
        api_key=test_api_key,
        namespace=test_namespace,
        timeout=10.0,
    ) as client:
        yield client
