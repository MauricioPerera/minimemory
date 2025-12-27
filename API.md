# MiniMemory API Reference

> A persistent memory system for AI agents with vector, keyword, and hybrid search.

**Base URL:** `https://minimemory-service.rckflr.workers.dev`

---

## Authentication

The API supports two authentication methods:

### API Key
```
X-API-Key: mm_dev_key_12345
```

### JWT Token
```
Authorization: Bearer eyJhbGc...
```

### Namespace Header
Most operations require specifying a namespace:
```
X-Namespace: my-agent-namespace
```
If not provided, uses the `default` namespace.

### Agent Tokens (MCP)
For AI agents connecting via MCP, use dual authentication:
```
/sse?api_key=mm_xxx&agent_token=at_yyy
```
Agent tokens define what memories an agent can access and what permissions it has.

---

## Memory Types

| Type | Description | Use Case |
|------|-------------|----------|
| **episodic** | Events and experiences with temporal context | "User asked about pricing at 10:30 AM" |
| **semantic** | Facts and knowledge | "User prefers short answers" |
| **working** | Temporary task-related state (supports TTL) | "Currently searching for product XYZ" |

---

## Endpoints

### Memory Operations

#### POST /api/v1/remember
Create a new memory.

**Request:**
```json
{
  "content": "The user prefers dark mode",
  "type": "semantic",
  "importance": 0.8,
  "metadata": {
    "source": "settings",
    "tags": ["preference"]
  }
}
```

**Parameters:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| content | string | Yes | The memory content text |
| type | string | No | Memory type: episodic, semantic, or working |
| importance | number | No | Importance score 0-1 (default: 0.5) |
| metadata | object | No | Additional metadata as JSON |
| embedding | number[] | No | Pre-computed embedding vector |
| sessionId | string | No | Session ID for working memory |
| ttl | number | No | Time-to-live in ms for working memory |

**Response:**
```json
{
  "success": true,
  "id": "mem_abc123",
  "persisted": true
}
```

---

#### POST /api/v1/recall
Search for memories using vector, keyword, or hybrid search.

**Request:**
```json
{
  "keywords": "user preferences",
  "mode": "hybrid",
  "limit": 10,
  "type": "semantic",
  "minImportance": 0.5
}
```

**Parameters:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| keywords | string | No* | Search keywords for BM25 |
| embedding | number[] | No* | Query embedding for vector search |
| mode | string | No | Search mode: vector, keyword, or hybrid |
| limit | number | No | Max results (default: 10) |
| type | string | No | Filter by memory type |
| minImportance | number | No | Minimum importance threshold |
| sessionId | string | No | Filter by session ID |

*At least one of `keywords` or `embedding` is required.

**Response:**
```json
{
  "success": true,
  "results": [
    {
      "id": "mem_abc123",
      "content": "The user prefers dark mode",
      "type": "semantic",
      "importance": 0.8,
      "score": 0.95,
      "metadata": { "source": "settings" },
      "createdAt": 1700000000000
    }
  ],
  "count": 1,
  "mode": "hybrid"
}
```

---

#### GET /api/v1/memory/:id
Get a specific memory by ID.

**Response:**
```json
{
  "success": true,
  "memory": {
    "id": "mem_abc123",
    "content": "The user prefers dark mode",
    "type": "semantic",
    "importance": 0.8,
    "metadata": { "source": "settings" },
    "createdAt": 1700000000000,
    "updatedAt": 1700000000000,
    "accessCount": 5
  }
}
```

---

#### PATCH /api/v1/memory/:id
Update a memory.

**Request:**
```json
{
  "content": "Updated content",
  "importance": 0.9,
  "metadata": { "updated": true }
}
```

---

#### DELETE /api/v1/forget/:id
Delete a specific memory.

**Response:**
```json
{
  "success": true,
  "deleted": true
}
```

---

#### POST /api/v1/forget
Delete memories by filter.

**Request:**
```json
{
  "type": "working",
  "sessionId": "session_123",
  "olderThan": 1700000000000
}
```

---

### Namespace Management

#### GET /api/v1/namespaces
List all namespaces.

**Response:**
```json
{
  "success": true,
  "namespaces": [
    { "name": "default", "dimensions": 1536 },
    { "name": "agent-1", "dimensions": 1536 }
  ],
  "count": 2
}
```

---

#### POST /api/v1/namespaces
Create a new namespace.

**Request:**
```json
{
  "name": "my-agent",
  "dimensions": 1536
}
```

---

#### DELETE /api/v1/namespaces/:name
Delete a namespace and all its memories.

---

### Statistics & Maintenance

#### GET /api/v1/stats
Get memory statistics for a namespace.

**Response:**
```json
{
  "success": true,
  "stats": {
    "total": 150,
    "byType": { "episodic": 50, "semantic": 80, "working": 20 },
    "avgImportance": 0.65,
    "oldest": 1699000000000,
    "newest": 1700000000000
  }
}
```

---

#### POST /api/v1/cleanup
Clean up expired working memories.

---

#### POST /api/v1/decay
Apply importance decay to memories.

**Request:**
```json
{
  "rate": 0.1,
  "minImportance": 0.1
}
```

---

#### DELETE /api/v1/clear
Clear all memories in a namespace.

---

### Import/Export

#### POST /api/v1/export
Export all memories from a namespace.

**Response:**
```json
{
  "success": true,
  "memories": [...],
  "count": 150,
  "exportedAt": 1700000000000
}
```

---

#### POST /api/v1/import
Import memories into a namespace.

**Request:**
```json
{
  "memories": [...],
  "overwrite": false
}
```

---

### Agent Tokens (MCP Access Control)

Agent tokens provide granular access control for AI agents connecting via MCP. Each token defines:
- **Allowed Memories**: Which memory IDs the agent can access (`["*"]` for all)
- **Permissions**: `read`, `write`, or both
- **Expiration**: Optional expiration timestamp

---

#### GET /api/v1/agent-tokens
List all agent tokens for the authenticated user.

**Headers:** `Authorization: Bearer <jwt>`

**Query Parameters:**
| Name | Type | Description |
|------|------|-------------|
| active | boolean | Filter by active status |
| limit | number | Max results (default: 100) |
| offset | number | Pagination offset |

**Response:**
```json
{
  "tokens": [
    {
      "id": "at_mjlmoemw_ojitblhk",
      "userId": "user_123",
      "name": "Work Assistant",
      "description": "Agent for work memories",
      "allowedMemories": ["*"],
      "permissions": ["read", "write"],
      "isActive": true,
      "useCount": 42,
      "lastUsedAt": 1700000000000,
      "createdAt": 1699000000000,
      "updatedAt": 1700000000000
    }
  ],
  "total": 1,
  "hasMore": false
}
```

---

#### POST /api/v1/agent-tokens
Create a new agent token.

**Headers:** `Authorization: Bearer <jwt>`

**Request:**
```json
{
  "name": "Work Assistant",
  "description": "Agent for work-related memories",
  "permissions": ["read", "write"],
  "allowedMemories": ["*"],
  "expiresAt": 1735689600000
}
```

**Parameters:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| name | string | Yes | Token name (max 100 chars) |
| description | string | No | Token description |
| permissions | string[] | No | `["read"]`, `["write"]`, or both (default: both) |
| allowedMemories | string[] | No | Memory IDs or `["*"]` for all (default: `["*"]`) |
| expiresAt | number | No | Expiration timestamp in ms |

**Response:**
```json
{
  "token": {
    "id": "at_mjlmoemw_ojitblhk",
    "userId": "user_123",
    "name": "Work Assistant",
    "allowedMemories": ["*"],
    "permissions": ["read", "write"],
    "isActive": true,
    "useCount": 0,
    "createdAt": 1700000000000,
    "updatedAt": 1700000000000
  },
  "message": "Agent token created. Use this ID with your API key to authenticate MCP connections."
}
```

---

#### GET /api/v1/agent-tokens/:id
Get a specific agent token.

**Headers:** `Authorization: Bearer <jwt>`

---

#### PATCH /api/v1/agent-tokens/:id
Update an agent token.

**Headers:** `Authorization: Bearer <jwt>`

**Request:**
```json
{
  "name": "Updated Name",
  "permissions": ["read"],
  "allowedMemories": ["mem_123", "mem_456"],
  "isActive": false
}
```

---

#### DELETE /api/v1/agent-tokens/:id
Delete an agent token.

**Headers:** `Authorization: Bearer <jwt>`

**Response:**
```json
{
  "success": true
}
```

---

#### POST /api/v1/agent-tokens/:id/toggle
Toggle token active status.

**Headers:** `Authorization: Bearer <jwt>`

**Response:**
```json
{
  "token": { ... },
  "message": "Agent token deactivated"
}
```

---

#### POST /api/v1/agent-tokens/:id/add-memory
Add a memory ID to the allowed list.

**Headers:** `Authorization: Bearer <jwt>`

**Request:**
```json
{
  "memoryId": "mem_abc123"
}
```

---

#### POST /api/v1/agent-tokens/:id/remove-memory
Remove a memory ID from the allowed list.

**Headers:** `Authorization: Bearer <jwt>`

**Request:**
```json
{
  "memoryId": "mem_abc123"
}
```

---

#### GET /api/v1/agent-tokens/stats
Get usage statistics for the user's tokens.

**Headers:** `Authorization: Bearer <jwt>`

**Response:**
```json
{
  "stats": {
    "total": 3,
    "active": 2,
    "inactive": 1,
    "expired": 0,
    "totalUseCount": 150
  }
}
```

---

#### POST /api/v1/auth/validate-agent
Validate API key + agent token combination. Used by MCP servers.

**Request:**
```json
{
  "apiKey": "mm_dev_key_12345",
  "agentToken": "at_mjlmoemw_ojitblhk"
}
```

**Response (success):**
```json
{
  "valid": true,
  "userId": "user_123",
  "tenantId": "tenant_456",
  "agentTokenId": "at_mjlmoemw_ojitblhk",
  "agentName": "Work Assistant",
  "allowedMemories": ["*"],
  "permissions": ["read", "write"],
  "expiresAt": null
}
```

**Response (failure):**
```json
{
  "valid": false,
  "error": "Agent token is inactive"
}
```

Possible errors:
- `Invalid API key`
- `Invalid agent token`
- `Agent token is inactive`
- `Agent token has expired`
- `Agent token does not belong to this user`

---

## Search Modes

### Vector Search
Uses cosine similarity to find semantically similar memories.
```json
{
  "embedding": [0.1, 0.2, ...],
  "mode": "vector"
}
```

### Keyword Search (BM25)
Full-text search using the BM25 algorithm.
```json
{
  "keywords": "user preferences",
  "mode": "keyword"
}
```

### Hybrid Search
Combines vector and keyword search using Reciprocal Rank Fusion (RRF).
```json
{
  "keywords": "user preferences",
  "embedding": [0.1, 0.2, ...],
  "mode": "hybrid",
  "alpha": 0.7
}
```
The `alpha` parameter controls the balance: 1.0 = all vector, 0.0 = all keyword.

---

## Error Handling

All errors follow this format:
```json
{
  "error": "Error message description"
}
```

Common HTTP status codes:
- `400` - Bad Request (invalid parameters)
- `401` - Unauthorized (missing or invalid auth)
- `404` - Not Found (memory/namespace doesn't exist)
- `429` - Too Many Requests (rate limit exceeded)
- `500` - Internal Server Error

---

## Rate Limits

Default limits per API key:
- 100 requests per minute
- 1000 requests per hour

---

## Examples

### cURL - Create Memory
```bash
curl -X POST "https://minimemory-service.rckflr.workers.dev/api/v1/remember" \
  -H "Content-Type: application/json" \
  -H "X-API-Key: your-api-key" \
  -H "X-Namespace: default" \
  -d '{
    "content": "User prefers dark mode",
    "type": "semantic",
    "importance": 0.8
  }'
```

### cURL - Search Memories
```bash
curl -X POST "https://minimemory-service.rckflr.workers.dev/api/v1/recall" \
  -H "Content-Type: application/json" \
  -H "X-API-Key: your-api-key" \
  -H "X-Namespace: default" \
  -d '{
    "keywords": "user preferences",
    "mode": "keyword",
    "limit": 10
  }'
```

### JavaScript
```javascript
const response = await fetch('https://minimemory-service.rckflr.workers.dev/api/v1/remember', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'X-API-Key': 'your-api-key',
    'X-Namespace': 'default'
  },
  body: JSON.stringify({
    content: 'User prefers dark mode',
    type: 'semantic',
    importance: 0.8
  })
});

const data = await response.json();
console.log(data.id); // mem_abc123
```

### Python
```python
import requests

response = requests.post(
    'https://minimemory-service.rckflr.workers.dev/api/v1/remember',
    headers={
        'Content-Type': 'application/json',
        'X-API-Key': 'your-api-key',
        'X-Namespace': 'default'
    },
    json={
        'content': 'User prefers dark mode',
        'type': 'semantic',
        'importance': 0.8
    }
)

data = response.json()
print(data['id'])  # mem_abc123
```

---

## MCP Server

The MCP (Model Context Protocol) server allows AI agents like Claude to interact with minimemory.

### Connection URL
```
https://minimemory-mcp.rckflr.workers.dev/sse?api_key=YOUR_API_KEY&agent_token=YOUR_AGENT_TOKEN
```

### Claude Desktop Configuration

**macOS:** `~/Library/Application Support/Claude/claude_desktop_config.json`
**Windows:** `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "work-memories": {
      "url": "https://minimemory-mcp.rckflr.workers.dev/sse?api_key=mm_xxx&agent_token=at_work123",
      "transport": "sse"
    },
    "research-memories": {
      "url": "https://minimemory-mcp.rckflr.workers.dev/sse?api_key=mm_xxx&agent_token=at_research456",
      "transport": "sse"
    }
  }
}
```

### Available MCP Tools

| Tool | Permission | Description |
|------|------------|-------------|
| `remember` | write | Store a new memory |
| `recall` | read | Search for memories |
| `get` | read | Get a specific memory by ID |
| `forget` | write | Delete a memory |
| `stats` | read | Get memory statistics |
| `ingest` | write | Ingest a document into knowledge bank |

### Permission Enforcement

- **Read-only tokens**: Can only use `recall`, `get`, `stats`
- **Write tokens**: Can use all tools
- **Memory filtering**: Results are filtered to only show allowed memories
- **Access checks**: Operations on specific memory IDs check the allowed list

### Example: Creating Tokens for Different Agents

```bash
# Create a read/write token for a work assistant
curl -X POST https://minimemory-service.rckflr.workers.dev/api/v1/agent-tokens \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Work Assistant",
    "permissions": ["read", "write"],
    "allowedMemories": ["*"]
  }'

# Create a read-only token for a research bot
curl -X POST https://minimemory-service.rckflr.workers.dev/api/v1/agent-tokens \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Research Bot",
    "permissions": ["read"],
    "allowedMemories": ["mem_research_1", "mem_research_2"]
  }'
```

---

## Status

- **Service:** https://minimemory-service.rckflr.workers.dev
- **MCP Server:** https://minimemory-mcp.rckflr.workers.dev
- **Dashboard:** https://minimemory-dashboard.pages.dev
- **Storage:** Cloudflare D1 (SQLite at the edge)
- **Version:** 1.0.0
