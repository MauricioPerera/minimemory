/**
 * minimemory-service - Agentic Memory Service
 *
 * A serverless API for AI agent memory management with:
 * - Vector similarity search
 * - Keyword (BM25) search
 * - Hybrid search (vector + keyword)
 * - Memory types: episodic, semantic, working
 * - Memory decay and consolidation
 * - D1 persistent storage
 * - Multi-tenant authentication (JWT + API keys)
 *
 * Works on: Node.js, Cloudflare Workers, Bun
 */
import { Hono } from 'hono';
import type { D1Database } from '@cloudflare/workers-types';
import { MemoryManager } from './memory/MemoryManager.js';
import { defaultKeyStore, ApiKeyStore } from './middleware/index.js';
import { D1Storage } from './storage/index.js';
interface Ai {
    run(model: string, inputs: Record<string, unknown>): Promise<unknown>;
}
type Bindings = {
    DB?: D1Database;
    AI?: Ai;
    ENVIRONMENT?: string;
    JWT_SECRET?: string;
    JWT_REFRESH_SECRET?: string;
};
declare const managers: Map<string, MemoryManager>;
/**
 * Get or create a MemoryManager for a namespace
 */
declare function getManager(namespace: string, dimensions?: number): MemoryManager;
declare const app: Hono<{
    Bindings: Bindings;
}, import("hono/types").BlankSchema, "/">;
export default app;
export { app, getManager, managers, defaultKeyStore, ApiKeyStore, D1Storage };
//# sourceMappingURL=index.d.ts.map