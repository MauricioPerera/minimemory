# minimemory-service

Agentic memory service - serverless vector database with automatic embeddings, hybrid search, and quantization.

## Features

- **Auto Embeddings**: Automatic embedding generation via Workers AI (EmbeddingGemma)
- **Vector Search**: Cosine, Euclidean, and Dot Product distance metrics
- **Hybrid Search**: Combine vector similarity with BM25 keyword search
- **Vector Quantization**: Reduce memory usage by 75-97% with int8/binary quantization
- **Matryoshka**: Truncate embeddings to 768, 512, 256, or 128 dimensions
- **Metadata Filtering**: Rich query operators ($eq, $gt, $in, $contains, etc.)
- **Multi-Namespace**: Isolate data per user/tenant
- **Audit Logging**: Full traceability of all operations
- **Knowledge Bank**: RAG document ingestion with source citation
- **Edge-Ready**: Runs on Cloudflare Workers with D1 persistence

## Quick Start

### Installation

```bash
npm install
```

### Local Development

```bash
npm run dev
```

### Run Tests

```bash
npm test
```

### Deploy to Cloudflare

```bash
npm run deploy
```

## API Overview

All endpoints use `X-Namespace` header for multi-tenancy.

### Memory Operations

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/v1/remember` | POST | Store a new memory |
| `/api/v1/recall` | POST | Search for memories |
| `/api/v1/memory/:id` | GET | Get a specific memory |
| `/api/v1/memory/:id` | PATCH | Update a memory |
| `/api/v1/forget/:id` | DELETE | Delete a memory |
| `/api/v1/forget` | POST | Delete memories by filter |
| `/api/v1/clear` | DELETE | Clear all memories |

### Embeddings

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/v1/embed` | POST | Generate embeddings (EmbeddingGemma) |
| `/api/v1/embed/info` | GET | Get embedding service info |

### Management

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/v1/stats` | GET | Get memory statistics |
| `/api/v1/export` | POST | Export all memories |
| `/api/v1/import` | POST | Import memories |
| `/api/v1/cleanup` | POST | Remove expired memories |
| `/api/v1/decay` | POST | Apply importance decay |

### Audit Log

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/v1/audit` | GET | Query audit logs |
| `/api/v1/audit/:id` | GET | Get audit entry by ID |
| `/api/v1/audit/resource/:type/:id` | GET | Get resource history |
| `/api/v1/audit/user/:id` | GET | Get user activity |
| `/api/v1/audit/stats` | GET | Get audit statistics |
| `/api/v1/audit/cleanup` | POST | Clean up old logs |

### Knowledge Bank (RAG)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/v1/knowledge/ingest` | POST | Ingest document into knowledge bank |
| `/api/v1/knowledge/sources` | GET | List knowledge sources |
| `/api/v1/knowledge/sources/:id` | GET | Get source details |
| `/api/v1/knowledge/sources/:id` | DELETE | Delete source and chunks |
| `/api/v1/knowledge/sources/:id/chunks` | GET | Get source chunks |
| `/api/v1/knowledge/stats` | GET | Get knowledge bank statistics |
| `/api/v1/knowledge/chunk-preview` | POST | Preview document chunking |

## Core Components

### VectorDB

In-memory vector database with support for:

```typescript
import { VectorDB } from './core/VectorDB.js';

const db = new VectorDB({
  dimensions: 768,
  distance: 'cosine',      // 'cosine' | 'euclidean' | 'dot'
  quantization: 'int8',    // 'none' | 'int8' | 'binary'
});

// Insert vectors
db.insert('doc-1', embedding, { title: 'Hello World' });

// Search
const results = db.search(queryVector, 10, {
  filter: { type: 'article' },
  minSimilarity: 0.7,
});

// Hybrid search (vector + keyword)
const hybridResults = db.hybridSearch({
  mode: 'hybrid',
  queryVector,
  keywords: 'machine learning',
  k: 10,
  alpha: 0.7, // 70% vector, 30% keyword
});
```

### Quantization

Reduce memory usage dramatically:

| Type | Memory Reduction | Accuracy | Use Case |
|------|-----------------|----------|----------|
| `none` | 0% | 100% | Small datasets (<10K) |
| `int8` | 75% | ~99% | Large datasets, high accuracy |
| `binary` | 97% | ~95% | Massive datasets, fast search |

```typescript
const db = new VectorDB({
  dimensions: 768,
  quantization: 'int8',  // 75% memory savings
});
```

### Auto Embeddings (Workers AI)

When Workers AI is configured, embeddings are automatically generated:

```bash
# Store memory - embedding generated automatically
curl -X POST /api/v1/remember \
  -d '{"content": "User prefers dark mode"}'

# Response includes embeddingGenerated: true
```

**EmbeddingGemma-300m specs:**
- 768 dimensions (Matryoshka: truncatable to 512, 256, 128)
- 100+ languages supported
- ~15ms inference latency
- $0.011 per 1000 neurons

**Cost estimate:**
| Scale | 768d | 256d |
|-------|------|------|
| 10K embeddings | $84.48 | $28.16 |
| 100K embeddings | $844.80 | $281.60 |

**Free tier:** 10,000 neurons/day (~13 embeddings at 768d)

### Metadata Filtering

Rich query operators:

```typescript
db.search(query, 10, {
  filter: {
    type: 'article',
    importance: { $gte: 0.7 },
    tags: { $in: ['ai', 'ml'] },
    'author.verified': true,
    $or: [
      { status: 'published' },
      { status: 'draft', createdBy: 'admin' }
    ]
  }
});
```

Supported operators:
- `$eq`, `$ne` - Equality
- `$gt`, `$gte`, `$lt`, `$lte` - Comparison
- `$in`, `$nin` - Array membership
- `$exists` - Field existence
- `$contains`, `$startsWith`, `$endsWith` - String matching
- `$and`, `$or` - Logical operators

## Memory Types

| Type | Description | TTL |
|------|-------------|-----|
| `semantic` | Facts, preferences, knowledge | None |
| `episodic` | Events, conversations | None |
| `working` | Temporary context | Optional |

## Configuration

### Environment Variables

```bash
# Cloudflare D1 Database
DATABASE=minimemory

# JWT Authentication (optional)
JWT_SECRET=your-secret-key
```

### wrangler.toml

```toml
name = "minimemory-service"
main = "src/index.ts"
compatibility_date = "2024-01-01"

# D1 Database (persistence)
[[d1_databases]]
binding = "DB"
database_name = "minimemory"
database_id = "your-database-id"

# Workers AI (auto embeddings)
[ai]
binding = "AI"
```

## Cost Estimation

With quantization and Cloudflare pricing:

| Scale | Storage (D1) | Compute | Total/month |
|-------|-------------|---------|-------------|
| 10K memories | ~$0.02 | Free tier | ~$0.02 |
| 100K memories | ~$0.15 | Free tier | ~$0.15 |
| 1M memories | ~$1.50 | ~$5 | ~$6.50 |

## Documentation

- [API Reference](./API.md)
- [Quantization Guide](./QUANTIZATION.md)
- [Audit Logging Guide](./AUDIT.md)
- [Knowledge Bank Guide](./KNOWLEDGE.md)

## License

MIT
