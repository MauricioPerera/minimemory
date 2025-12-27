# minimemory - Estado del Proyecto

> Última actualización: Diciembre 2024

## Resumen Ejecutivo

**minimemory** es una plataforma completa de memoria para agentes de IA que consiste en:

| Componente | Tecnología | Estado |
|------------|------------|--------|
| Core Library | Rust | v0.1.0 |
| API Service | TypeScript/Hono | v0.5.0 Deployed |
| Dashboard | React/Vite | v0.0.0 Deployed |
| n8n Nodes | TypeScript | v0.4.0 |
| JavaScript SDK | TypeScript | v0.1.0 Ready |
| Python SDK | Python/httpx | v0.1.0 Ready |
| Webhooks | TypeScript | v0.1.0 Ready |

**URLs de Producción:**
- Dashboard: https://minimemory-dashboard.pages.dev
- API: https://minimemory-service.rckflr.workers.dev

---

## 1. Arquitectura General

```
┌─────────────────────────────────────────────────────────────────┐
│                    MINIMEMORY DASHBOARD                          │
│  React 19 + Vite + Tailwind + React Query                       │
│  Cloudflare Pages                                                │
└──────────────────────────┬──────────────────────────────────────┘
                           │ HTTPS
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                    MINIMEMORY SERVICE                            │
│  Hono Framework + TypeScript                                     │
│  Cloudflare Workers (Edge)                                       │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │   Auth      │  │   Memory    │  │     Tenant              │  │
│  │   Routes    │  │   API       │  │     Management          │  │
│  └──────┬──────┘  └──────┬──────┘  └───────────┬─────────────┘  │
│         │                │                      │                │
│         ▼                ▼                      ▼                │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │              CLOUDFLARE D1 (SQLite Edge)                   │  │
│  │  users │ tenants │ sessions │ namespaces │ memories        │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 2. Estructura del Repositorio

```
minimemory/
├── src/                          # Rust Core Library
│   ├── lib.rs                    # Entry point
│   ├── db.rs                     # VectorDB principal
│   ├── types.rs                  # Tipos de datos
│   ├── error.rs                  # Manejo de errores
│   ├── index/                    # Índices de búsqueda
│   │   ├── flat.rs               # Brute-force O(n)
│   │   └── hnsw.rs               # HNSW O(log n)
│   ├── distance/                 # Métricas de distancia
│   │   ├── mod.rs                # Cosine, Euclidean, Dot
│   │   └── simd.rs               # Optimizaciones SIMD
│   ├── storage/                  # Persistencia
│   │   ├── memory.rs             # En memoria
│   │   ├── disk.rs               # En disco
│   │   └── format.rs             # Formato binario
│   └── bindings/                 # Bindings a otros lenguajes
│       ├── python.rs             # PyO3
│       ├── nodejs.rs             # NAPI
│       └── ffi.rs                # C FFI
│
├── minimemory-service/           # API Serverless
│   ├── src/
│   │   ├── index.ts              # App principal (Hono)
│   │   ├── api/memory.ts         # Endpoints de memoria
│   │   ├── routes/
│   │   │   ├── auth.ts           # Autenticación JWT
│   │   │   └── tenants.ts        # Gestión de tenants
│   │   ├── middleware/
│   │   │   ├── auth.ts           # API Key auth
│   │   │   ├── jwt.ts            # JWT validation
│   │   │   ├── tenant.ts         # Tenant context
│   │   │   └── rateLimit.ts      # Rate limiting
│   │   ├── memory/
│   │   │   ├── MemoryManager.ts  # Orquestador
│   │   │   └── types.ts          # Tipos de memoria
│   │   ├── core/
│   │   │   ├── VectorDB.ts       # Motor vectorial
│   │   │   ├── BM25Index.ts      # Búsqueda keyword
│   │   │   └── HybridSearch.ts   # Búsqueda híbrida
│   │   ├── storage/
│   │   │   └── D1Storage.ts      # Adapter D1
│   │   └── utils/
│   │       ├── password.ts       # PBKDF2 hashing
│   │       └── tokens.ts         # JWT generation
│   ├── schema.sql                # Schema D1
│   └── wrangler.toml             # Config Workers
│
├── minimemory-dashboard/         # React Dashboard
│   ├── src/
│   │   ├── App.tsx               # Routing principal
│   │   ├── pages/
│   │   │   ├── Dashboard.tsx     # Vista principal
│   │   │   ├── Documentation.tsx # Docs API
│   │   │   ├── Login.tsx         # Autenticación
│   │   │   └── Register.tsx      # Registro
│   │   ├── components/
│   │   │   ├── layout/           # Header, Layout
│   │   │   ├── memory/           # MemoryList, SearchBar
│   │   │   ├── stats/            # Charts, Cards
│   │   │   └── auth/             # AuthGuard
│   │   ├── contexts/
│   │   │   └── AuthContext.tsx   # Estado auth
│   │   ├── api/
│   │   │   ├── client.ts         # HTTP client
│   │   │   ├── auth.ts           # Auth API
│   │   │   └── hooks.ts          # React Query
│   │   └── hooks/
│   │       └── useTheme.ts       # Dark mode
│   └── vite.config.ts
│
├── n8n-nodes-minimemory/         # n8n Integration
│   └── nodes/Minimemory/
│       ├── Minimemory.node.ts    # Nodo principal
│       ├── VectorDB.ts           # Motor vectorial
│       └── HybridSearch.ts       # Búsqueda híbrida
│
├── minimemory-sdk/               # JavaScript/TypeScript SDK
│   ├── src/
│   │   ├── index.ts              # Exports públicos
│   │   ├── client.ts             # MiniMemoryClient class
│   │   └── types.ts              # Type definitions
│   ├── tests/
│   │   ├── client.test.ts        # Unit tests (33)
│   │   └── e2e.test.ts           # E2E tests (10)
│   ├── package.json              # @minimemory/sdk
│   └── README.md                 # Documentación completa
│
├── minimemory-py/                # Python SDK
│   ├── minimemory/
│   │   ├── __init__.py           # Exports públicos
│   │   ├── client.py             # MiniMemoryClient async
│   │   ├── types.py              # Pydantic models
│   │   └── exceptions.py         # Exception classes
│   ├── tests/
│   │   ├── test_client.py        # Unit tests (28)
│   │   ├── test_knowledge.py     # Knowledge tests (9)
│   │   ├── test_embed.py         # Embed tests (4)
│   │   └── test_e2e.py           # E2E tests (9)
│   ├── pyproject.toml            # Package config
│   └── README.md                 # Documentación completa
│
├── bindings/                     # Language Bindings
│   ├── nodejs/                   # Node.js NAPI
│   ├── python/                   # Python PyO3
│   └── php/                      # PHP wrapper
│
├── Cargo.toml                    # Rust config
└── README.md
```

---

## 3. Base de Datos (D1 Schema)

### Tablas de Memoria

```sql
-- Espacios de nombres aislados
CREATE TABLE namespaces (
    name TEXT PRIMARY KEY,
    tenant_id TEXT,
    dimensions INTEGER DEFAULT 1536,
    created_at INTEGER,
    updated_at INTEGER
);

-- Memorias con embeddings vectoriales
CREATE TABLE memories (
    id TEXT PRIMARY KEY,
    namespace TEXT NOT NULL,
    type TEXT CHECK (type IN ('episodic', 'semantic', 'working')),
    content TEXT NOT NULL,
    embedding TEXT,           -- JSON array de floats
    importance REAL DEFAULT 0.5,
    metadata TEXT,            -- JSON arbitrario
    session_id TEXT,
    ttl INTEGER,              -- Time-to-live en ms
    created_at INTEGER,
    updated_at INTEGER,
    last_accessed INTEGER,
    access_count INTEGER DEFAULT 0
);
```

### Tablas de Autenticación

```sql
-- Organizaciones/equipos
CREATE TABLE tenants (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    plan TEXT DEFAULT 'free',  -- free, starter, pro, enterprise
    max_memories INTEGER DEFAULT 1000,
    max_namespaces INTEGER DEFAULT 1,
    created_at INTEGER,
    updated_at INTEGER
);

-- Usuarios
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,  -- PBKDF2
    name TEXT,
    is_active INTEGER DEFAULT 1,
    created_at INTEGER,
    last_login INTEGER
);

-- Relación usuario-tenant con roles
CREATE TABLE user_tenants (
    user_id TEXT,
    tenant_id TEXT,
    role TEXT DEFAULT 'member',  -- owner, admin, member, viewer
    created_at INTEGER,
    PRIMARY KEY (user_id, tenant_id)
);

-- Sesiones JWT
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT,
    refresh_token_hash TEXT,
    expires_at INTEGER,
    created_at INTEGER
);

-- API Keys
CREATE TABLE api_keys (
    key TEXT PRIMARY KEY,
    name TEXT,
    user_id TEXT,
    tenant_id TEXT,
    rate_limit INTEGER DEFAULT 100,
    is_active INTEGER DEFAULT 1,
    created_at INTEGER,
    last_used INTEGER
);
```

---

## 4. API Endpoints

### Autenticación (`/api/v1/auth`)

| Método | Endpoint | Descripción |
|--------|----------|-------------|
| POST | `/register` | Crear cuenta + tenant inicial |
| POST | `/login` | Login, retorna JWT access + refresh |
| POST | `/refresh` | Renovar access token |
| POST | `/logout` | Invalidar sesión |
| GET | `/me` | Perfil del usuario actual |

### Memoria (`/api/v1`)

| Método | Endpoint | Descripción |
|--------|----------|-------------|
| POST | `/remember` | Guardar memoria con embedding |
| POST | `/recall` | Buscar memorias (vector/keyword/hybrid) |
| GET | `/memory/:id` | Obtener memoria específica |
| PATCH | `/memory/:id` | Actualizar memoria |
| DELETE | `/forget/:id` | Eliminar memoria |
| POST | `/forget` | Eliminar por filtro |
| GET | `/stats` | Estadísticas de memoria |
| POST | `/cleanup` | Limpiar memorias expiradas |
| POST | `/decay` | Aplicar decay de importancia |
| POST | `/export` | Exportar memorias |
| POST | `/import` | Importar memorias |
| DELETE | `/clear` | Limpiar namespace |

### Tenants (`/api/v1/tenants`)

| Método | Endpoint | Descripción |
|--------|----------|-------------|
| GET | `/` | Listar tenants del usuario |
| POST | `/` | Crear tenant |
| GET | `/:id` | Detalle de tenant |
| PUT | `/:id` | Actualizar tenant |
| DELETE | `/:id` | Eliminar tenant |
| GET | `/:id/members` | Listar miembros |
| POST | `/:id/members` | Invitar miembro |

### Namespaces (`/api/v1/namespaces`)

| Método | Endpoint | Descripción |
|--------|----------|-------------|
| GET | `/` | Listar namespaces |
| POST | `/` | Crear namespace |
| DELETE | `/:name` | Eliminar namespace |

---

## 5. Tipos de Memoria

### Episodic Memory
Eventos y experiencias con contexto temporal.

```typescript
{
  type: 'episodic',
  content: 'Usuario preguntó sobre precios',
  importance: 0.7,
  metadata: {
    timestamp: '2024-12-24T10:30:00Z',
    context: 'conversación de ventas'
  }
}
```

### Semantic Memory
Hechos y conocimiento general.

```typescript
{
  type: 'semantic',
  content: 'Usuario prefiere respuestas cortas',
  importance: 0.9,
  metadata: {
    confidence: 0.85,
    source: 'múltiples interacciones'
  }
}
```

### Working Memory
Estado temporal de la sesión actual.

```typescript
{
  type: 'working',
  content: 'Buscando producto XYZ',
  session_id: 'session-123',
  ttl: 3600000,  // 1 hora
  importance: 0.5
}
```

---

## 6. Modos de Búsqueda

### Vector Search
Búsqueda por similitud semántica usando embeddings.

```bash
POST /api/v1/recall
{
  "embedding": [0.1, 0.2, ...],  # 1536 dimensiones
  "topK": 10,
  "searchMode": "vector"
}
```

### Keyword Search (BM25)
Búsqueda por palabras clave.

```bash
POST /api/v1/recall
{
  "keywords": "precio producto",
  "topK": 10,
  "searchMode": "keyword"
}
```

### Hybrid Search
Combinación de vector + keyword usando RRF.

```bash
POST /api/v1/recall
{
  "embedding": [0.1, 0.2, ...],
  "keywords": "precio producto",
  "topK": 10,
  "searchMode": "hybrid",
  "hybridAlpha": 0.7  # 70% vector, 30% keyword
}
```

---

## 7. Autenticación

### JWT Flow

```
1. POST /auth/register → {accessToken, refreshToken, user, tenants[]}
2. Authorization: Bearer <accessToken>
3. X-Tenant-Id: <tenantId>
4. POST /auth/refresh → {accessToken, refreshToken} (cuando expira)
```

### API Key Flow

```
1. GET /api/v1/namespaces
   X-API-Key: mm_dev_key_12345
```

### Roles

| Rol | Memorias | Namespaces | Miembros | Tenant |
|-----|----------|------------|----------|--------|
| owner | CRUD | CRUD | CRUD | CRUD |
| admin | CRUD | CRUD | CRU | R |
| member | CRUD | R | R | R |
| viewer | R | R | R | R |

---

## 8. Dashboard Features

### Páginas
- **Dashboard** - Vista principal con stats y lista de memorias
- **Documentation** - Documentación interactiva de la API
- **Login/Register** - Autenticación

### Componentes
- **StatsCards** - Total memorias, distribución por tipo
- **TypeChart** - Gráfico circular de tipos
- **ImportanceChart** - Distribución de importancia
- **SearchBar** - Búsqueda keyword/hybrid
- **MemoryList** - Lista paginada de memorias
- **CreateMemoryModal** - Crear nueva memoria
- **MemoryModal** - Ver detalles de memoria

### Features
- Dark mode
- Multi-tenant (selector de tenant)
- Multi-namespace (selector de namespace)
- Búsqueda híbrida
- CRUD de memorias

---

## 9. Deployment

### Service (Cloudflare Workers)

```bash
cd minimemory-service
npm install
npm run build
npm run deploy
```

**Variables de entorno:**
- `JWT_SECRET` - Secret para access tokens
- `JWT_REFRESH_SECRET` - Secret para refresh tokens

### Dashboard (Cloudflare Pages)

```bash
cd minimemory-dashboard
npm install
npm run build
npx wrangler pages deploy dist --project-name=minimemory-dashboard
```

---

## 10. Desarrollo Local

### Service

```bash
cd minimemory-service
npm install
npm run dev  # Puerto 8787
```

### Dashboard

```bash
cd minimemory-dashboard
npm install
npm run dev  # Puerto 5173
```

---

## 11. Roadmap

### Completado
- [x] Core Rust library (HNSW, Flat index)
- [x] API Service con Hono
- [x] Persistencia D1
- [x] Búsqueda vectorial
- [x] Búsqueda keyword (BM25)
- [x] Búsqueda híbrida (RRF)
- [x] Dashboard React
- [x] Autenticación JWT
- [x] Multi-tenant
- [x] Rate limiting
- [x] Documentación API

### Pendiente
- [ ] SIMD optimizations en servicio
- [ ] Graph relationships
- [ ] Memory consolidation automática
- [x] SDK JavaScript (`@minimemory/sdk` v0.1.0 - 43 tests)
- [x] SDK Python (`minimemory` v0.1.0 - 50 tests)
- [x] Webhooks (v0.1.0 - 24 tests, 5 eventos)
- [ ] Analytics dashboard
- [ ] SOC 2 compliance

---

## 12. Licencia

MIT License - Mauricio Perera
