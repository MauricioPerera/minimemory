# Plan de Implementación: VectorPass MCP Server

## Resumen

Crear un servidor MCP (Model Context Protocol) para VectorPass que actúe como su propio OAuth provider, reutilizando el sistema de autenticación existente (email + código de verificación).

---

## Arquitectura

```
┌─────────────────────┐     ┌─────────────────────────────────────────┐
│  Cliente MCP        │     │  VectorPass MCP Server                  │
│  (Claude Desktop,   │     │  (Cloudflare Worker)                    │
│   Cursor, etc.)     │     │                                         │
└──────────┬──────────┘     │  ┌─────────────────────────────────┐   │
           │                │  │  OAuth Endpoints                 │   │
           │ 1. Connect     │  │  - GET  /authorize (login page)  │   │
           ├───────────────>│  │  - POST /authorize (submit code) │   │
           │                │  │  - POST /token (exchange code)   │   │
           │ 2. Login       │  │  - GET  /callback                │   │
           │<───────────────│  └─────────────────────────────────┘   │
           │                │                                         │
           │ 3. Token       │  ┌─────────────────────────────────┐   │
           │<───────────────│  │  MCP Endpoints                   │   │
           │                │  │  - POST /mcp (Streamable HTTP)   │   │
           │ 4. Use Tools   │  │  - GET  /sse (SSE fallback)      │   │
           ├───────────────>│  └─────────────────────────────────┘   │
           │                │                                         │
           │ 5. Results     │  ┌─────────────────────────────────┐   │
           │<───────────────│  │  VectorPass API (interno)        │   │
           │                │  │  - Usa API existente con apiKey  │   │
           │                │  └─────────────────────────────────┘   │
           │                │                                         │
           │                └─────────────────────────────────────────┘
           │                              │
           │                              ▼
           │                ┌─────────────────────────┐
           │                │  KV Namespaces          │
           │                │  - OAUTH_CODES          │
           │                │  - OAUTH_TOKENS         │
           │                │  - USERS (existente)    │
           │                └─────────────────────────┘
```

---

## Flujo de Autenticación OAuth

### Paso 1: Cliente inicia conexión
```
GET /authorize?
  response_type=code&
  client_id=mcp_client&
  redirect_uri=http://localhost:3000/callback&
  state=random_state&
  code_challenge=sha256_hash&
  code_challenge_method=S256
```

### Paso 2: Usuario ve página de login
- Muestra formulario para ingresar email
- Usuario ingresa email registrado en VectorPass
- Sistema envía código de 6 dígitos (reutiliza `sendVerificationEmail`)

### Paso 3: Usuario ingresa código
- Formulario para código de verificación
- Se valida contra KV (reutiliza `verifyCode`)
- Si es válido, genera authorization code

### Paso 4: Redirect con code
```
HTTP 302 Location: {redirect_uri}?code=auth_code_xxx&state=random_state
```

### Paso 5: Cliente intercambia code por token
```
POST /token
Content-Type: application/x-www-form-urlencoded

grant_type=authorization_code&
code=auth_code_xxx&
redirect_uri=http://localhost:3000/callback&
code_verifier=original_verifier
```

### Paso 6: Servidor retorna access token
```json
{
  "access_token": "vp_mcp_xxxxx",
  "token_type": "Bearer",
  "expires_in": 86400,
  "refresh_token": "vp_refresh_xxxxx"
}
```

El `access_token` internamente mapea al `apiKey` del usuario en VectorPass.

---

## Estructura de Archivos

```
vectorpass-mcp/
├── src/
│   ├── index.ts              # Worker entry point, router
│   ├── oauth/
│   │   ├── authorize.ts      # GET/POST /authorize - login flow
│   │   ├── token.ts          # POST /token - exchange code
│   │   ├── types.ts          # OAuth types
│   │   └── pkce.ts           # PKCE validation helpers
│   ├── mcp/
│   │   ├── server.ts         # MCP server implementation
│   │   ├── tools.ts          # Tool definitions
│   │   ├── transport.ts      # Streamable HTTP + SSE
│   │   └── types.ts          # MCP types
│   ├── auth/
│   │   ├── middleware.ts     # Token validation middleware
│   │   └── session.ts        # Session management
│   ├── pages/
│   │   ├── login.ts          # Login page HTML
│   │   └── verify.ts         # Verification page HTML
│   ├── vectorpass-client.ts  # Cliente para API de VectorPass
│   └── types.ts              # Shared types
├── wrangler.toml
├── package.json
└── tsconfig.json
```

---

## MCP Tools a Exponer

### 1. `vectorpass_index`
Indexar un documento en la base de datos vectorial.

```typescript
{
  name: "vectorpass_index",
  description: "Index a document in VectorPass vector database",
  inputSchema: {
    type: "object",
    properties: {
      id: { type: "string", description: "Unique document ID" },
      text: { type: "string", description: "Text content to index" },
      metadata: { type: "object", description: "Optional metadata" },
      database: { type: "string", description: "Database name (default: 'default')" }
    },
    required: ["id", "text"]
  }
}
```

### 2. `vectorpass_search`
Búsqueda semántica.

```typescript
{
  name: "vectorpass_search",
  description: "Semantic search in VectorPass",
  inputSchema: {
    type: "object",
    properties: {
      query: { type: "string", description: "Search query" },
      k: { type: "number", description: "Number of results (default: 10)" },
      database: { type: "string", description: "Database name" },
      filter: { type: "object", description: "Metadata filter" }
    },
    required: ["query"]
  }
}
```

### 3. `vectorpass_keyword_search`
Búsqueda por palabras clave (BM25).

```typescript
{
  name: "vectorpass_keyword_search",
  description: "Keyword search using BM25 algorithm",
  inputSchema: {
    type: "object",
    properties: {
      query: { type: "string" },
      k: { type: "number" },
      database: { type: "string" }
    },
    required: ["query"]
  }
}
```

### 4. `vectorpass_delete`
Eliminar documento.

```typescript
{
  name: "vectorpass_delete",
  description: "Delete a document from VectorPass",
  inputSchema: {
    type: "object",
    properties: {
      id: { type: "string", description: "Document ID to delete" },
      database: { type: "string" }
    },
    required: ["id"]
  }
}
```

### 5. `vectorpass_list_databases`
Listar bases de datos del usuario.

```typescript
{
  name: "vectorpass_list_databases",
  description: "List all databases for the authenticated user",
  inputSchema: {
    type: "object",
    properties: {}
  }
}
```

### 6. `vectorpass_create_database`
Crear nueva base de datos.

```typescript
{
  name: "vectorpass_create_database",
  description: "Create a new vector database",
  inputSchema: {
    type: "object",
    properties: {
      name: { type: "string", description: "Database name" }
    },
    required: ["name"]
  }
}
```

### 7. `vectorpass_stats`
Obtener estadísticas de uso.

```typescript
{
  name: "vectorpass_stats",
  description: "Get usage statistics for VectorPass account",
  inputSchema: {
    type: "object",
    properties: {
      database: { type: "string" }
    }
  }
}
```

---

## KV Namespaces

### Nuevos (para OAuth):

**OAUTH_CODES** - Authorization codes temporales (TTL: 10 min)
```
code:{code} → {
  userId: string,
  clientId: string,
  redirectUri: string,
  codeChallenge: string,
  codeChallengeMethod: string,
  expiresAt: number
}
```

**OAUTH_TOKENS** - Access tokens (TTL: 24h) y Refresh tokens (TTL: 30 días)
```
access:{token} → {
  userId: string,
  apiKey: string,  // API key de VectorPass del usuario
  expiresAt: number
}

refresh:{token} → {
  userId: string,
  expiresAt: number
}
```

**OAUTH_SESSIONS** - Sesiones de login en progreso (TTL: 15 min)
```
session:{sessionId} → {
  email: string,
  clientId: string,
  redirectUri: string,
  state: string,
  codeChallenge: string,
  verified: boolean
}
```

### Existentes (reutilizar):
- **USERS** - Ya contiene usuarios con apiKey
- **RATE_LIMITS** - Para rate limiting

---

## Configuración wrangler.toml

```toml
name = "vectorpass-mcp"
main = "src/index.ts"
compatibility_date = "2024-01-01"

[vars]
VECTORPASS_API_URL = "https://vectorpass-api.automators.work"
MCP_SERVER_NAME = "vectorpass"
MCP_SERVER_VERSION = "1.0.0"

[[kv_namespaces]]
binding = "OAUTH_CODES"
id = "xxx"

[[kv_namespaces]]
binding = "OAUTH_TOKENS"
id = "xxx"

[[kv_namespaces]]
binding = "OAUTH_SESSIONS"
id = "xxx"

[[kv_namespaces]]
binding = "USERS"
id = "existing_users_kv_id"

[routes]
pattern = "mcp.vectorpass.com/*"
```

---

## Dependencias

```json
{
  "dependencies": {
    "@modelcontextprotocol/sdk": "^1.0.0",
    "hono": "^4.0.0"
  },
  "devDependencies": {
    "@cloudflare/workers-types": "^4.0.0",
    "typescript": "^5.0.0",
    "wrangler": "^3.0.0"
  }
}
```

---

## Pasos de Implementación

### Fase 1: Setup y OAuth (Día 1)
1. Crear estructura de proyecto `vectorpass-mcp/`
2. Configurar wrangler.toml con KV namespaces
3. Implementar `/authorize` - página de login con email
4. Implementar envío de código (reutilizar lógica de email.ts)
5. Implementar verificación de código
6. Implementar `/token` - intercambio de code por access token
7. Implementar PKCE validation

### Fase 2: MCP Server (Día 2)
1. Implementar servidor MCP con Streamable HTTP transport
2. Implementar SSE como fallback
3. Crear cliente interno para VectorPass API
4. Implementar middleware de autenticación (validar access token)

### Fase 3: Tools (Día 2-3)
1. Implementar `vectorpass_index`
2. Implementar `vectorpass_search`
3. Implementar `vectorpass_keyword_search`
4. Implementar `vectorpass_delete`
5. Implementar `vectorpass_list_databases`
6. Implementar `vectorpass_create_database`
7. Implementar `vectorpass_stats`

### Fase 4: Testing y Deploy (Día 3)
1. Test con MCP Inspector
2. Test con Claude Desktop
3. Deploy a producción
4. Configurar dominio mcp.vectorpass.com
5. Actualizar documentación

---

## Endpoints Finales

| Método | Path | Descripción |
|--------|------|-------------|
| GET | `/authorize` | Muestra página de login |
| POST | `/authorize` | Procesa email/código |
| POST | `/token` | Intercambia code por token |
| GET | `/.well-known/oauth-authorization-server` | OAuth metadata |
| POST | `/mcp` | MCP Streamable HTTP |
| GET | `/sse` | MCP SSE transport (fallback) |
| GET | `/health` | Health check |

---

## Consideraciones de Seguridad

1. **PKCE obligatorio** - Previene authorization code interception
2. **Tokens con TTL corto** - Access token 24h, refresh 30 días
3. **Rate limiting** - Reutilizar sistema existente
4. **CORS restrictivo** - Solo orígenes de clientes MCP conocidos
5. **Validación de redirect_uri** - Whitelist de URIs permitidas
