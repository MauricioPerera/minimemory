# minimemory

Embedded vector database for Rust, JavaScript, and Python. Like SQLite for vectors.

**493KB WASM** | **Zero deps** | **HNSW + BM25 + Filters** | **5 quantization types** | **314 Rust tests** | **51 browser tests**

[![npm](https://img.shields.io/npm/v/@rckflr/minimemory)](https://www.npmjs.com/package/@rckflr/minimemory)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

## What it is

minimemory is an embedded vector database that runs everywhere: native Rust, browser via WASM, Cloudflare Workers, Node.js. It combines vector similarity search, BM25 full-text search, and metadata filters in a single library with no external dependencies.

- **[npm package](https://www.npmjs.com/package/@rckflr/minimemory)** — `npm install @rckflr/minimemory`
- **[miniCMS](https://minicms.pages.dev)** — Full CMS built on minimemory, runs 100% in browser
- **[Cloudflare DO demo](https://minimemory-do-demo.rckflr.workers.dev)** — Durable Objects benchmark vs D1

## Install

**Rust:**
```toml
[dependencies]
minimemory = { git = "https://github.com/MauricioPerera/minimemory" }
```

**JavaScript/TypeScript:**
```bash
npm install @rckflr/minimemory
```

## Quick Start

**Rust:**
```rust
use minimemory::{VectorDB, Config, Distance};

let db = VectorDB::new(Config::new(384))?;
db.insert("doc-1", &vec![0.1; 384], None)?;
let results = db.search(&vec![0.15; 384], 10)?;
```

**JavaScript:**
```javascript
import init, { WasmVectorDB } from '@rckflr/minimemory';
await init();

const db = new WasmVectorDB(384, "cosine", "flat");
db.insert("doc-1", new Float32Array(384));
const results = JSON.parse(db.search(new Float32Array(384), 10));
```

## Features

| Category | Features |
|----------|----------|
| **Distance metrics** | Cosine, Euclidean, DotProduct, Manhattan |
| **Index types** | Flat (exact), HNSW (approximate), IVF (clustered) |
| **Quantization** | None (f32), Int8 (4x), Int3 (10.7x), Binary (32x), Polar (21x) |
| **Search** | Vector similarity, BM25 keywords, hybrid (RRF fusion), metadata filters |
| **Filters** | $eq, $ne, $gt, $gte, $lt, $lte, $contains, $regex, $and, $or |
| **Query** | ORDER BY any field, OFFSET/LIMIT pagination, PagedResult |
| **Persistence** | .mmdb binary format v3 with CRC32 checksums, atomic writes |
| **WASM** | 493KB, runs in browser + Cloudflare Workers + Node.js |
| **Extras** | Reranker (trait-based), agent memory system, local embeddings (Candle) |

## Quantization

| Type | Compression | Accuracy | Memory (10K x 384d) |
|------|-------------|----------|---------------------|
| None (f32) | 1x | 100% | 15.0 MB |
| Int8 | 4x | ~99% | 3.8 MB |
| Int3 | 10.7x | ~96% | 1.4 MB |
| Polar | 21x | ~90% | 0.7 MB |
| Binary | 32x | ~90% | 0.5 MB |

```javascript
// JavaScript
const db = WasmVectorDB.new_int3(384, "cosine", "flat"); // 10.7x compression
```

```rust
// Rust
let config = Config::new(384)
    .with_quantization(QuantizationType::Int3);
```

## Filter Syntax

**Rust (builder pattern):**
```rust
use minimemory::Filter;

Filter::eq("category", "tech")
    .and(Filter::gt("score", 0.5f64))
    .or(Filter::regex("title", "^Rust"));
```

**JavaScript (MongoDB-style JSON):**
```javascript
db.filter_search('{"category": "tech"}', 100);
db.filter_search('{"score": {"$gt": 0.5}}', 100);
db.filter_search('{"$and": [{"category": "tech"}, {"year": {"$gte": 2024}}]}', 100);
db.filter_search('{"title": {"$regex": "^Rust"}}', 100);
```

## Pagination

```rust
// Rust
let page = db.list_documents(
    Some(Filter::eq("status", "active")),
    Some(OrderBy::desc("created_at")),
    10,  // limit
    0,   // offset
)?;
// page.items, page.total, page.has_more()
```

```javascript
// JavaScript
const page = JSON.parse(db.list_documents(
    '{"status": "active"}',
    "created_at", true, 10, 0
));
// { items: [...], total: 42, has_more: true }
```

## Persistence

**Rust (.mmdb files):**
```rust
db.save("my_database.mmdb")?;
let db = VectorDB::open("my_database.mmdb")?;
```

**JavaScript (export/import):**
```javascript
const snapshot = db.export_snapshot();
localStorage.setItem("my-db", snapshot);

// Later...
db.import_snapshot(localStorage.getItem("my-db"));
```

## Browser Usage

```html
<script type="module">
import init, { WasmVectorDB } from './minimemory.js';
await init();

const db = new WasmVectorDB(384, "cosine", "flat");
db.insert_document("user-1", null, JSON.stringify({
    name: "Alice", role: "admin"
}));

const page = JSON.parse(db.list_documents(
    '{"role": "admin"}', "name", false, 10, 0
));
</script>
```

## Cloudflare Workers

```javascript
import init, { WasmVectorDB } from '@rckflr/minimemory';
import wasmModule from '@rckflr/minimemory/minimemory_bg.wasm';

export default {
    async fetch(request, env) {
        await init(wasmModule);
        const db = new WasmVectorDB(384, "cosine", "flat");
        // ... use db
    }
}
```

## WASM API (35 methods)

### Constructors
| Method | Description |
|--------|-------------|
| `new WasmVectorDB(dims, distance, index)` | Create database |
| `WasmVectorDB.new_int8(dims, dist, idx)` | 4x compressed |
| `WasmVectorDB.new_int3(dims, dist, idx)` | 10.7x compressed |
| `WasmVectorDB.new_binary(dims, dist, idx)` | 32x compressed |
| `WasmVectorDB.new_hnsw(dims, dist, m, ef)` | Custom HNSW |
| `WasmVectorDB.new_with_config(...)` | Full config |

### CRUD
`insert`, `insert_with_metadata`, `insert_document`, `get`, `delete`, `update`, `update_with_metadata`, `contains`, `ids`, `len`, `is_empty`, `clear`

### Search
`search`, `keyword_search`, `filter_search`, `search_with_filter`, `list_documents`, `search_paged`

### Persistence
`export_snapshot`, `import_snapshot`

### Matryoshka
`insert_auto`, `insert_auto_with_metadata`, `search_auto`, `update_auto`, `update_auto_with_metadata`

## Benchmarks

**Cloudflare Workers production** (Durable Objects):

| Docs | Dims | Index | Search time |
|------|------|-------|-------------|
| 100 | 64 | Flat | <1ms |
| 1,000 | 64 | Flat | <1ms |
| 1,000 | 64 | HNSW | <1ms |
| 5,000 | 64 | Flat+Int3 | <1ms |

**vs D1**: minimemory 3x faster for vector search (50 docs benchmark).

## Ecosystem

| Project | Description | Link |
|---------|-------------|------|
| **minimemory** | Core vector DB (Rust + WASM) | [GitHub](https://github.com/MauricioPerera/minimemory) |
| **@rckflr/minimemory** | npm package (493KB WASM) | [npm](https://www.npmjs.com/package/@rckflr/minimemory) |
| **miniCMS** | PocketBase-like CMS in browser | [Live](https://minicms.pages.dev) / [GitHub](https://github.com/MauricioPerera/minicms) |
| **minimemory-do-demo** | Cloudflare DO benchmark | [Live](https://minimemory-do-demo.rckflr.workers.dev) / [GitHub](https://github.com/MauricioPerera/minimemory-do-demo) |

## Architecture

```
minimemory (19,700 LOC Rust)
├── db.rs              — VectorDB main API
├── distance/          — Cosine, Euclidean, DotProduct, Manhattan (SIMD)
├── index/             — Flat, HNSW, IVF
├── quantization.rs    — None, Int8, Int3, Binary, Polar
├── query/             — Filters ($eq, $gt, $regex, $and, $or)
├── search/            — Hybrid search (BM25 + vector + RRF)
├── storage/           — Memory, Disk (.mmdb v3), format
├── reranker.rs        — Trait-based cross-encoder
├── agent_memory.rs    — Semantic + episodic + working memory
├── memory_traits.rs   — Domain-agnostic memory system
├── bindings/wasm.rs   — 35-method WASM API
└── types.rs           — PagedResult, OrderBy, Config
```

## License

MIT

## Author

[Mauricio Perera](https://github.com/MauricioPerera)
