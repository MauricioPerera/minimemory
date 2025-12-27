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
import { cors } from 'hono/cors';
import { logger } from 'hono/logger';
import { prettyJSON } from 'hono/pretty-json';

import type { D1Database } from '@cloudflare/workers-types';

import { MemoryManager } from './memory/MemoryManager.js';
import { createMemoryRoutes } from './api/memory.js';
import {
	createAuthMiddleware,
	createRateLimitMiddleware,
	defaultKeyStore,
	ApiKeyStore,
} from './middleware/index.js';
import { D1Storage } from './storage/index.js';
import { jwtAuth } from './middleware/jwt.js';
import { extractTenant } from './middleware/tenant.js';
import authRoutes from './routes/auth.js';
import tenantRoutes from './routes/tenants.js';
import auditRoutes from './routes/audit.js';
import knowledgeRoutes from './routes/knowledge.js';
import webhookRoutes from './routes/webhooks.js';
import agentTokenRoutes from './routes/agent-tokens.js';

// Workers AI binding type
interface Ai {
	run(model: string, inputs: Record<string, unknown>): Promise<unknown>;
}

// Cloudflare Workers Bindings
type Bindings = {
	DB?: D1Database;
	AI?: Ai;
	ENVIRONMENT?: string;
	JWT_SECRET?: string;
	JWT_REFRESH_SECRET?: string;
};

// Configuration
const DEFAULT_DIMENSIONS = 1536;
const AUTH_ENABLED = true;

// In-memory caches (used alongside D1 for performance)
const managers = new Map<string, MemoryManager>();
const namespaceConfig = new Map<string, number>();

/**
 * Get or create a MemoryManager for a namespace
 */
function getManager(namespace: string, dimensions?: number): MemoryManager {
	let manager = managers.get(namespace);
	if (!manager) {
		const dims = dimensions || namespaceConfig.get(namespace) || DEFAULT_DIMENSIONS;
		manager = new MemoryManager({
			dimensions: dims,
			textFields: ['content', 'event', 'fact', 'context', 'description'],
		});
		managers.set(namespace, manager);
		namespaceConfig.set(namespace, dims);
	}
	return manager;
}

/**
 * Create a namespace with specific configuration
 */
function createNamespaceLocal(name: string, dimensions: number): MemoryManager {
	if (managers.has(name)) {
		throw new Error(`Namespace "${name}" already exists`);
	}
	namespaceConfig.set(name, dimensions);
	const manager = new MemoryManager({
		dimensions,
		textFields: ['content', 'event', 'fact', 'context', 'description'],
	});
	managers.set(name, manager);
	return manager;
}

// Create Hono app with bindings
const app = new Hono<{ Bindings: Bindings }>();

// Middleware
app.use('*', logger());
app.use('*', cors());
app.use('*', prettyJSON());

// Auth middleware stack:
// 1. JWT auth (extracts user from Bearer token if present)
// 2. API key auth (fallback for backwards compatibility)
// 3. Tenant extraction (from X-Tenant-Id header or API key)
// 4. Rate limiting
if (AUTH_ENABLED) {
	// JWT middleware runs first (but doesn't require auth - just extracts if present)
	app.use('/api/*', jwtAuth);

	// API key auth as fallback (for clients not using JWT)
	app.use('/api/*', createAuthMiddleware({
		publicPaths: ['/api/v1/auth/register', '/api/v1/auth/login', '/api/v1/auth/refresh', '/api/v1/auth/validate-agent'],
	}));

	// Extract tenant context
	app.use('/api/*', extractTenant);

	// Rate limiting
	app.use('/api/*', createRateLimitMiddleware({
		defaultLimit: 100,
		defaultWindow: 60,
	}));
}

// Mount auth routes (register, login, refresh, etc.)
app.route('/api/v1/auth', authRoutes);

// Mount tenant routes (CRUD for tenants and members)
app.route('/api/v1/tenants', tenantRoutes);

// Mount audit routes (query audit logs)
app.route('/api/v1/audit', auditRoutes);

// Mount knowledge routes (RAG knowledge bank)
app.route('/api/v1/knowledge', knowledgeRoutes);

// Mount webhook routes
app.route('/api/v1/webhooks', webhookRoutes);

// Mount agent token routes (MCP access control)
app.route('/api/v1/agent-tokens', agentTokenRoutes);

// Health check
app.get('/', (c) => {
	const hasD1 = !!c.env?.DB;
	const hasJWT = !!c.env?.JWT_SECRET;
	const hasAI = !!c.env?.AI;
	return c.json({
		service: 'minimemory',
		version: '0.4.0',
		status: 'ok',
		storage: hasD1 ? 'd1' : 'memory',
		embeddings: hasAI ? 'workers-ai' : 'external',
		auth: {
			enabled: AUTH_ENABLED,
			jwt: hasJWT ? 'configured' : 'not configured',
			apiKeys: 'supported',
			devKey: AUTH_ENABLED ? 'mm_dev_key_12345' : undefined,
		},
		endpoints: {
			// Auth
			'POST /api/v1/auth/register': 'Create new account',
			'POST /api/v1/auth/login': 'Login and get tokens',
			'POST /api/v1/auth/refresh': 'Refresh access token',
			'POST /api/v1/auth/logout': 'Logout and invalidate session',
			'GET /api/v1/auth/me': 'Get current user profile',
			// Tenants
			'GET /api/v1/tenants': 'List user tenants',
			'POST /api/v1/tenants': 'Create new tenant',
			'GET /api/v1/tenants/:id': 'Get tenant details',
			'PUT /api/v1/tenants/:id': 'Update tenant',
			'DELETE /api/v1/tenants/:id': 'Delete tenant',
			'GET /api/v1/tenants/:id/members': 'List tenant members',
			'POST /api/v1/tenants/:id/members': 'Invite member',
			// Memory
			'POST /api/v1/remember': 'Store a memory (auto-generates embedding)',
			'POST /api/v1/recall': 'Search for memories (query auto-generates embedding)',
			'DELETE /api/v1/forget/:id': 'Delete a memory',
			'POST /api/v1/forget': 'Delete memories by filter',
			'GET /api/v1/memory/:id': 'Get a specific memory',
			'PATCH /api/v1/memory/:id': 'Update a memory',
			'GET /api/v1/stats': 'Get memory statistics',
			'POST /api/v1/cleanup': 'Clean up expired memories',
			'POST /api/v1/decay': 'Apply importance decay',
			'POST /api/v1/export': 'Export all memories',
			'POST /api/v1/import': 'Import memories',
			'DELETE /api/v1/clear': 'Clear all memories',
			// Embeddings
			'POST /api/v1/embed': 'Generate embeddings (EmbeddingGemma)',
			'GET /api/v1/embed/info': 'Get embedding service info',
			// Audit
			'GET /api/v1/audit': 'Query audit logs',
			'GET /api/v1/audit/:id': 'Get audit entry by ID',
			'GET /api/v1/audit/resource/:type/:id': 'Get resource history',
			'GET /api/v1/audit/user/:id': 'Get user activity',
			'GET /api/v1/audit/failures': 'Get failed operations',
			'GET /api/v1/audit/stats': 'Get audit statistics',
			'POST /api/v1/audit/cleanup': 'Clean up old audit logs',
			// Knowledge Bank (RAG)
			'POST /api/v1/knowledge/ingest': 'Ingest document into knowledge bank',
			'GET /api/v1/knowledge/sources': 'List knowledge sources',
			'GET /api/v1/knowledge/sources/:id': 'Get source details',
			'DELETE /api/v1/knowledge/sources/:id': 'Delete source and chunks',
			'GET /api/v1/knowledge/sources/:id/chunks': 'Get source chunks',
			'GET /api/v1/knowledge/stats': 'Get knowledge bank statistics',
			'POST /api/v1/knowledge/chunk-preview': 'Preview document chunking',
			// Webhooks
			'GET /api/v1/webhooks/events': 'List available webhook events',
			'GET /api/v1/webhooks': 'List webhooks',
			'POST /api/v1/webhooks': 'Create webhook',
			'GET /api/v1/webhooks/:id': 'Get webhook details',
			'PUT /api/v1/webhooks/:id': 'Update webhook',
			'DELETE /api/v1/webhooks/:id': 'Delete webhook',
			'POST /api/v1/webhooks/:id/test': 'Test webhook',
			'POST /api/v1/webhooks/:id/rotate-secret': 'Rotate webhook secret',
			'GET /api/v1/webhooks/:id/deliveries': 'Get delivery history',
			// Agent Tokens (MCP access control)
			'GET /api/v1/agent-tokens': 'List agent tokens',
			'POST /api/v1/agent-tokens': 'Create agent token',
			'GET /api/v1/agent-tokens/:id': 'Get agent token',
			'PATCH /api/v1/agent-tokens/:id': 'Update agent token',
			'DELETE /api/v1/agent-tokens/:id': 'Delete agent token',
			'POST /api/v1/agent-tokens/:id/toggle': 'Toggle token active status',
			'POST /api/v1/agent-tokens/:id/add-memory': 'Add memory to allowed list',
			'POST /api/v1/agent-tokens/:id/remove-memory': 'Remove memory from allowed list',
			'POST /api/v1/auth/validate-agent': 'Validate API key + agent token (for MCP)',
		},
	});
});

// Health endpoint
app.get('/health', (c) => {
	return c.json({
		status: 'healthy',
		timestamp: new Date().toISOString(),
		storage: c.env?.DB ? 'd1' : 'memory',
	});
});

// Mount memory API
app.route('/api/v1', createMemoryRoutes(getManager));

// List namespaces (with D1 support)
app.get('/api/v1/namespaces', async (c) => {
	if (c.env?.DB) {
		const storage = new D1Storage(c.env.DB);
		const namespaces = await storage.listNamespaces();
		return c.json({
			success: true,
			namespaces: namespaces.map(ns => ({ name: ns.name, dimensions: ns.dimensions })),
			count: namespaces.length,
			storage: 'd1',
		});
	}

	// Fallback to memory
	const namespaces = Array.from(managers.keys()).map(name => ({
		name,
		dimensions: namespaceConfig.get(name) || DEFAULT_DIMENSIONS,
	}));
	return c.json({
		success: true,
		namespaces,
		count: namespaces.length,
		storage: 'memory',
	});
});

// Create namespace (with D1 support)
app.post('/api/v1/namespaces', async (c) => {
	try {
		const body = await c.req.json();
		const { name, dimensions } = body;

		if (!name || typeof name !== 'string') {
			return c.json({ error: 'name is required' }, 400);
		}

		if (!dimensions || typeof dimensions !== 'number' || dimensions < 1) {
			return c.json({ error: 'dimensions must be a positive number' }, 400);
		}

		if (c.env?.DB) {
			const storage = new D1Storage(c.env.DB);
			const existing = await storage.getNamespace(name);
			if (existing) {
				return c.json({ error: `Namespace "${name}" already exists` }, 400);
			}
			await storage.createNamespace(name, dimensions);
			// Also create in memory cache
			namespaceConfig.set(name, dimensions);
		} else {
			createNamespaceLocal(name, dimensions);
		}

		return c.json({
			success: true,
			namespace: { name, dimensions },
			message: `Namespace "${name}" created with ${dimensions} dimensions`,
			storage: c.env?.DB ? 'd1' : 'memory',
		});
	} catch (error) {
		return c.json({
			error: error instanceof Error ? error.message : 'Unknown error',
		}, 400);
	}
});

// Delete namespace (with D1 support)
app.delete('/api/v1/namespaces/:name', async (c) => {
	const name = c.req.param('name');

	let deleted = false;
	if (c.env?.DB) {
		const storage = new D1Storage(c.env.DB);
		deleted = await storage.deleteNamespace(name);
	}

	// Also delete from memory cache
	managers.delete(name);
	namespaceConfig.delete(name);

	return c.json({
		success: deleted || managers.has(name) === false,
		message: deleted ? `Namespace "${name}" deleted` : `Namespace "${name}" not found`,
	});
});

// Error handler
app.onError((err, c) => {
	console.error('Error:', err);
	return c.json({
		error: err.message || 'Internal server error',
	}, 500);
});

// 404 handler
app.notFound((c) => {
	return c.json({
		error: 'Not found',
		path: c.req.path,
	}, 404);
});

// Export for Cloudflare Workers
export default app;

// Named exports
export { app, getManager, managers, defaultKeyStore, ApiKeyStore, D1Storage };
