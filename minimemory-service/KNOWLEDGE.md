# Knowledge Bank Guide

RAG (Retrieval-Augmented Generation) knowledge management for minimemory-service.

## Overview

The Knowledge Bank feature provides:

- **Document Ingestion**: Chunk and embed documents automatically
- **Source Tracking**: Full provenance for every chunk
- **Source Citation**: Results include source attribution
- **Multiple Source Types**: Documents, URLs, APIs, manual input
- **Configurable Chunking**: Control chunk size and overlap

## Quick Start

### Ingest a Document

```bash
curl -X POST http://localhost:8787/api/v1/knowledge/ingest \
  -H "Content-Type: application/json" \
  -H "X-API-Key: mm_dev_key_12345" \
  -H "X-Namespace: default" \
  -d '{
    "name": "product-docs.md",
    "content": "# Product Documentation\n\nThis is the complete documentation...",
    "type": "document",
    "mimeType": "text/markdown",
    "metadata": { "version": "1.0", "author": "team" },
    "chunking": {
      "chunkSize": 1000,
      "chunkOverlap": 200
    }
  }'
```

**Response:**

```json
{
  "success": true,
  "sourceId": "src_m1abc_xyz123",
  "sourceName": "product-docs.md",
  "chunksCreated": 15,
  "embeddingsGenerated": true,
  "totalCharacters": 14500,
  "averageChunkSize": 967,
  "durationMs": 1250
}
```

### Search Knowledge

Use the standard `/recall` endpoint to search knowledge memories:

```bash
curl -X POST http://localhost:8787/api/v1/recall \
  -H "Content-Type: application/json" \
  -H "X-API-Key: mm_dev_key_12345" \
  -H "X-Namespace: default" \
  -d '{
    "query": "How do I configure the API?",
    "type": "knowledge",
    "limit": 5
  }'
```

**Response with Source Citation:**

```json
{
  "success": true,
  "count": 3,
  "embeddingGenerated": true,
  "results": [
    {
      "id": "mem_src_m1abc_xyz123_0",
      "type": "knowledge",
      "content": "## API Configuration\n\nTo configure the API, set the following environment variables...",
      "score": 0.89,
      "vectorSimilarity": 0.89,
      "importance": 0.5,
      "source": {
        "id": "src_m1abc_xyz123",
        "name": "product-docs.md",
        "type": "document",
        "chunkIndex": 3,
        "totalChunks": 15
      }
    }
  ]
}
```

## API Endpoints

### Document Ingestion

#### POST /api/v1/knowledge/ingest

Ingest a document into the knowledge bank.

**Request Body:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| content | string | Yes | Document content to ingest |
| name | string | Yes | Source name (filename, URL, etc.) |
| type | string | No | Source type: `document`, `url`, `api`, `manual` (default: `document`) |
| url | string | No | Original URL (for `url` type) |
| mimeType | string | No | Content type (e.g., `text/markdown`) |
| metadata | object | No | Custom metadata |
| chunking | object | No | Chunking configuration |
| generateEmbeddings | boolean | No | Generate embeddings (default: `true`) |

**Chunking Options:**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| chunkSize | number | 1000 | Target chunk size in characters |
| chunkOverlap | number | 200 | Overlap between chunks |
| separators | string[] | ["\n\n", "\n", ". ", "! ", "? "] | Separators for splitting |
| preserveParagraphs | boolean | true | Prefer paragraph boundaries |

### Source Management

#### GET /api/v1/knowledge/sources

List all knowledge sources.

**Query Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| type | string | Filter by source type |
| limit | number | Max results (default: 100) |
| offset | number | Pagination offset |

**Response:**

```json
{
  "success": true,
  "sources": [
    {
      "id": "src_m1abc_xyz123",
      "name": "product-docs.md",
      "type": "document",
      "mimeType": "text/markdown",
      "size": 14500,
      "chunkCount": 15,
      "createdAt": 1703443200000,
      "updatedAt": 1703443200000
    }
  ],
  "total": 1,
  "hasMore": false
}
```

#### GET /api/v1/knowledge/sources/:id

Get a specific source.

#### DELETE /api/v1/knowledge/sources/:id

Delete a source and all its chunks.

```bash
curl -X DELETE http://localhost:8787/api/v1/knowledge/sources/src_m1abc_xyz123 \
  -H "X-API-Key: mm_dev_key_12345"
```

**Response:**

```json
{
  "success": true,
  "message": "Source \"product-docs.md\" and 15 chunks deleted"
}
```

#### GET /api/v1/knowledge/sources/:id/chunks

Get all chunks for a source.

**Query Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| limit | number | Max results (default: 100) |
| offset | number | Pagination offset |

**Response:**

```json
{
  "success": true,
  "source": {
    "id": "src_m1abc_xyz123",
    "name": "product-docs.md",
    "type": "document"
  },
  "chunks": [
    {
      "id": "mem_src_m1abc_xyz123_0",
      "content": "# Product Documentation\n\nThis is the introduction...",
      "chunkIndex": 0,
      "startOffset": 0,
      "endOffset": 980,
      "createdAt": 1703443200000
    }
  ],
  "total": 15,
  "hasMore": true
}
```

### Statistics

#### GET /api/v1/knowledge/stats

Get knowledge bank statistics.

```bash
curl http://localhost:8787/api/v1/knowledge/stats \
  -H "X-API-Key: mm_dev_key_12345" \
  -H "X-Namespace: default"
```

**Response:**

```json
{
  "success": true,
  "namespace": "default",
  "stats": {
    "totalSources": 5,
    "totalChunks": 125,
    "byType": {
      "document": 3,
      "url": 2,
      "api": 0,
      "manual": 0
    },
    "totalSize": 145000
  }
}
```

### Chunking Preview

#### POST /api/v1/knowledge/chunk-preview

Preview how content will be chunked without storing it.

```bash
curl -X POST http://localhost:8787/api/v1/knowledge/chunk-preview \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Long document content here...",
    "chunking": {
      "chunkSize": 500,
      "chunkOverlap": 100
    }
  }'
```

**Response:**

```json
{
  "success": true,
  "totalChunks": 3,
  "totalCharacters": 1500,
  "averageChunkSize": 500,
  "chunks": [
    {
      "index": 0,
      "length": 500,
      "preview": "Long document content here. This is the first section...",
      "startOffset": 0,
      "endOffset": 500
    }
  ]
}
```

## Source Types

| Type | Description | Use Case |
|------|-------------|----------|
| `document` | Static documents | PDFs, Markdown, text files |
| `url` | Web content | Blog posts, documentation pages |
| `api` | API responses | Dynamic data from external APIs |
| `manual` | Manual entries | User-provided knowledge |

## Memory Type Integration

Knowledge is stored as memories with `type: 'knowledge'`. This means:

- Search with `/recall` using `type: 'knowledge'` filter
- Knowledge appears in `/stats` under `byType.knowledge`
- Standard memory operations work (get, update, delete)

### Filter by Source

```bash
curl -X POST http://localhost:8787/api/v1/recall \
  -H "Content-Type: application/json" \
  -H "X-API-Key: mm_dev_key_12345" \
  -d '{
    "query": "configuration",
    "type": "knowledge",
    "filter": {
      "sourceId": "src_m1abc_xyz123"
    }
  }'
```

## Chunking Strategy

The chunking algorithm:

1. **Target Size**: Splits at `chunkSize` characters
2. **Natural Breaks**: Finds nearest separator before cutoff
3. **Paragraph Preference**: Prioritizes `\n\n` when `preserveParagraphs: true`
4. **Overlap**: Creates overlap for context continuity
5. **Safety Limits**: Respects `maxChunksPerDocument` (default: 1000)

### Recommended Settings

| Content Type | Chunk Size | Overlap | Notes |
|--------------|------------|---------|-------|
| Technical docs | 1000 | 200 | Good for code/API docs |
| Long-form content | 1500 | 300 | Articles, blog posts |
| FAQs | 500 | 100 | Short, focused chunks |
| Legal documents | 800 | 150 | Paragraph-aware |

## Auto Embeddings

When Workers AI is configured, embeddings are generated automatically:

- Uses EmbeddingGemma (768 dimensions by default)
- Batch processes chunks for efficiency
- Falls back gracefully if AI unavailable

**Cost Estimate (EmbeddingGemma):**

| Chunks | Neurons | Cost |
|--------|---------|------|
| 100 | 76,800 | ~$0.85 |
| 1,000 | 768,000 | ~$8.45 |
| 10,000 | 7,680,000 | ~$84.48 |

## Best Practices

### 1. Use Meaningful Names

```json
{
  "name": "api-reference-v2.3.md",
  "type": "document",
  "metadata": { "version": "2.3", "section": "api" }
}
```

### 2. Include Source URLs

```json
{
  "name": "Getting Started Guide",
  "type": "url",
  "url": "https://docs.example.com/getting-started",
  "mimeType": "text/html"
}
```

### 3. Preview Before Ingesting

Test chunking settings with `/chunk-preview` before committing.

### 4. Monitor Chunk Count

Large documents create many chunks. Consider:
- Splitting into logical sections
- Increasing chunk size for overview content
- Using separate sources per major section

### 5. Update Strategy

To update a source:
1. Delete the old source
2. Re-ingest with new content

This ensures consistent chunk IDs and metadata.

## Integration Examples

### RAG Pipeline

```typescript
// 1. Search knowledge
const results = await fetch('/api/v1/recall', {
  method: 'POST',
  headers: { 'X-API-Key': apiKey },
  body: JSON.stringify({
    query: userQuestion,
    type: 'knowledge',
    limit: 5
  })
});

// 2. Build context from chunks
const context = results.results
  .map(r => r.content)
  .join('\n\n---\n\n');

// 3. Build citations
const sources = results.results
  .map(r => `[${r.source.name}](chunk ${r.source.chunkIndex + 1}/${r.source.totalChunks})`)
  .join(', ');

// 4. Send to LLM with context
const answer = await llm.complete({
  prompt: `Context:\n${context}\n\nQuestion: ${userQuestion}`,
  system: `Answer using only the provided context. Cite sources.`
});
```

### Bulk Ingestion

```bash
# Ingest multiple documents
for file in docs/*.md; do
  name=$(basename "$file")
  content=$(cat "$file")

  curl -X POST http://localhost:8787/api/v1/knowledge/ingest \
    -H "X-API-Key: mm_dev_key_12345" \
    -d "{\"name\": \"$name\", \"content\": $(echo "$content" | jq -Rs .)}"
done
```

## Database Schema

```sql
CREATE TABLE knowledge_sources (
  id TEXT PRIMARY KEY,
  namespace TEXT NOT NULL,
  name TEXT NOT NULL,
  type TEXT NOT NULL CHECK (type IN ('document', 'url', 'api', 'manual')),
  url TEXT,
  mime_type TEXT,
  size INTEGER,
  chunk_count INTEGER NOT NULL DEFAULT 0,
  metadata TEXT,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
```

Chunks are stored in the `memories` table with `type = 'knowledge'` and source metadata.

## Cost Estimation

| Scale | Sources | Chunks | D1 Storage | Embeddings | Monthly |
|-------|---------|--------|------------|------------|---------|
| Small | 10 | 150 | ~0.5MB | ~$1.50 | ~$0.01 |
| Medium | 100 | 1,500 | ~5MB | ~$15 | ~$0.10 |
| Large | 1,000 | 15,000 | ~50MB | ~$150 | ~$1.00 |

*Embeddings are one-time cost at ingestion*
