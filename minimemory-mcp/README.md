# minimemory-mcp

MCP (Model Context Protocol) Server for minimemory - enables AI agents to read and write memories with granular access control.

## Overview

This MCP server provides 6 tools that allow AI agents (like Claude) to interact with minimemory:

| Tool | Description |
|------|-------------|
| `remember` | Store a new memory |
| `recall` | Search for memories by query |
| `get` | Get a specific memory by ID |
| `forget` | Delete a memory |
| `stats` | Get namespace statistics |
| `ingest` | Ingest a document into the knowledge bank |

## Architecture

```
┌─────────────────┐    SSE/MCP    ┌──────────────────┐    HTTP    ┌─────────────────┐
│  AI Agent       │──────────────▶│  MCP Server      │───────────▶│  minimemory     │
│  (Claude, etc.) │◀──────────────│  (Workers + DO)  │◀───────────│  Service        │
└─────────────────┘               └──────────────────┘            └─────────────────┘

Authentication Flow:
1. User creates an Agent Token in the minimemory dashboard
2. Agent connects with: /sse?api_key=xxx&agent_token=yyy
3. MCP validates credentials and applies permission filtering
```

## Authentication

This MCP server uses **dual authentication**:

| Parameter | Description |
|-----------|-------------|
| `api_key` | Your minimemory API key - identifies who you are |
| `agent_token` | Agent token - defines what the agent can access |

### Agent Tokens

Agent tokens are created via the minimemory dashboard or API (`/api/v1/agent-tokens`). Each token defines:

- **Name**: Human-readable identifier (e.g., "Work Assistant")
- **Allowed Memories**: List of memory IDs the agent can access, or `["*"]` for all
- **Permissions**: `["read"]`, `["write"]`, or `["read", "write"]`
- **Expiration**: Optional expiration timestamp

### Permission Behavior

| Tool | Required Permission |
|------|-------------------|
| `remember` | `write` |
| `recall` | `read` (results filtered to allowed memories) |
| `get` | `read` + memory must be in allowed list |
| `forget` | `write` + memory must be in allowed list |
| `stats` | `read` |
| `ingest` | `write` |

## Deployment

### 1. Install Dependencies

```bash
cd minimemory-mcp
npm install
```

### 2. Configure Environment

Edit `wrangler.toml` to set your minimemory service URL:

```toml
[vars]
MINIMEMORY_URL = "https://your-minimemory-service.workers.dev"
```

### 3. Deploy

```bash
npm run deploy
```

## Usage with Claude Desktop

Add to your Claude Desktop config file:

**macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
**Windows**: `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "work-assistant": {
      "url": "https://minimemory-mcp.your-subdomain.workers.dev/sse?api_key=mm_xxx&agent_token=at_work123",
      "transport": "sse"
    },
    "research-bot": {
      "url": "https://minimemory-mcp.your-subdomain.workers.dev/sse?api_key=mm_xxx&agent_token=at_research456",
      "transport": "sse"
    }
  }
}
```

You can configure multiple agents, each with different permissions:

- **work-assistant**: Full read/write access to work-related memories
- **research-bot**: Read-only access to research memories

Restart Claude Desktop to apply changes.

## Creating Agent Tokens

### Via API

```bash
# Login and get access token
ACCESS_TOKEN=$(curl -s -X POST https://your-minimemory.workers.dev/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"you@example.com","password":"yourpass"}' | jq -r '.accessToken')

# Create agent token
curl -X POST https://your-minimemory.workers.dev/api/v1/agent-tokens \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Work Assistant",
    "description": "Agent for work-related memories",
    "permissions": ["read", "write"],
    "allowedMemories": ["*"]
  }'
```

### Via Dashboard

1. Log into the minimemory dashboard
2. Navigate to "Agent Tokens"
3. Click "Create Token"
4. Configure name, permissions, and allowed memories
5. Copy the token ID

## Tools

### remember

Store a new memory in the memory bank.

**Parameters:**
- `content` (required): The memory content to store
- `type` (optional): Memory type - `episodic`, `semantic`, or `working` (default: `semantic`)
- `importance` (optional): Importance score from 0 to 1 (default: 0.5)
- `metadata` (optional): Additional key-value metadata

**Example:**
```
Remember that the user prefers dark mode with high importance
```

### recall

Search for memories similar to a query.

**Parameters:**
- `query` (required): Search query
- `type` (optional): Filter by memory type
- `limit` (optional): Maximum results (default: 10, max: 50)
- `threshold` (optional): Minimum similarity (default: 0.7)
- `mode` (optional): Search mode - `vector`, `keyword`, or `hybrid` (default: `hybrid`)

**Example:**
```
Recall any memories about user preferences
```

### get

Get a specific memory by its ID.

**Parameters:**
- `id` (required): Memory ID

### forget

Delete a memory by its ID.

**Parameters:**
- `id` (required): Memory ID to delete

### stats

Get statistics about the memory bank.

**No parameters.**

### ingest

Ingest a document into the knowledge bank for RAG.

**Parameters:**
- `content` (required): Document content
- `name` (required): Document name or title
- `type` (optional): Document type - `document`, `webpage`, `code`, or `note`
- `chunking` (optional): Chunking options
  - `strategy`: `fixed`, `semantic`, or `paragraph`
  - `maxChunkSize`: Maximum chunk size in characters
  - `overlap`: Overlap between chunks

## Security

- Agent tokens can be activated/deactivated in real-time
- Tokens can have expiration dates
- Each token's usage is tracked (last used, use count)
- Permissions are checked on every operation
- Memory access is filtered based on the token's allowed list

## Development

### Local Development

```bash
npm run dev
```

This starts the worker locally. You can test the MCP endpoint at `http://localhost:8787/sse?api_key=xxx&agent_token=yyy`.

### Type Checking

```bash
npm run typecheck
```

## License

MIT
