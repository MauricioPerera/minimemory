# minimemory Python SDK

Python SDK for the minimemory vector database service.

## Installation

```bash
pip install minimemory
```

For development:

```bash
pip install minimemory[dev]
```

## Quick Start

```python
import asyncio
from minimemory import MiniMemoryClient, MemoryType, SearchMode

async def main():
    async with MiniMemoryClient(
        base_url="https://your-worker.workers.dev/api/v1",
        api_key="mm_your_api_key",
        namespace="default",
    ) as client:
        # Store a memory
        memory, embedding_generated, persisted = await client.remember(
            "User prefers dark mode",
            type=MemoryType.SEMANTIC,
            importance=0.8,
            metadata={"category": "preferences"},
        )
        print(f"Created memory: {memory.id}")

        # Search memories
        results, _ = await client.recall(
            "user preferences",
            limit=10,
            mode=SearchMode.HYBRID,
        )
        for result in results:
            print(f"  - {result.content} (score: {result.score:.2f})")

        # Get statistics
        stats = await client.stats()
        print(f"Total memories: {stats.total}")

asyncio.run(main())
```

## Features

- **Async-first**: Built with `httpx` for async HTTP requests
- **Type-safe**: Full type hints with Pydantic models
- **Memory operations**: remember, recall, get, update, forget
- **Knowledge Bank**: Document ingestion with RAG support
- **Embeddings**: Generate embeddings via the service
- **Multi-tenant**: Namespace isolation for different contexts

## API Reference

### MiniMemoryClient

```python
from minimemory import MiniMemoryClient

client = MiniMemoryClient(
    base_url="https://api.example.com/api/v1",
    api_key="mm_your_api_key",       # API key authentication
    access_token="jwt_token",         # Or JWT authentication
    namespace="default",              # Namespace for isolation
    timeout=30.0,                     # Request timeout in seconds
    headers={"X-Custom": "value"},    # Additional headers
)
```

### Memory Operations

#### remember()

Store a new memory:

```python
memory, embedding_generated, persisted = await client.remember(
    content="User likes dark mode",
    type=MemoryType.SEMANTIC,        # episodic, semantic, working, knowledge
    importance=0.8,                   # 0.0 to 1.0
    metadata={"key": "value"},        # Optional metadata
    embedding=[0.1, 0.2, ...],        # Pre-computed embedding (optional)
    generate_embedding=True,          # Auto-generate embedding
    session_id="session_123",         # For working memory
    ttl=3600000,                      # TTL in ms for working memory
)
```

#### recall()

Search for memories:

```python
results, embedding_generated = await client.recall(
    query="user preferences",         # Natural language query
    keywords="dark mode",             # For keyword search
    embedding=[0.1, 0.2, ...],        # Pre-computed embedding
    type=MemoryType.SEMANTIC,         # Filter by type
    limit=10,                         # Max results
    min_importance=0.3,               # Minimum importance
    min_similarity=0.5,               # Minimum similarity score
    session_id="session_123",         # Filter by session
    mode=SearchMode.HYBRID,           # vector, keyword, or hybrid
    alpha=0.5,                        # Hybrid weight (0=keyword, 1=vector)
)

for result in results:
    print(f"{result.content} - score: {result.score}")
```

#### get()

Get a specific memory:

```python
memory = await client.get("mem_123")
print(memory.content)
```

#### update()

Update a memory:

```python
memory = await client.update(
    "mem_123",
    content="Updated content",
    importance=0.9,
    metadata={"updated": True},
)
```

#### forget()

Delete a memory:

```python
success = await client.forget("mem_123")
```

#### forget_by_filter()

Delete memories matching a filter:

```python
count = await client.forget_by_filter({"type": "working"})
```

### Knowledge Bank

```python
# Ingest a document
result = await client.knowledge.ingest(
    content="Long document content...",
    name="document.pdf",
    type=KnowledgeSourceType.DOCUMENT,
    url="https://example.com/doc",
    metadata={"author": "Alice"},
    chunking=ChunkingOptions(chunk_size=500, chunk_overlap=50),
)
print(f"Created {result.chunks_created} chunks")

# List sources
sources, total, has_more = await client.knowledge.list_sources(
    type=KnowledgeSourceType.DOCUMENT,
    limit=100,
)

# Get source
source = await client.knowledge.get_source("src_123")

# Get chunks
chunks, total, has_more = await client.knowledge.get_chunks("src_123")

# Delete source
await client.knowledge.delete_source("src_123")

# Get stats
stats = await client.knowledge.stats()
```

### Embeddings

```python
# Single embedding
result = await client.embed.single("Hello world", dimensions=768)
print(f"Embedding: {result.embedding[:5]}...")

# Batch embeddings
result = await client.embed.batch(["Text 1", "Text 2", "Text 3"])
print(f"Generated {result.count} embeddings")

# Get embedding info
info = await client.embed.info()
print(f"Model: {info.model}, Available: {info.available}")
```

### Utility Operations

```python
# Get statistics
stats = await client.stats()
print(f"Total: {stats.total}, By type: {stats.by_type}")

# Cleanup expired working memories
count = await client.cleanup()

# Apply importance decay
await client.decay()

# Export all memories
memories = await client.export_memories()

# Import memories
count = await client.import_memories(memories)

# Clear namespace
await client.clear()
```

## Error Handling

```python
from minimemory import (
    MiniMemoryError,
    AuthenticationError,
    NotFoundError,
    RateLimitError,
    ValidationError,
    TimeoutError,
    NetworkError,
)

try:
    memory = await client.get("mem_123")
except NotFoundError:
    print("Memory not found")
except AuthenticationError:
    print("Invalid API key")
except RateLimitError as e:
    print(f"Rate limited, retry after: {e.retry_after}")
except ValidationError as e:
    print(f"Invalid request: {e.message}")
except TimeoutError:
    print("Request timed out")
except NetworkError:
    print("Network error")
except MiniMemoryError as e:
    print(f"Error: {e.message} (status={e.status})")
```

## Types

### Enums

- `MemoryType`: `EPISODIC`, `SEMANTIC`, `WORKING`, `KNOWLEDGE`
- `KnowledgeSourceType`: `DOCUMENT`, `URL`, `API`, `MANUAL`
- `SearchMode`: `VECTOR`, `KEYWORD`, `HYBRID`

### Models

- `Memory`: Memory object with id, type, content, importance, metadata
- `RecallResult`: Search result with score and source citation
- `MemoryStats`: Namespace statistics
- `KnowledgeSource`: Knowledge source metadata
- `KnowledgeChunk`: Document chunk
- `KnowledgeStats`: Knowledge bank statistics
- `ChunkingOptions`: Chunking configuration
- `IngestResult`: Document ingestion result
- `EmbedResult`: Single embedding result
- `EmbedBatchResult`: Batch embedding result
- `EmbedInfo`: Embedding service information

## Testing

Run unit tests:

```bash
pytest tests/test_client.py tests/test_knowledge.py tests/test_embed.py -v
```

Run E2E tests (requires running minimemory-service):

```bash
# Start the service first
cd ../minimemory-service && npm run dev

# Run E2E tests
pytest tests/test_e2e.py -v
```

## Development

```bash
# Install dev dependencies
pip install -e ".[dev]"

# Run linter
ruff check .

# Run type checker
mypy minimemory
```

## License

MIT
