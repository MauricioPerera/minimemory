# Changelog

All notable changes to minimemory will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
