# VectorPass

RAG as a Service - Semantic search for AI agents, powered by minimemory.

## Quick Start

### 1. Install dependencies

```bash
npm install
```

### 2. Create KV namespaces

```bash
wrangler kv:namespace create USERS
wrangler kv:namespace create VECTORS
wrangler kv:namespace create RATE_LIMITS
```

Update `wrangler.toml` with the namespace IDs from the output.

### 3. Run locally

```bash
npm run dev
```

### 4. Deploy

```bash
# Configure subdomain in wrangler.toml first
npm run deploy
```

## API Usage

### Register

```bash
curl -X POST http://localhost:8787/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com"}'
```

Response:
```json
{
  "success": true,
  "data": {
    "apiKey": "vp_live_xxxxx",
    "tier": "free",
    "limits": { "maxVectors": 1000, "searchesPerDay": 100 }
  }
}
```

### Index Document

```bash
curl -X POST http://localhost:8787/v1/index \
  -H "X-API-Key: vp_live_xxxxx" \
  -H "Content-Type: application/json" \
  -d '{"id": "doc1", "text": "Hello world", "metadata": {"category": "greeting"}}'
```

### Batch Index

```bash
curl -X POST http://localhost:8787/v1/batch \
  -H "X-API-Key: vp_live_xxxxx" \
  -H "Content-Type: application/json" \
  -d '{
    "items": [
      {"id": "doc1", "text": "First document"},
      {"id": "doc2", "text": "Second document"}
    ]
  }'
```

### Semantic Search

```bash
curl -X POST http://localhost:8787/v1/search \
  -H "X-API-Key: vp_live_xxxxx" \
  -H "Content-Type: application/json" \
  -d '{"query": "greeting", "k": 5}'
```

### Keyword Search (BM25)

```bash
curl -X POST http://localhost:8787/v1/keyword \
  -H "X-API-Key: vp_live_xxxxx" \
  -H "Content-Type: application/json" \
  -d '{"query": "hello world", "k": 5}'
```

### Delete Document

```bash
curl -X DELETE http://localhost:8787/v1/vectors/doc1 \
  -H "X-API-Key: vp_live_xxxxx"
```

### Get Stats

```bash
curl http://localhost:8787/v1/stats \
  -H "X-API-Key: vp_live_xxxxx"
```

## Pricing Tiers

| Tier | Vectors | Searches/day | Price |
|------|---------|--------------|-------|
| Free | 1,000 | 100 | $0 |
| Starter | 50,000 | 10,000 | $9/mo |
| Pro | 500,000 | 100,000 | $29/mo |
| Business | 5,000,000 | 1,000,000 | $79/mo |

## Architecture

- **Cloudflare Workers** - Edge compute
- **Workers AI** - EmbeddingGemma-300m for embeddings
- **KV** - Persistence for users and vectors
- **minimemory** - Vector database (WASM)

## Next Steps

1. Integrate minimemory WASM build
2. Add Stripe webhook for subscriptions
3. Add email verification
4. Add dashboard UI
