# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2024-12-25

### Added

- Initial release of @minimemory/sdk
- `MiniMemoryClient` class with full API support
- Memory operations:
  - `remember()` - Store memories with auto-embedding
  - `recall()` - Search with vector, keyword, or hybrid mode
  - `get()` - Retrieve memory by ID
  - `update()` - Update memory content/metadata
  - `forget()` - Delete single memory
  - `forgetByFilter()` - Bulk delete by filter
  - `stats()` - Get namespace statistics
  - `cleanup()` - Remove expired working memories
  - `decay()` - Apply importance decay
  - `export()` / `import()` - Backup and restore
  - `clear()` - Clear all memories
- Knowledge Bank (RAG) operations:
  - `knowledge.ingest()` - Document ingestion with chunking
  - `knowledge.listSources()` - List knowledge sources
  - `knowledge.getSource()` - Get source details
  - `knowledge.deleteSource()` - Delete source and chunks
  - `knowledge.getChunks()` - Get source chunks
  - `knowledge.stats()` - Knowledge statistics
  - `knowledge.previewChunking()` - Preview chunking
- Embedding operations:
  - `embed.single()` - Single text embedding
  - `embed.batch()` - Batch embeddings
  - `embed.info()` - Service information
- Authentication support:
  - API Key authentication (`X-API-Key` header)
  - JWT Bearer token authentication
- `MiniMemoryError` class for typed error handling
- Full TypeScript type definitions
- ESM and CommonJS builds
- Unit tests (33 tests)
- E2E integration tests (10 tests)

### Technical Details

- Built with TypeScript 5.3
- Bundled with tsup (ESM + CJS + DTS)
- Tested with Vitest
- Zero runtime dependencies (uses native fetch)
- Supports Node.js >= 18.0.0
