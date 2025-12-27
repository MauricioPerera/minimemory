# Changelog

All notable changes to the minimemory Python SDK will be documented in this file.

## [0.1.0] - 2024-12-24

### Added

- Initial release of the minimemory Python SDK
- `MiniMemoryClient` async client with context manager support
- Memory operations:
  - `remember()` - Store memories with type, importance, and metadata
  - `recall()` - Search with vector, keyword, or hybrid modes
  - `get()` - Retrieve memory by ID
  - `update()` - Update memory content, importance, or metadata
  - `forget()` - Delete memory by ID
  - `forget_by_filter()` - Bulk delete by filter
  - `stats()` - Get namespace statistics
  - `cleanup()` - Remove expired working memories
  - `decay()` - Apply importance decay
  - `export_memories()` - Export all memories
  - `import_memories()` - Import memories
  - `clear()` - Delete all memories in namespace
- Knowledge Bank API (`client.knowledge`):
  - `ingest()` - Ingest documents with chunking
  - `list_sources()` - List knowledge sources
  - `get_source()` - Get source by ID
  - `delete_source()` - Delete source and chunks
  - `get_chunks()` - Get chunks for a source
  - `stats()` - Get knowledge bank statistics
  - `preview_chunking()` - Preview chunking configuration
- Embedding API (`client.embed`):
  - `single()` - Generate single embedding
  - `batch()` - Generate batch embeddings
  - `info()` - Get embedding service info
- Pydantic models for type safety:
  - `Memory`, `RecallResult`, `MemoryStats`
  - `KnowledgeSource`, `KnowledgeChunk`, `KnowledgeStats`
  - `ChunkingOptions`, `IngestResult`
  - `EmbedResult`, `EmbedBatchResult`, `EmbedInfo`
- Enums: `MemoryType`, `KnowledgeSourceType`, `SearchMode`
- Exception classes:
  - `MiniMemoryError` (base)
  - `AuthenticationError` (401)
  - `NotFoundError` (404)
  - `RateLimitError` (429)
  - `ValidationError` (400)
  - `TimeoutError`, `NetworkError`
- Authentication support:
  - API key (`X-API-Key` header)
  - JWT Bearer token (`Authorization` header)
- Namespace isolation (`X-Namespace` header)
- 50 tests (41 unit + 9 E2E)

### Dependencies

- `httpx>=0.25.0` - Async HTTP client
- `pydantic>=2.0.0` - Data validation
- Python 3.9+
