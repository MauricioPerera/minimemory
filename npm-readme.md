# minimemory

Embedded vector database for JavaScript/TypeScript. Like SQLite for vectors — runs in the browser, Cloudflare Workers, and Node.js via WebAssembly.

**493KB WASM** | **Zero dependencies** | **HNSW + BM25 + Filters** | **5 quantization levels**

## Install

```bash
npm install minimemory
```

## Quick Start

```javascript
import init, { WasmVectorDB } from 'minimemory';

await init();

// Create a database (384 dimensions, cosine similarity)
const db = new WasmVectorDB(384, "cosine", "flat");

// Insert vectors with metadata
db.insert_with_metadata("doc-1", new Float32Array(384), JSON.stringify({
  title: "Introduction to Rust",
  category: "programming",
  year: 2024
}));

// Semantic search
const results = JSON.parse(db.search(new Float32Array(384), 10));
// [{ id: "doc-1", distance: 0.05, metadata: { title: "...", ... } }]

// Full-text keyword search
const hits = JSON.parse(db.keyword_search("rust programming", 10));

// Filter by metadata (like SQL WHERE)
const filtered = JSON.parse(db.filter_search('{"category": "programming"}', 50));

// List with ORDER BY + pagination
const page = JSON.parse(db.list_documents(
  '{"category": "programming"}',  // filter (or "{}" for all)
  "year",                          // order by field
  true,                            // descending
  10,                              // limit
  0                                // offset
));
// { items: [...], total: 42, has_more: true }
```

## Document Store (No Vector Required)

Use minimemory as a document database — no embeddings needed:

```javascript
const db = new WasmVectorDB(1, "cosine", "flat"); // minimal dims

// Insert documents with metadata only
db.insert_document("user-1", null, JSON.stringify({
  name: "Alice",
  email: "alice@example.com",
  role: "admin",
  created_at: "2024-01-15"
}));

// Query by metadata
const admins = JSON.parse(db.filter_search('{"role": "admin"}', 100));

// List with pagination
const page = JSON.parse(db.list_documents("{}", "created_at", true, 10, 0));
```

## Persistence (Browser)

Export to IndexedDB, localStorage, or any storage:

```javascript
// Save
const snapshot = db.export_snapshot();
localStorage.setItem("my-db", snapshot);

// Restore
const saved = localStorage.getItem("my-db");
if (saved) {
  const count = db.import_snapshot(saved);
  console.log(`Restored ${count} documents`);
}
```

## Quantization (Memory Compression)

| Constructor | Compression | Accuracy | Use Case |
|-------------|-------------|----------|----------|
| `new WasmVectorDB(dims, dist, idx)` | 1x | 100% | Default |
| `WasmVectorDB.new_int8(dims, dist, idx)` | 4x | ~99% | General purpose |
| `WasmVectorDB.new_int3(dims, dist, idx)` | 10.7x | ~96% | Browser / edge |
| `WasmVectorDB.new_binary(dims, dist, idx)` | 32x | ~90% | Massive scale |

```javascript
// 10K vectors at 384 dimensions:
// Default: 15 MB | Int8: 3.8 MB | Int3: 1.4 MB | Binary: 0.5 MB
const db = WasmVectorDB.new_int3(384, "cosine", "flat");
```

## Distance Metrics

- `"cosine"` — Cosine similarity (best for embeddings)
- `"euclidean"` — L2 distance
- `"dot"` — Dot product
- `"manhattan"` — L1 distance

## Index Types

- `"flat"` — Exact brute-force search (best for < 10K vectors)
- `"hnsw"` — Approximate nearest neighbor (best for > 10K vectors)

```javascript
// HNSW with custom parameters
const db = WasmVectorDB.new_hnsw(384, "cosine", 16, 200);

// Full configuration
const db = WasmVectorDB.new_with_config(
  384,      // dimensions
  "cosine", // distance
  "hnsw",   // index
  "int3",   // quantization: "none", "int8", "int3", "binary", "polar"
  16,       // hnsw_m (optional)
  200       // hnsw_ef (optional)
);
```

## Filter Syntax

MongoDB-style filters for metadata queries:

```javascript
// Equality
db.filter_search('{"status": "active"}', 100)

// Comparison
db.filter_search('{"price": {"$gt": 10}}', 100)
db.filter_search('{"score": {"$gte": 0.5, "$lt": 1.0}}', 100)

// String operations
db.filter_search('{"title": {"$contains": "rust"}}', 100)
db.filter_search('{"name": {"$regex": "^Al"}}', 100)

// Logical operators
db.filter_search('{"$and": [{"category": "tech"}, {"year": {"$gte": 2024}}]}', 100)
db.filter_search('{"$or": [{"status": "active"}, {"priority": {"$gt": 5}}]}', 100)

// Vector search with filter
db.search_with_filter(queryVector, 10, '{"category": "tech"}')
```

## Cloudflare Workers

```javascript
import init, { WasmVectorDB } from 'minimemory';
import wasmModule from 'minimemory/minimemory_bg.wasm';

export default {
  async fetch(request, env) {
    await init(wasmModule);
    const db = new WasmVectorDB(384, "cosine", "flat");
    // ... use db
  }
}
```

## Matryoshka Embeddings

Auto-truncate higher-dimensional embeddings:

```javascript
const db = new WasmVectorDB(256, "cosine", "flat");

// Insert 768-dim embedding, auto-truncated to 256
db.insert_auto("doc-1", new Float32Array(768));

// Search with 768-dim query, auto-truncated
const results = JSON.parse(db.search_auto(new Float32Array(768), 10));
```

## API Reference

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
| Method | Description |
|--------|-------------|
| `insert(id, vector)` | Insert vector |
| `insert_with_metadata(id, vector, json)` | Insert with metadata |
| `insert_document(id, vector?, json)` | Insert document (vector optional) |
| `get(id)` | Get by ID |
| `delete(id)` | Delete by ID |
| `update(id, vector)` | Update vector |
| `contains(id)` | Check if exists |
| `ids()` | Get all IDs (JSON) |
| `len()` | Count documents |
| `clear()` | Delete all |

### Search
| Method | Description |
|--------|-------------|
| `search(vector, k)` | Semantic similarity search |
| `keyword_search(text, k)` | BM25 full-text search |
| `filter_search(filterJson, limit)` | Metadata filter search |
| `search_with_filter(vector, k, filterJson)` | Vector + filter |
| `list_documents(filter, orderField, desc, limit, offset)` | Paginated listing |
| `search_paged(vector, limit, offset)` | Paginated vector search |

### Persistence
| Method | Description |
|--------|-------------|
| `export_snapshot()` | Export as JSON string |
| `import_snapshot(json)` | Import from JSON (returns count) |

### Matryoshka
| Method | Description |
|--------|-------------|
| `insert_auto(id, vector)` | Auto-truncate + normalize |
| `search_auto(vector, k)` | Auto-truncate query |

## Size

| Component | Size |
|-----------|------|
| WASM binary | 493 KB |
| JS wrapper | ~30 KB |
| TypeScript types | included |
| Total (gzipped) | ~190 KB |

## License

MIT
