-- minimemory-service D1 Schema
-- Persistent storage for agentic memories

-- Namespaces table
CREATE TABLE IF NOT EXISTS namespaces (
    name TEXT PRIMARY KEY,
    tenant_id TEXT, -- nullable for legacy, linked to tenants table
    dimensions INTEGER NOT NULL DEFAULT 1536,
    created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000)
);

-- Memories table
CREATE TABLE IF NOT EXISTS memories (
    id TEXT PRIMARY KEY,
    namespace TEXT NOT NULL DEFAULT 'default',
    type TEXT NOT NULL CHECK (type IN ('episodic', 'semantic', 'working', 'knowledge')),
    content TEXT NOT NULL,
    embedding TEXT NOT NULL, -- JSON array of floats
    importance REAL NOT NULL DEFAULT 0.5,
    metadata TEXT, -- JSON object
    session_id TEXT,
    ttl INTEGER, -- Time to live in ms (for working memory)
    created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    last_accessed INTEGER,
    access_count INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (namespace) REFERENCES namespaces(name) ON DELETE CASCADE
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_memories_namespace ON memories(namespace);
CREATE INDEX IF NOT EXISTS idx_memories_type ON memories(namespace, type);
CREATE INDEX IF NOT EXISTS idx_memories_importance ON memories(namespace, importance DESC);
CREATE INDEX IF NOT EXISTS idx_memories_created ON memories(namespace, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_memories_session ON memories(namespace, session_id);

-- ============================================
-- MULTI-TENANT AUTHENTICATION TABLES
-- ============================================

-- Tenants (organizations/teams)
CREATE TABLE IF NOT EXISTS tenants (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    plan TEXT NOT NULL DEFAULT 'free', -- free, starter, pro, enterprise
    max_memories INTEGER NOT NULL DEFAULT 1000,
    max_namespaces INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000)
);

-- Users
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    name TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    last_login INTEGER
);

-- User-Tenant relationship with roles
CREATE TABLE IF NOT EXISTS user_tenants (
    user_id TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'member', -- owner, admin, member, viewer
    created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    PRIMARY KEY (user_id, tenant_id),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE
);

-- Sessions (JWT refresh tokens)
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    refresh_token_hash TEXT NOT NULL,
    expires_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Indexes for auth tables
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_user_tenants_user ON user_tenants(user_id);
CREATE INDEX IF NOT EXISTS idx_user_tenants_tenant ON user_tenants(tenant_id);
CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_expires ON sessions(expires_at);

-- API Keys table (for production) - updated with tenant support
CREATE TABLE IF NOT EXISTS api_keys (
    key TEXT PRIMARY KEY,
    user_id TEXT, -- nullable for legacy keys
    tenant_id TEXT, -- nullable for legacy keys
    namespace TEXT,
    permissions TEXT NOT NULL DEFAULT '["read","write"]', -- JSON array
    rate_limit INTEGER NOT NULL DEFAULT 100,
    rate_window INTEGER NOT NULL DEFAULT 60,
    created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    last_used INTEGER,
    is_active INTEGER NOT NULL DEFAULT 1,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_api_keys_user ON api_keys(user_id);
CREATE INDEX IF NOT EXISTS idx_api_keys_tenant ON api_keys(tenant_id);

-- ============================================
-- DEFAULT DATA
-- ============================================

-- Create default tenant
INSERT OR IGNORE INTO tenants (id, name, plan, max_memories, max_namespaces)
VALUES ('default', 'Default Tenant', 'free', 10000, 10);

-- Create default namespace (linked to default tenant)
INSERT OR IGNORE INTO namespaces (name, dimensions)
VALUES ('default', 1536);

-- Default development key (legacy support)
INSERT OR IGNORE INTO api_keys (key, user_id, tenant_id, namespace, permissions, rate_limit, rate_window)
VALUES ('mm_dev_key_12345', NULL, 'default', 'default', '["read","write","admin"]', 1000, 60);

-- ============================================
-- KNOWLEDGE BANK TABLES (RAG)
-- ============================================

-- Knowledge sources (documents, URLs, etc.)
CREATE TABLE IF NOT EXISTS knowledge_sources (
    id TEXT PRIMARY KEY,
    namespace TEXT NOT NULL DEFAULT 'default',
    name TEXT NOT NULL,
    type TEXT NOT NULL CHECK (type IN ('document', 'url', 'api', 'manual')),
    url TEXT,
    mime_type TEXT,
    size INTEGER,                    -- Original content size in bytes
    chunk_count INTEGER NOT NULL DEFAULT 0,
    metadata TEXT,                   -- JSON object
    created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    FOREIGN KEY (namespace) REFERENCES namespaces(name) ON DELETE CASCADE
);

-- Indexes for knowledge sources
CREATE INDEX IF NOT EXISTS idx_knowledge_sources_namespace ON knowledge_sources(namespace);
CREATE INDEX IF NOT EXISTS idx_knowledge_sources_type ON knowledge_sources(namespace, type);
CREATE INDEX IF NOT EXISTS idx_knowledge_sources_created ON knowledge_sources(created_at DESC);

-- ============================================
-- AUDIT LOG TABLE
-- ============================================

CREATE TABLE IF NOT EXISTS audit_log (
    id TEXT PRIMARY KEY,
    timestamp INTEGER NOT NULL DEFAULT (unixepoch() * 1000),

    -- Operation details
    action TEXT NOT NULL CHECK (action IN (
        'create', 'read', 'update', 'delete',
        'search', 'import', 'export', 'clear',
        'login', 'logout', 'register'
    )),
    resource_type TEXT NOT NULL CHECK (resource_type IN (
        'memory', 'namespace', 'user', 'tenant', 'session', 'api_key', 'knowledge_source'
    )),
    resource_id TEXT,

    -- Actor information
    user_id TEXT,
    tenant_id TEXT,
    namespace TEXT,
    api_key_prefix TEXT,

    -- Request context
    ip_address TEXT,
    user_agent TEXT,
    request_id TEXT,

    -- Change details (JSON)
    details TEXT,

    -- Result
    success INTEGER NOT NULL DEFAULT 1,
    error_message TEXT,

    -- Performance
    duration_ms INTEGER,

    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE SET NULL,
    FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE SET NULL
);

-- Audit log indexes
CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_log(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_audit_action ON audit_log(action, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_audit_resource ON audit_log(resource_type, resource_id);
CREATE INDEX IF NOT EXISTS idx_audit_user ON audit_log(user_id, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_audit_tenant ON audit_log(tenant_id, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_audit_namespace ON audit_log(namespace, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_audit_request ON audit_log(request_id);
CREATE INDEX IF NOT EXISTS idx_audit_tenant_action ON audit_log(tenant_id, action, timestamp DESC);

-- ============================================
-- WEBHOOKS TABLE
-- ============================================

-- Webhooks for event notifications
CREATE TABLE IF NOT EXISTS webhooks (
    id TEXT PRIMARY KEY,
    namespace TEXT NOT NULL,
    tenant_id TEXT,
    url TEXT NOT NULL,
    secret TEXT NOT NULL,              -- HMAC secret for signing
    events TEXT NOT NULL,              -- JSON array: ["memory.remembered", ...]
    is_active INTEGER NOT NULL DEFAULT 1,
    description TEXT,
    max_retries INTEGER NOT NULL DEFAULT 3,
    retry_backoff_ms INTEGER NOT NULL DEFAULT 1000,
    success_count INTEGER NOT NULL DEFAULT 0,
    failure_count INTEGER NOT NULL DEFAULT 0,
    last_triggered_at INTEGER,
    created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    FOREIGN KEY (namespace) REFERENCES namespaces(name) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE
);

-- Webhook delivery log (for retry tracking)
CREATE TABLE IF NOT EXISTS webhook_deliveries (
    id TEXT PRIMARY KEY,
    webhook_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    event_id TEXT NOT NULL,
    payload TEXT NOT NULL,             -- JSON payload
    status TEXT NOT NULL CHECK (status IN ('pending', 'success', 'failed', 'retrying')) DEFAULT 'pending',
    attempt_count INTEGER NOT NULL DEFAULT 0,
    next_retry_at INTEGER,
    response_status INTEGER,
    response_body TEXT,
    error_message TEXT,
    created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    completed_at INTEGER,
    FOREIGN KEY (webhook_id) REFERENCES webhooks(id) ON DELETE CASCADE
);

-- Webhook indexes
CREATE INDEX IF NOT EXISTS idx_webhooks_namespace ON webhooks(namespace);
CREATE INDEX IF NOT EXISTS idx_webhooks_tenant ON webhooks(tenant_id);
CREATE INDEX IF NOT EXISTS idx_webhooks_active ON webhooks(namespace, is_active);
CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_webhook ON webhook_deliveries(webhook_id);
CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_status ON webhook_deliveries(status, next_retry_at);
CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_created ON webhook_deliveries(created_at DESC);

-- ============================================
-- AGENT TOKENS TABLE (MCP Access Control)
-- ============================================

-- Agent tokens for MCP server access control
-- Users create tokens that define what memories an agent can access
CREATE TABLE IF NOT EXISTS agent_tokens (
    id TEXT PRIMARY KEY,                    -- at_xxxxx format
    user_id TEXT NOT NULL,
    tenant_id TEXT,
    name TEXT NOT NULL,                     -- Descriptive name (e.g., "Work Assistant")
    description TEXT,
    allowed_memories TEXT NOT NULL,         -- JSON array: ["mem_123", "mem_456"] or ["*"] for all
    permissions TEXT NOT NULL DEFAULT '["read","write"]', -- JSON array: ["read"], ["write"], or both
    is_active INTEGER NOT NULL DEFAULT 1,
    last_used_at INTEGER,
    use_count INTEGER NOT NULL DEFAULT 0,
    expires_at INTEGER,                     -- NULL = no expiration
    created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE
);

-- Agent token indexes
CREATE INDEX IF NOT EXISTS idx_agent_tokens_user ON agent_tokens(user_id);
CREATE INDEX IF NOT EXISTS idx_agent_tokens_tenant ON agent_tokens(tenant_id);
CREATE INDEX IF NOT EXISTS idx_agent_tokens_active ON agent_tokens(user_id, is_active);
