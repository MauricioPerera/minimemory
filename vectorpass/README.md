# VectorPass

RAG as a Service - Semantic search for AI agents, powered by minimemory.

## Features

- **Semantic Search** - EmbeddingGemma-300m with Matryoshka truncation (768d -> 256d)
- **Keyword Search** - BM25 full-text search
- **WASM Backend** - minimemory vector database compiled to WebAssembly
- **Int8 Quantization** - 4x memory reduction
- **Rate Limiting** - Per-tier usage limits
- **Email Verification** - Passwordless authentication
- **Stripe Integration** - Subscription management

## Quick Start

### 1. Install dependencies

```bash
cd vectorpass
npm install
```

### 2. Create KV namespaces

```bash
wrangler kv:namespace create USERS
wrangler kv:namespace create VECTORS
wrangler kv:namespace create RATE_LIMITS
```

Update `wrangler.toml` with the namespace IDs from the output.

### 3. Build WASM module (optional)

```bash
# From project root - requires wasm-pack
./vectorpass/build-wasm.sh
```

The worker uses a JavaScript fallback if WASM is not available.

### 4. Run locally

```bash
npm run dev
```

### 5. Deploy to production

```bash
# Set secrets first
wrangler secret put STRIPE_WEBHOOK_SECRET
wrangler secret put STRIPE_SECRET_KEY
wrangler secret put EMAIL_API_KEY

# Deploy
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
  "message": "Verification code sent to your email",
  "data": { "id": "...", "email": "user@example.com", "verified": false }
}
```

### Verify Email

```bash
curl -X POST http://localhost:8787/auth/verify \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com", "code": "123456"}'
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

| Tier | Vectors | Searches/day | Batch Size | Price |
|------|---------|--------------|------------|-------|
| Free | 1,000 | 100 | 10 | $0 |
| Starter | 50,000 | 10,000 | 100 | $9/mo |
| Pro | 500,000 | 100,000 | 500 | $29/mo |
| Business | 5,000,000 | 1,000,000 | 1,000 | $79/mo |

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `EMAIL_SERVICE` | Email provider: `log`, `resend`, `sendgrid`, `mailgun` | `log` |
| `FROM_EMAIL` | Sender email address | `noreply@vectorpass.automators.work` |

### Secrets

Set via `wrangler secret put <NAME>`:

| Secret | Description |
|--------|-------------|
| `STRIPE_WEBHOOK_SECRET` | Stripe webhook signing secret |
| `STRIPE_SECRET_KEY` | Stripe API secret key |
| `EMAIL_API_KEY` | API key for email service |
| `MAILGUN_DOMAIN` | Mailgun domain (if using Mailgun) |

## Architecture

```
vectorpass/
├── src/
│   ├── index.ts      # Main worker, routing
│   ├── types.ts      # TypeScript types, tier limits
│   ├── auth.ts       # API key management
│   ├── email.ts      # Email verification
│   ├── ratelimit.ts  # Rate limiting
│   ├── stripe.ts     # Stripe webhooks
│   └── vectordb.ts   # Vector database wrapper
├── pkg/              # WASM build output (after build)
├── wrangler.toml     # Cloudflare config
└── package.json
```

**Stack:**
- **Cloudflare Workers** - Edge compute
- **Workers AI** - EmbeddingGemma-300m for embeddings
- **KV** - Persistence for users and vectors
- **minimemory** - Vector database (WASM)

## Security

- **Email Verification** - Required for write operations (index, delete)
- **Rate Limiting** - Per-tier daily limits
- **API Key Auth** - `X-API-Key` header or `Authorization: Bearer` token
- **Stripe Signature** - Webhook signature verification

## Stripe Setup

1. Create products/prices in Stripe Dashboard
2. Update price IDs in `src/stripe.ts`
3. Set webhook endpoint: `https://vectorpass.automators.work/webhooks/stripe`
4. Events to listen for:
   - `checkout.session.completed`
   - `customer.subscription.created`
   - `customer.subscription.updated`
   - `customer.subscription.deleted`
   - `invoice.payment_failed`

## Development

```bash
# Run locally with hot reload
npm run dev

# View logs
npm run tail

# Deploy
npm run deploy
```
