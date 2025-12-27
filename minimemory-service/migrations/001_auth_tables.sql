-- Migration: Add multi-tenant authentication tables
-- This migration adds support for user authentication and multi-tenancy

-- ============================================
-- MULTI-TENANT AUTHENTICATION TABLES
-- ============================================

-- Tenants (organizations/teams)
CREATE TABLE IF NOT EXISTS tenants (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    plan TEXT NOT NULL DEFAULT 'free',
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
    role TEXT NOT NULL DEFAULT 'member',
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

-- Add tenant_id to namespaces (nullable for backwards compatibility)
-- Note: SQLite doesn't support ADD COLUMN IF NOT EXISTS, so we use a workaround
CREATE TABLE IF NOT EXISTS _migration_check (done INTEGER);

-- Add tenant columns to api_keys and namespaces if they don't exist
-- We'll recreate these tables if needed in a future migration

-- Create default tenant for legacy data
INSERT OR IGNORE INTO tenants (id, name, plan, max_memories, max_namespaces)
VALUES ('default', 'Default Tenant', 'free', 10000, 10);

-- Link existing default namespace to default tenant
-- This requires modifying existing data which we'll handle in application code
