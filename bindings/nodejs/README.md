# minimemory - Node.js

Embedded vector database for Node.js/TypeScript. Like SQLite, but for vector similarity search.

## Installation

### From npm (when published)

```bash
npm install minimemory
```

### From source

```bash
# Requires Rust and napi-rs CLI
npm install -g @napi-rs/cli
cd bindings/nodejs
npm install
npm run build
```

## Usage

### JavaScript

```javascript
const { VectorDB } = require('minimemory');

// Create database with 384 dimensions
const db = new VectorDB({
  dimensions: 384,
  distance: 'cosine',
  indexType: 'hnsw'
});

// Insert vectors
const embedding = new Array(384).fill(0.1);
db.insert('doc-1', embedding, { title: 'My Document', score: 42 });

// Search for similar vectors
const query = new Array(384).fill(0.15);
const results = db.search(query, 10);

for (const result of results) {
  console.log(`ID: ${result.id}, Distance: ${result.distance}`);
}

// Save to disk
db.save('my_vectors.mmdb');

// Load from disk
const db2 = VectorDB.load('my_vectors.mmdb');

// Other operations
console.log(`Total vectors: ${db.length}`);
console.log(`Contains doc-1: ${db.contains('doc-1')}`);

const [vector, metadata] = db.get('doc-1');
db.delete('doc-1');
db.clear();
```

### TypeScript

```typescript
import { VectorDB, SearchResult } from 'minimemory';

const db = new VectorDB({
  dimensions: 384,
  distance: 'cosine',
  indexType: 'hnsw'
});

// Type-safe operations
db.insert('doc-1', new Float32Array(384).fill(0.1), {
  title: 'My Document'
});

const results: SearchResult[] = db.search(
  new Float32Array(384).fill(0.15),
  10
);

results.forEach(({ id, distance }) => {
  console.log(`${id}: ${distance}`);
});
```

## API Reference

### Constructor

```typescript
new VectorDB(options: {
  dimensions: number;       // Vector dimensions
  distance?: string;        // "cosine" | "euclidean" | "dot" (default: "cosine")
  indexType?: string;       // "flat" | "hnsw" (default: "flat")
})
```

### Methods

| Method | Description |
|--------|-------------|
| `insert(id, vector, metadata?)` | Insert a vector with optional metadata |
| `search(query, k)` | Find k nearest neighbors |
| `get(id)` | Get vector and metadata by ID |
| `delete(id)` | Delete vector by ID |
| `contains(id)` | Check if ID exists |
| `update(id, vector, metadata?)` | Update existing vector |
| `save(path)` | Save database to file |
| `VectorDB.load(path)` | Load database from file |
| `clear()` | Remove all vectors |

### Properties

| Property | Description |
|----------|-------------|
| `length` | Number of vectors |
| `dimensions` | Configured dimensions |

## Performance Tips

1. Use `Float32Array` instead of regular arrays for better performance
2. Use `hnsw` index for datasets > 10,000 vectors
3. Batch operations when possible
4. Use `cosine` distance for text embeddings

## CommonJS vs ESM

```javascript
// CommonJS
const { VectorDB } = require('minimemory');

// ESM
import { VectorDB } from 'minimemory';
```
