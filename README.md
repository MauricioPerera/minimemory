# minimemory

Embedded vector database for Rust, JavaScript, and Python. Like SQLite for vectors.

**520KB WASM** | **Zero deps** | **HNSW + BM25 + Filters** | **5 quantization types** | **437 tests** | **51 browser tests**

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
| **Durability (WAL)** | Opt-in write-ahead log; per-op append, checkpoint compaction, crash recovery (native Rust only) |
| **Metadata indexes** | Opt-in per-field indexes; sub-linear `$eq` and range filters (Rust core, not in WASM/JS bindings) |
| **Validation** | Rejects NaN/Inf vectors (`Error::InvalidVector`), dimension checks on insert and update |
| **Search contract** | Returns `min(k, qualifying)`; offset applied before truncation, filters before RRF fusion |
| **Replication** | `ConflictResolution` (LWW / KeepLocal / ApplyRemote); compaction preserves unexported log entries |
| **Indexing** | `VectorDB::rebuild_index()` — mandatory for IVF after bulk load to activate clustering |
| **WASM** | 520KB, runs in browser + Cloudflare Workers + Node.js |
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

## Metadata Indexes

Per-field indexes turn `$eq` and range (`$gt`/`$gte`/`$lt`/`$lte`) filters from a full scan into a candidate lookup. `$and` intersects indexed branches; `$or` unions them when every branch is indexable. The planner always re-evaluates the full filter over the candidates, so an index can only speed a query up — it never changes results (verified by an equivalence test against direct evaluation). Anything not accelerated (`$ne`, `$contains`, `$regex`, `Float` in `$eq`) falls back to full-scan with identical results.

```rust
use minimemory::{VectorDB, Config, Filter, Metadata};

let mut db = VectorDB::new(Config::new(128))?;

let mut m1 = Metadata::new();
m1.insert("category", "tech").insert("score", 0.9f64);
db.insert("a", &vec![0.1; 128], Some(m1))?;

let mut m2 = Metadata::new();
m2.insert("category", "tech").insert("score", 0.3f64);
db.insert("b", &vec![0.2; 128], Some(m2))?;

// Retroactive: indexes everything already in storage.
db.create_metadata_index("category")?;

let hits = db.filter_search(
    Filter::eq("category", "tech").and(Filter::gte("score", 0.5f64)),
    100,
)?;
```

Limitations: indexes are not persisted in `.mmdb` — recreate them with one retroactive `create_metadata_index` call after `open()`. Available in the Rust core only; not exposed in the WASM/JS bindings.

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

## Durability (WAL)

minimemory is memory-first: mutations apply in RAM and a `.mmdb` snapshot is a point-in-time dump of the whole database. The write-ahead log (WAL) is an opt-in layer that makes every individual mutation durable without taking a full snapshot.

With a WAL enabled, each successful `insert`/`update`/`delete`/`clear` is appended to the log (O(1) per op) **after** it has been applied in memory. `WalConfig::new()` (the default) flushes appends to the OS, which survives a process crash; `WalConfig::new().with_fsync_on_append(true)` issues an explicit `fsync` per append and also survives power loss. `checkpoint(snapshot_path)` writes an atomic `.mmdb` snapshot and then truncates the WAL (compaction); the order is deliberate, so a crash between the snapshot and the truncate still leaves a recoverable state. Recovery via `open_with_wal` (existing snapshot) or `new_with_wal` (no snapshot yet) replays the log with idempotent upsert semantics; a torn tail from a crash mid-append is truncated to the last valid entry.

```rust
use minimemory::{VectorDB, Config};
use minimemory::wal::WalConfig;

let mut db = VectorDB::new(Config::new(384))?;
db.enable_wal("appending.wal")?;          // &mut self; default survives process crash
db.insert("doc-1", &vec![0.1; 384], None)?;

// Opt in to power-loss durability.
db.enable_wal_with("appending.wal", WalConfig::new().with_fsync_on_append(true))?;
db.insert("doc-2", &vec![0.2; 384], None)?;

db.checkpoint("snap.mmdb")?;              // &self: atomic snapshot + WAL truncate

// Recover: load snapshot, replay any WAL entries appended after it.
let db = VectorDB::open_with_wal("snap.mmdb", "appending.wal")?;
```

Limitations: the WAL is native Rust only (not available in the WASM/JS bindings) and, in this first version, does not cover `insert_chunk`/`ingest_markdown`.

## Indexing & IVF

The IVF index does not train its clusters on insert. After a bulk load you must call `rebuild_index()` so K-means runs over all stored vectors; otherwise IVF silently falls back to brute-force search and `num_probes` has no effect. For HNSW and Flat the call is optional (useful to compact/reorganize after mass deletes).

```rust
use minimemory::{VectorDB, Config, IndexType};

let config = Config::new(384)
    .with_index(IndexType::IVF { num_clusters: 100, num_probes: 10 });
let db = VectorDB::new(config)?;

// Bulk insert...
for i in 0..10_000 {
    db.insert(&format!("doc-{i}"), &vec![0.1; 384], None)?;
}

// Mandatory for IVF: trains clusters so num_probes takes effect.
db.rebuild_index()?;
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

## WASM API (38 methods)

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

### Metadata indexes
`create_metadata_index`, `drop_metadata_index`, `list_metadata_indexes` — retroactive, accelerate `$eq`/range filters; not included in snapshots (recreate after `import_snapshot`)

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
| **@rckflr/minimemory** | npm package (520KB WASM) | [npm](https://www.npmjs.com/package/@rckflr/minimemory) |
| **miniCMS** | PocketBase-like CMS in browser | [Live](https://minicms.pages.dev) / [GitHub](https://github.com/MauricioPerera/minicms) |
| **minimemory-do-demo** | Cloudflare DO benchmark | [Live](https://minimemory-do-demo.rckflr.workers.dev) / [GitHub](https://github.com/MauricioPerera/minimemory-do-demo) |

## Architecture

```
minimemory (~25,400 LOC Rust)
├── db.rs              — VectorDB main API
├── distance/          — Cosine, Euclidean, DotProduct, Manhattan (SIMD)
├── index/             — Flat, HNSW, IVF
├── quantization.rs    — None, Int8, Int3, Binary, Polar
├── query/             — Filters ($eq, $gt, $regex, $and, $or)
├── search/            — Hybrid search (BM25 + vector + RRF)
├── storage/           — Memory, Disk (.mmdb v3), format
├── wal.rs             — Write-ahead log (opt-in durability, native only)
├── metadata_index.rs  — Per-field metadata indexes (sub-linear filters)
├── reranker.rs        — Trait-based cross-encoder
├── agent_memory.rs    — Semantic + episodic + working memory
├── memory_traits.rs   — Domain-agnostic memory system
├── bindings/wasm.rs   — 35-method WASM API
└── types.rs           — PagedResult, OrderBy, Config
```

A deep code audit (66 findings across core storage, indexes/SIMD, search/query/quantization, memory/replication, and bindings/embeddings) was performed and resolved in v3.0.0 — see [audit/AUDIT-SUMMARY.md](audit/AUDIT-SUMMARY.md).

## License

MIT

## Author

[Mauricio Perera](https://github.com/MauricioPerera)
