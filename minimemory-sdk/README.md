# @minimemory/sdk

JavaScript/TypeScript SDK for the minimemory vector database service.

[![npm version](https://badge.fury.io/js/@minimemory%2Fsdk.svg)](https://www.npmjs.com/package/@minimemory/sdk)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Features

- Full TypeScript support with complete type definitions
- Memory operations (CRUD, search, import/export)
- Knowledge Bank (RAG) with document ingestion and chunking
- Hybrid search (vector + BM25 keyword)
- Embedding generation via Workers AI
- JWT and API Key authentication
- Automatic error handling with typed exceptions

## Installation

```bash
npm install @minimemory/sdk
```

## Quick Start

```typescript
import { createClient } from '@minimemory/sdk';

const client = createClient({
  baseUrl: 'https://your-worker.workers.dev/api/v1',
  apiKey: 'mm_your_api_key',
  namespace: 'my-app'
});

// Store a memory
await client.remember('User prefers dark mode', {
  type: 'semantic',
  importance: 0.8,
  metadata: { category: 'preferences' }
});

// Search memories with hybrid search
const results = await client.recall('user preferences', {
  mode: 'hybrid',
  limit: 5
});

console.log(results.results);
```

## Configuration

```typescript
import { createClient } from '@minimemory/sdk';

const client = createClient({
  // Required: Base URL of the minimemory service API
  baseUrl: 'https://your-worker.workers.dev/api/v1',

  // Authentication (choose one or both)
  apiKey: 'mm_your_api_key',    // API Key authentication
  accessToken: 'jwt_token',      // JWT Bearer token

  // Optional settings
  namespace: 'default',          // Default namespace for operations
  timeout: 30000,                // Request timeout in milliseconds
  headers: {                     // Custom headers for all requests
    'X-Custom-Header': 'value'
  },
});

// Runtime configuration
client.setNamespace('production');
client.setApiKey('mm_new_key');
client.setAccessToken('new_jwt_token');
```

## API Reference

### Memory Operations

#### `remember(content, options?)`

Store a new memory in the database.

```typescript
const result = await client.remember('Important information', {
  type: 'semantic',           // 'episodic' | 'semantic' | 'working' | 'knowledge'
  importance: 0.8,            // 0-1 scale, affects decay and retrieval
  metadata: { key: 'value' }, // Arbitrary metadata
  embedding: [...],           // Pre-computed embedding (optional)
  generateEmbedding: true,    // Auto-generate embedding (default: true)
  sessionId: 'session_123',   // Session ID (for working memory)
  ttl: 3600000,               // Time-to-live in ms (for working memory)
});

// Response
{
  success: true,
  memory: {
    id: 'mem_abc123',
    type: 'semantic',
    content: 'Important information',
    importance: 0.8,
    createdAt: 1703520000000
  },
  embeddingGenerated: true,
  persisted: true
}
```

#### `recall(query, options?)`

Search for relevant memories using vector similarity, keyword matching, or hybrid search.

```typescript
// Simple text query (auto-generates embedding)
const results = await client.recall('user preferences');

// With options
const results = await client.recall('dark mode settings', {
  type: 'semantic',        // Filter by memory type
  limit: 10,               // Max results (default: 10)
  minSimilarity: 0.7,      // Minimum similarity score
  minImportance: 0.5,      // Minimum importance threshold
  mode: 'hybrid',          // 'vector' | 'keyword' | 'hybrid'
  alpha: 0.5,              // Hybrid weight: 0=keyword only, 1=vector only
  sessionId: 'session_123' // Filter by session
});

// With pre-computed embedding
const results = await client.recall({
  embedding: [0.1, 0.2, 0.3, ...],
  mode: 'vector',
  limit: 5
});

// Response
{
  success: true,
  count: 3,
  embeddingGenerated: true,
  results: [
    {
      id: 'mem_abc123',
      type: 'semantic',
      content: 'User prefers dark mode',
      score: 0.95,
      vectorSimilarity: 0.92,
      keywordScore: 0.98,
      importance: 0.8,
      metadata: { category: 'preferences' },
      createdAt: 1703520000000,
      source: { ... }  // For knowledge memories
    }
  ]
}
```

#### `get(id)`

Retrieve a specific memory by ID.

```typescript
const { memory } = await client.get('mem_abc123');
```

#### `update(id, updates)`

Update an existing memory.

```typescript
const result = await client.update('mem_abc123', {
  content: 'Updated content',
  importance: 0.9,
  metadata: { updated: true }
});
```

#### `forget(id)`

Delete a specific memory.

```typescript
const result = await client.forget('mem_abc123');
// { success: true, message: 'Memory forgotten' }
```

#### `forgetByFilter(filter)`

Delete multiple memories matching a filter.

```typescript
const result = await client.forgetByFilter({ type: 'working' });
// { success: true, count: 5, message: 'Forgot 5 memories' }
```

#### `stats()`

Get memory statistics for the current namespace.

```typescript
const { stats } = await client.stats();
// {
//   total: 150,
//   byType: { episodic: 50, semantic: 80, working: 15, knowledge: 5 },
//   averageImportance: 0.72,
//   oldestMemory: 1703000000000,
//   newestMemory: 1703520000000,
//   knowledgeSources: 3
// }
```

#### `cleanup()`

Remove expired working memories.

```typescript
const { count } = await client.cleanup();
// count: number of removed memories
```

#### `decay()`

Apply importance decay to all memories (reduces importance over time).

```typescript
await client.decay();
```

#### `export()` / `import(memories)`

Export and import memories for backup or migration.

```typescript
// Export
const { data } = await client.export();
console.log(data.memories);

// Import
await client.import(data.memories);
```

#### `clear()`

Delete all memories in the namespace.

```typescript
await client.clear();
```

### Knowledge Bank (RAG)

The Knowledge Bank provides document ingestion and retrieval for RAG applications.

#### `knowledge.ingest(options)`

Ingest a document, automatically chunking and generating embeddings.

```typescript
const result = await client.knowledge.ingest({
  content: documentText,           // Document content
  name: 'product-guide.md',        // Source name
  type: 'document',                // 'document' | 'url' | 'api' | 'manual'
  url: 'https://...',              // Original URL (optional)
  mimeType: 'text/markdown',       // Content type
  metadata: { version: '2.0' },    // Custom metadata
  chunking: {
    chunkSize: 1000,               // Target characters per chunk
    chunkOverlap: 200,             // Overlap between chunks
    separators: ['\n\n', '\n'],    // Custom separators
    preserveParagraphs: true       // Keep paragraphs intact
  },
  generateEmbeddings: true         // Generate embeddings for chunks
});

// Response
{
  success: true,
  sourceId: 'src_xyz789',
  sourceName: 'product-guide.md',
  chunksCreated: 15,
  embeddingsGenerated: true,
  totalCharacters: 12500,
  averageChunkSize: 833,
  durationMs: 1250
}
```

#### `knowledge.listSources(options?)`

List all ingested knowledge sources.

```typescript
const { sources, total, hasMore } = await client.knowledge.listSources({
  type: 'document',  // Filter by type
  limit: 20,
  offset: 0
});
```

#### `knowledge.getSource(id)`

Get details of a specific source.

```typescript
const { source } = await client.knowledge.getSource('src_xyz789');
```

#### `knowledge.deleteSource(id)`

Delete a source and all its chunks.

```typescript
await client.knowledge.deleteSource('src_xyz789');
```

#### `knowledge.getChunks(sourceId, options?)`

Get chunks for a specific source.

```typescript
const { chunks, total } = await client.knowledge.getChunks('src_xyz789', {
  limit: 50,
  offset: 0
});
```

#### `knowledge.stats()`

Get knowledge bank statistics.

```typescript
const { stats } = await client.knowledge.stats();
// { totalSources: 5, totalChunks: 150, byType: {...}, averageChunksPerSource: 30 }
```

#### `knowledge.previewChunking(content, options?)`

Preview how content will be chunked without storing.

```typescript
const preview = await client.knowledge.previewChunking(longDocument, {
  chunkSize: 500,
  chunkOverlap: 100
});
// { totalChunks: 25, chunks: [{ index, length, preview, ... }] }
```

### Embedding Operations

Generate embeddings using Workers AI (Gemma Embedding model).

#### `embed.single(text, options?)`

Generate embedding for a single text.

```typescript
const { embedding, dimensions, model } = await client.embed.single('Hello world', {
  dimensions: 768  // 768 | 512 | 256 | 128 (Matryoshka)
});
```

#### `embed.batch(texts, options?)`

Generate embeddings for multiple texts.

```typescript
const { embeddings, count } = await client.embed.batch([
  'First text',
  'Second text',
  'Third text'
], { dimensions: 256 });
```

#### `embed.info()`

Get embedding service information.

```typescript
const info = await client.embed.info();
// {
//   available: true,
//   model: '@cf/google/gemma-embedding-300m',
//   dimensions: { default: 768, available: [768, 512, 256, 128] },
//   matryoshka: true
// }
```

## Error Handling

The SDK throws `MiniMemoryError` for all API errors.

```typescript
import { MiniMemoryError } from '@minimemory/sdk';

try {
  await client.get('invalid_id');
} catch (error) {
  if (error instanceof MiniMemoryError) {
    console.log(error.message);  // Human-readable error message
    console.log(error.status);   // HTTP status code (e.g., 404)
    console.log(error.code);     // Error code (e.g., 'TIMEOUT', 'NETWORK')
  }
}
```

Common error codes:
- `404` - Memory/resource not found
- `400` - Invalid request parameters
- `401` - Authentication required
- `403` - Permission denied
- `429` - Rate limit exceeded
- `500` - Server error
- `TIMEOUT` - Request timeout

## TypeScript Types

All types are exported for TypeScript users:

```typescript
import type {
  // Memory types
  Memory,
  MemoryType,
  RecallResult,
  MemoryStats,

  // Options
  RememberOptions,
  RecallOptions,
  ForgetFilter,
  UpdateOptions,

  // Knowledge types
  KnowledgeSource,
  KnowledgeChunk,
  KnowledgeStats,
  ChunkingOptions,
  IngestOptions,
  IngestResult,

  // Embedding types
  EmbeddingDimensions,
  EmbedOptions,
  EmbedResult,
  EmbedBatchResult,

  // Configuration
  MiniMemoryConfig,
  RequestOptions
} from '@minimemory/sdk';
```

## Memory Types

The SDK supports four memory types:

| Type | Description | Use Case |
|------|-------------|----------|
| `episodic` | Specific events/experiences | User actions, conversations, events |
| `semantic` | Facts and knowledge | User preferences, learned information |
| `working` | Temporary context | Current session data, expires with TTL |
| `knowledge` | RAG document chunks | Ingested documents for retrieval |

## Examples

### Building a Chatbot Memory

```typescript
const client = createClient({
  baseUrl: 'https://memory.example.com/api/v1',
  apiKey: process.env.MINIMEMORY_API_KEY,
  namespace: 'chatbot'
});

// Store conversation context
await client.remember(`User asked about pricing for enterprise plan`, {
  type: 'episodic',
  importance: 0.7,
  metadata: {
    userId: 'user_123',
    topic: 'pricing',
    timestamp: Date.now()
  }
});

// Retrieve relevant context for response
const context = await client.recall('enterprise pricing', {
  mode: 'hybrid',
  limit: 5,
  minSimilarity: 0.6
});

// Use context.results in your LLM prompt
```

### RAG Document Search

```typescript
// Ingest documentation
await client.knowledge.ingest({
  content: fs.readFileSync('docs/api-reference.md', 'utf-8'),
  name: 'api-reference.md',
  type: 'document',
  chunking: { chunkSize: 800, chunkOverlap: 150 }
});

// Search for relevant chunks
const results = await client.recall('authentication endpoints', {
  type: 'knowledge',
  mode: 'hybrid',
  limit: 3
});

// Results include source citations
results.results.forEach(r => {
  console.log(`From: ${r.source?.name} (chunk ${r.source?.chunkIndex})`);
  console.log(r.content);
});
```

### Session-Based Working Memory

```typescript
const sessionId = 'session_' + Date.now();

// Store temporary context
await client.remember('User is browsing product category: Electronics', {
  type: 'working',
  sessionId,
  ttl: 30 * 60 * 1000,  // 30 minutes
  importance: 0.5
});

// Retrieve session context
const context = await client.recall('current browsing context', {
  type: 'working',
  sessionId,
  mode: 'keyword'
});

// Cleanup expired memories periodically
await client.cleanup();
```

## Development

```bash
# Install dependencies
npm install

# Run unit tests
npm test

# Run E2E tests (requires minimemory-service at localhost:8787)
npm run test:e2e

# Run all tests
npm run test:all

# Build
npm run build

# Watch mode
npm run test:watch
```

## Requirements

- Node.js >= 18.0.0
- minimemory-service instance

## License

MIT
