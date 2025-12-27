-- Migration: Add audit log table for traceability
-- Tracks all memory operations for compliance and debugging

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
        'memory', 'namespace', 'user', 'tenant', 'session', 'api_key'
    )),
    resource_id TEXT,              -- ID of the affected resource

    -- Actor information
    user_id TEXT,                  -- Who performed the action (null for API key)
    tenant_id TEXT,                -- Tenant context
    namespace TEXT,                -- Namespace context (for memory operations)
    api_key_prefix TEXT,           -- First 8 chars of API key (for auditing)

    -- Request context
    ip_address TEXT,
    user_agent TEXT,
    request_id TEXT,               -- For correlating related operations

    -- Change details (JSON)
    details TEXT,                  -- JSON: { before: {...}, after: {...}, query: {...} }

    -- Result
    success INTEGER NOT NULL DEFAULT 1,
    error_message TEXT,

    -- Performance
    duration_ms INTEGER,

    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE SET NULL,
    FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE SET NULL
);

-- Indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_log(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_audit_action ON audit_log(action, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_audit_resource ON audit_log(resource_type, resource_id);
CREATE INDEX IF NOT EXISTS idx_audit_user ON audit_log(user_id, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_audit_tenant ON audit_log(tenant_id, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_audit_namespace ON audit_log(namespace, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_audit_request ON audit_log(request_id);

-- Composite index for common queries
CREATE INDEX IF NOT EXISTS idx_audit_tenant_action ON audit_log(tenant_id, action, timestamp DESC);
