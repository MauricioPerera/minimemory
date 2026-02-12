# Changelog

All notable changes to minimemory will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.5.0] - 2025

### Added
- `GenericMemory::with_db()` constructor to wrap an existing `VectorDB` instance
- `GenericMemory::learn_raw()` and `learn_raw_with_priority()` for pre-built metadata ingestion
- `GenericMemory::db()` accessor for direct `VectorDB` access
- `AgentMemory::generic_memory()` accessor for the underlying `GenericMemory<SoftwareDevelopment>`
- HNSW index serialization/deserialization to `.mmdb` files (tagged "HNSW" block format)
- BM25 index persistence to `.mmdb` files (tagged "BM25" block format)
- CRC32 checksum verification on `.mmdb` file load (via `crc32fast`)
- Atomic writes for `.mmdb` persistence (write to `.tmp` then rename)
- `CRC32 mismatch` error variant in `Error` enum
- HNSW `level_for_node` tracking for correct entry point recovery after deletion
- HNSW `free_indices` pool for reusing deleted node slots without index fragmentation
- 83 integration tests (up from 69) covering memory systems, persistence round-trips, and concurrency
- 30 doc-tests across public API

### Changed
- **AgentMemory now wraps `GenericMemory<SoftwareDevelopment>`** instead of raw `VectorDB`, inheriting priority scoring, temporal decay, usage stats, transfer level inference, and concept extraction automatically
- `AgentMemory::learn_*` methods delegate to `GenericMemory::learn_raw()`, which auto-enriches metadata with `timestamp`, `priority`, `transfer_level`, `concepts`, `domain`, `usage_stats`
- `current_timestamp()` in `agent_memory` now returns seconds (was milliseconds), consistent with `memory_traits`
- `AgentMemory::embed()` returns `Error::InvalidConfig` when no `embed_fn` is set (was silently returning zero vectors)
- `GenericMemory::update_priority()` uses `db.update()` instead of `db.insert()` (was failing with `AlreadyExists`)
- `UsageStats` (access_count, useful_count) now persist in vector metadata, surviving save/load cycles
- `WorkingContext` serialized as a special document (`__working_context__`) in `.mmdb` files
- `TransferLevel` enum unified: `transfer.rs` now imports from `memory_traits` (eliminated duplicate enum)
- `transfer::ConceptExtractor` now implements `memory_traits::ConceptExtractor` trait
- `TransferableMemory` integrated with `GenericMemory` (removed duplicated transfer scoring logic)
- `AgentMemory::stats()` optimized to single-pass iteration (was 5 separate `filter_search` calls)
- Removed unused `config: MemoryConfig` field from `AgentMemory` struct
- `.mmdb` format version bumped to 2 with backward-compatible reading of v1 files
- HNSW `search_layer` skips deleted (freed) indices

### Fixed
- `update_priority()` no longer returns `AlreadyExists` error
- `cleanup_old()` timestamp arithmetic now consistent (seconds throughout)
- Zero-vector silent fallback replaced with explicit error propagation
- HNSW entry point correctly recovered after deleting the current entry point node
- HNSW index fragmentation after delete+insert cycles no longer causes search degradation
- BM25 `add()` correctly decrements old document frequencies before incrementing new ones on update
- Replication sequence/entries consistency: write lock acquired before atomic sequence increment
- `stream_position()` errors in disk storage now propagated (was `unwrap_or(0)`)
- CRC32 corruption detection on `.mmdb` load

## [2.4.1] - 2025

### Fixed
- SIMD optimizations and miscellaneous bug fixes
- Removed hardcoded secrets and sensitive infrastructure IDs
- MCP request handling on root endpoint
- Added support for more MCP methods and logging

## [2.4.0] - 2024

### Added
- Hybrid priority system with usage tracking and decay
- HNSW index with complete graph construction during insertion
- AVX2 SIMD acceleration for distance calculations (auto-detected at runtime)
- 69 integration tests covering all major modules
- Comprehensive benchmarks for performance testing
- Ollama integration example

### Changed
- Index trait now passes storage and distance to `add()` method
- HNSW `select_neighbors()` and `connect_neighbors()` are now actively used

### Fixed
- HNSW graph now properly builds bidirectional neighbor connections
- Persistence tests no longer have race conditions

## [2.3.0] - 2024

### Added
- Domain-agnostic memory system with traits (`GenericMemory<P: DomainPreset>`)
- Built-in presets: `SoftwareDevelopment`, `Conversational`, `CustomerService`
- Priority levels: `Critical`, `High`, `Normal`, `Low`
- Memory decay over time based on usage patterns
- `learn()` and `recall()` methods with automatic priority inference
- Context-aware recall with transfer level filtering

## [2.2.0] - 2024

### Added
- Agent memory system (`AgentMemory`) for AI agent workflows
- Knowledge transfer between agent instances
- `TaskEpisode` for storing task execution history
- `CodeSnippet` for code storage with language detection
- `ErrorSolution` for error pattern matching
- Working context management
- Changelog tracking for replication

## [2.1.0] - 2024

### Added
- `mq-markdown` integration for intelligent Markdown chunking
- Optional `chunking` feature flag
- Semantic document splitting based on structure

## [2.0.0] - 2024

### Added
- Hybrid database combining vector search with BM25 full-text search
- Optional vectors (documents can have text only, vectors only, or both)
- BM25 keyword search with TF-IDF scoring
- Metadata filtering with operators (`eq`, `ne`, `gt`, `lt`, `gte`, `lte`, `in`)
- Export/Import database as JSON
- Persist/Restore to workflow static data

### Changed
- Removed credentials requirement (fully serverless)
- Renamed save/load operations to export/import

### Breaking Changes
- API restructured for hybrid search support
- Storage layer redesigned for optional vectors

## [1.0.0] - 2024

### Added
- Initial embedded vector database for Rust
- HNSW approximate nearest neighbor search
- Flat index with brute-force exact search
- Multiple distance metrics: Cosine, Euclidean, Dot Product
- Memory-mapped file storage
- Thread-safe operations with `parking_lot`
- Python bindings (optional)
- Node.js bindings (optional)
