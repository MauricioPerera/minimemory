# minimemory-service API Reference

## Base URL

https://your-worker.workers.dev/api/v1

## Authentication

All requests should include an X-API-Key header for authentication.

## Memory Operations

| Endpoint | Method | Description |
|----------|--------|-------------|
| /remember | POST | Store a memory (auto-generates embedding) |
| /recall | POST | Search for memories (query auto-embeds) |
| /memory/:id | GET | Get specific memory |
| /memory/:id | PATCH | Update memory |
| /forget/:id | DELETE | Delete memory |
| /forget | POST | Delete by filter |
| /clear | DELETE | Clear all |
| /stats | GET | Get statistics |
| /export | POST | Export memories |
| /import | POST | Import memories |

## Embedding Operations

| Endpoint | Method | Description |
|----------|--------|-------------|
| /embed | POST | Generate embeddings |
| /embed/info | GET | Get embedding service info |

### POST /embed

Generate embeddings using EmbeddingGemma (Workers AI).

Request:
- text: string - Single text to embed
- texts: string[] - Multiple texts to embed
- dimensions: 768 | 512 | 256 | 128 - Target dimensions (default: 768)

Response:
- embedding: number[] - Single embedding
- embeddings: number[][] - Batch embeddings
- dimensions: number
- model: string
- truncated: boolean

## Search Modes

| Mode | Required | Description |
|------|----------|-------------|
| vector | embedding OR query | Pure vector similarity |
| keyword | keywords | BM25 keyword search |
| hybrid | query + keywords | RRF fusion |

### POST /remember

If embedding is not provided, it is automatically generated using Workers AI.

Request:
- content: string (required)
- embedding: number[] (optional - auto-generated if missing)
- generateEmbedding: boolean (default: true)
- type: semantic | episodic | working
- importance: number (0-1)
- metadata: object

### POST /recall

If query is provided without embedding, embedding is auto-generated.

Request:
- query: string - Text to search (auto-generates embedding)
- keywords: string - For keyword search
- embedding: number[] - Pre-computed vector
- mode: vector | keyword | hybrid
- limit: number (default: 10)
- alpha: number (0-1, for hybrid balance)

## Filter Operators

- $eq, $ne: Equality
- $gt, $gte, $lt, $lte: Comparison
- $in, $nin: Array membership
- $exists: Field existence
- $contains, $startsWith, $endsWith: String
- $and, $or: Logical

## Audit Log Operations

All memory operations are automatically logged for traceability.

| Endpoint | Method | Description |
|----------|--------|-------------|
| /audit | GET | Query audit logs |
| /audit/:id | GET | Get audit entry by ID |
| /audit/resource/:type/:id | GET | Get resource history |
| /audit/user/:id | GET | Get user activity |
| /audit/failures | GET | Get failed operations |
| /audit/stats | GET | Get audit statistics |
| /audit/cleanup | POST | Clean up old logs |

### GET /audit

Query audit logs with filters.

Query parameters:
- action: create | read | update | delete | search | import | export | clear
- resourceType: memory | namespace | user | tenant | session | api_key
- resourceId: string
- userId: string
- tenantId: string
- namespace: string
- startTime: number (ms timestamp)
- endTime: number (ms timestamp)
- success: boolean
- requestId: string
- limit: number (default: 100)
- offset: number (default: 0)

Response:
```json
{
  "success": true,
  "entries": [
    {
      "id": "aud_xxx",
      "timestamp": 1703443200000,
      "action": "create",
      "resourceType": "memory",
      "resourceId": "mem-123",
      "userId": "user-456",
      "tenantId": "tenant-789",
      "namespace": "default",
      "success": true,
      "durationMs": 45
    }
  ],
  "total": 100,
  "hasMore": true
}
```

### GET /audit/stats

Get audit statistics for a time period.

Query parameters:
- tenantId: string
- startTime: number (ms timestamp)
- endTime: number (ms timestamp)

Response:
```json
{
  "success": true,
  "stats": {
    "totalOperations": 1500,
    "byAction": {
      "create": 500,
      "read": 800,
      "search": 150,
      "delete": 50
    },
    "byResource": {
      "memory": 1400,
      "namespace": 100
    },
    "successRate": 99.5,
    "avgDurationMs": 45
  }
}
```

### POST /audit/cleanup

Clean up old audit logs.

Request:
- retentionDays: number (default: 90)

Response:
```json
{
  "success": true,
  "deletedCount": 1500,
  "message": "Deleted 1500 audit entries older than 90 days"
}
```

## Knowledge Bank Operations

All endpoints use `X-Namespace` header for multi-tenancy.

| Endpoint | Method | Description |
|----------|--------|-------------|
| /knowledge/ingest | POST | Ingest document into knowledge bank |
| /knowledge/sources | GET | List knowledge sources |
| /knowledge/sources/:id | GET | Get source details |
| /knowledge/sources/:id | DELETE | Delete source and chunks |
| /knowledge/sources/:id/chunks | GET | Get source chunks |
| /knowledge/stats | GET | Get knowledge bank statistics |
| /knowledge/chunk-preview | POST | Preview document chunking |

### POST /knowledge/ingest

Ingest a document into the knowledge bank with automatic chunking.

Request:
- content: string - Document content (required)
- name: string - Source name (required)
- type: 'document' | 'url' | 'api' | 'manual' - Source type (default: 'document')
- url: string - Original URL (optional)
- mimeType: string - Content type (optional)
- metadata: object - Custom metadata (optional)
- chunking: object - Chunking options (optional)
  - chunkSize: number (default: 1000)
  - chunkOverlap: number (default: 200)
  - separators: string[]
  - preserveParagraphs: boolean (default: true)
- generateEmbeddings: boolean (default: true)

Response:
```json
{
  "success": true,
  "sourceId": "src_xxx",
  "sourceName": "doc.pdf",
  "chunksCreated": 15,
  "embeddingsGenerated": true,
  "totalCharacters": 14500,
  "averageChunkSize": 967
}
```

### GET /knowledge/sources

List knowledge sources with optional filtering.

Query parameters:
- type: string - Filter by source type
- limit: number (default: 100)
- offset: number (default: 0)

### DELETE /knowledge/sources/:id

Delete a source and all its chunks.

Response:
```json
{
  "success": true,
  "message": "Source \"doc.pdf\" and 15 chunks deleted"
}
```

### GET /knowledge/stats

Get knowledge bank statistics.

Response:
```json
{
  "success": true,
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

## Memory Types

| Type | Description |
|------|-------------|
| `episodic` | Specific events/experiences |
| `semantic` | Facts and knowledge |
| `working` | Temporary context (with TTL) |
| `knowledge` | RAG knowledge bank chunks |

See README.md for full documentation.
