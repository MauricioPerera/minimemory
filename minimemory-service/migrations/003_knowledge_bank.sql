-- Migration: 003_knowledge_bank
-- Description: Add knowledge bank tables for RAG functionality
-- Date: 2024-12-24

-- ============================================
-- KNOWLEDGE BANK TABLES
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
-- UPDATE MEMORIES TABLE
-- ============================================

-- Note: SQLite doesn't support ALTER TABLE to modify CHECK constraints
-- The application code should handle 'knowledge' type validation
-- For new deployments, use the updated schema.sql

-- Add index for knowledge memory queries by source
CREATE INDEX IF NOT EXISTS idx_memories_source ON memories(namespace, type) WHERE type = 'knowledge';
