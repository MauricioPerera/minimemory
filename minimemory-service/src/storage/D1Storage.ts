/**
 * D1 Storage Adapter for minimemory-service
 * Provides persistent storage for memories using Cloudflare D1
 */

import type { D1Database } from '@cloudflare/workers-types';

export interface StoredMemory {
	id: string;
	namespace: string;
	type: 'episodic' | 'semantic' | 'working' | 'knowledge';
	content: string;
	embedding: number[];
	importance: number;
	metadata: Record<string, unknown>;
	sessionId?: string;
	ttl?: number;
	createdAt: number;
	updatedAt: number;
	lastAccessed?: number;
	accessCount: number;
	// Knowledge memory fields (stored in metadata for D1 compatibility)
	sourceId?: string;
	sourceName?: string;
	sourceType?: string;
	chunkIndex?: number;
	totalChunks?: number;
}

export interface NamespaceConfig {
	name: string;
	dimensions: number;
	createdAt: number;
	updatedAt: number;
}

export class D1Storage {
	constructor(private db: D1Database) {}

	// ============ Namespace Operations ============

	async getNamespace(name: string): Promise<NamespaceConfig | null> {
		const result = await this.db
			.prepare('SELECT * FROM namespaces WHERE name = ?')
			.bind(name)
			.first<{
				name: string;
				dimensions: number;
				created_at: number;
				updated_at: number;
			}>();

		if (!result) return null;

		return {
			name: result.name,
			dimensions: result.dimensions,
			createdAt: result.created_at,
			updatedAt: result.updated_at,
		};
	}

	async createNamespace(name: string, dimensions: number): Promise<NamespaceConfig> {
		const now = Date.now();
		await this.db
			.prepare('INSERT INTO namespaces (name, dimensions, created_at, updated_at) VALUES (?, ?, ?, ?)')
			.bind(name, dimensions, now, now)
			.run();

		return { name, dimensions, createdAt: now, updatedAt: now };
	}

	async listNamespaces(): Promise<NamespaceConfig[]> {
		const results = await this.db
			.prepare('SELECT * FROM namespaces ORDER BY name')
			.all<{
				name: string;
				dimensions: number;
				created_at: number;
				updated_at: number;
			}>();

		return (results.results || []).map(r => ({
			name: r.name,
			dimensions: r.dimensions,
			createdAt: r.created_at,
			updatedAt: r.updated_at,
		}));
	}

	async deleteNamespace(name: string): Promise<boolean> {
		const result = await this.db
			.prepare('DELETE FROM namespaces WHERE name = ?')
			.bind(name)
			.run();

		return (result.meta?.changes ?? 0) > 0;
	}

	// ============ Memory Operations ============

	async saveMemory(memory: StoredMemory): Promise<void> {
		const now = Date.now();
		await this.db
			.prepare(`
				INSERT OR REPLACE INTO memories
				(id, namespace, type, content, embedding, importance, metadata, session_id, ttl, created_at, updated_at, last_accessed, access_count)
				VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
			`)
			.bind(
				memory.id,
				memory.namespace,
				memory.type,
				memory.content,
				JSON.stringify(memory.embedding),
				memory.importance,
				JSON.stringify(memory.metadata),
				memory.sessionId || null,
				memory.ttl || null,
				memory.createdAt,
				now,
				memory.lastAccessed || null,
				memory.accessCount
			)
			.run();
	}

	async getMemory(namespace: string, id: string): Promise<StoredMemory | null> {
		const result = await this.db
			.prepare('SELECT * FROM memories WHERE namespace = ? AND id = ?')
			.bind(namespace, id)
			.first<MemoryRow>();

		if (!result) return null;

		// Update access count and last accessed
		await this.db
			.prepare('UPDATE memories SET access_count = access_count + 1, last_accessed = ? WHERE id = ?')
			.bind(Date.now(), id)
			.run();

		return this.rowToMemory(result);
	}

	async getAllMemories(namespace: string): Promise<StoredMemory[]> {
		const results = await this.db
			.prepare('SELECT * FROM memories WHERE namespace = ? ORDER BY created_at DESC')
			.bind(namespace)
			.all<MemoryRow>();

		return (results.results || []).map(r => this.rowToMemory(r));
	}

	async getMemoriesByType(namespace: string, type: string): Promise<StoredMemory[]> {
		const results = await this.db
			.prepare('SELECT * FROM memories WHERE namespace = ? AND type = ? ORDER BY created_at DESC')
			.bind(namespace, type)
			.all<MemoryRow>();

		return (results.results || []).map(r => this.rowToMemory(r));
	}

	async deleteMemory(namespace: string, id: string): Promise<boolean> {
		const result = await this.db
			.prepare('DELETE FROM memories WHERE namespace = ? AND id = ?')
			.bind(namespace, id)
			.run();

		return (result.meta?.changes ?? 0) > 0;
	}

	async deleteMemoriesByType(namespace: string, type: string): Promise<number> {
		const result = await this.db
			.prepare('DELETE FROM memories WHERE namespace = ? AND type = ?')
			.bind(namespace, type)
			.run();

		return result.meta?.changes ?? 0;
	}

	async clearNamespace(namespace: string): Promise<number> {
		const result = await this.db
			.prepare('DELETE FROM memories WHERE namespace = ?')
			.bind(namespace)
			.run();

		return result.meta?.changes ?? 0;
	}

	async updateMemory(namespace: string, id: string, updates: Partial<StoredMemory>): Promise<boolean> {
		const existing = await this.getMemory(namespace, id);
		if (!existing) return false;

		// Filter out undefined values to avoid D1 binding errors
		const filteredUpdates = Object.fromEntries(
			Object.entries(updates).filter(([, v]) => v !== undefined)
		);

		const updated = { ...existing, ...filteredUpdates, updatedAt: Date.now() };
		await this.saveMemory(updated);
		return true;
	}

	// ============ Stats ============

	async getStats(namespace: string): Promise<{
		total: number;
		byType: Record<string, number>;
		averageImportance: number;
		oldestMemory?: number;
		newestMemory?: number;
		knowledgeSources?: number;
	}> {
		const stats = await this.db
			.prepare(`
				SELECT
					COUNT(*) as total,
					SUM(CASE WHEN type = 'episodic' THEN 1 ELSE 0 END) as episodic,
					SUM(CASE WHEN type = 'semantic' THEN 1 ELSE 0 END) as semantic,
					SUM(CASE WHEN type = 'working' THEN 1 ELSE 0 END) as working,
					SUM(CASE WHEN type = 'knowledge' THEN 1 ELSE 0 END) as knowledge,
					AVG(importance) as avg_importance,
					MIN(created_at) as oldest,
					MAX(created_at) as newest
				FROM memories WHERE namespace = ?
			`)
			.bind(namespace)
			.first<{
				total: number;
				episodic: number;
				semantic: number;
				working: number;
				knowledge: number;
				avg_importance: number;
				oldest: number | null;
				newest: number | null;
			}>();

		// Get knowledge sources count
		const sourcesCount = await this.db
			.prepare('SELECT COUNT(*) as count FROM knowledge_sources WHERE namespace = ?')
			.bind(namespace)
			.first<{ count: number }>();

		return {
			total: stats?.total ?? 0,
			byType: {
				episodic: stats?.episodic ?? 0,
				semantic: stats?.semantic ?? 0,
				working: stats?.working ?? 0,
				knowledge: stats?.knowledge ?? 0,
			},
			averageImportance: stats?.avg_importance ?? 0,
			oldestMemory: stats?.oldest ?? undefined,
			newestMemory: stats?.newest ?? undefined,
			knowledgeSources: sourcesCount?.count ?? 0,
		};
	}

	// ============ Cleanup Operations ============

	async cleanupExpired(namespace: string): Promise<number> {
		const now = Date.now();
		const result = await this.db
			.prepare(`
				DELETE FROM memories
				WHERE namespace = ?
				AND type = 'working'
				AND ttl IS NOT NULL
				AND (created_at + ttl) < ?
			`)
			.bind(namespace, now)
			.run();

		return result.meta?.changes ?? 0;
	}

	async applyDecay(namespace: string, decayRate: number = 0.01): Promise<number> {
		const result = await this.db
			.prepare(`
				UPDATE memories
				SET importance = MAX(0.01, importance * (1 - ?)),
				    updated_at = ?
				WHERE namespace = ?
			`)
			.bind(decayRate, Date.now(), namespace)
			.run();

		return result.meta?.changes ?? 0;
	}

	// ============ API Key Operations ============

	async validateApiKey(key: string): Promise<{
		valid: boolean;
		userId?: string;
		namespace?: string;
		permissions?: string[];
		rateLimit?: { limit: number; window: number };
	} | null> {
		const result = await this.db
			.prepare('SELECT * FROM api_keys WHERE key = ? AND is_active = 1')
			.bind(key)
			.first<{
				key: string;
				user_id: string;
				namespace: string | null;
				permissions: string;
				rate_limit: number;
				rate_window: number;
			}>();

		if (!result) return null;

		// Update last used
		await this.db
			.prepare('UPDATE api_keys SET last_used = ? WHERE key = ?')
			.bind(Date.now(), key)
			.run();

		return {
			valid: true,
			userId: result.user_id,
			namespace: result.namespace || undefined,
			permissions: JSON.parse(result.permissions),
			rateLimit: {
				limit: result.rate_limit,
				window: result.rate_window,
			},
		};
	}

	// ============ Helpers ============

	private rowToMemory(row: MemoryRow): StoredMemory {
		const metadata = row.metadata ? JSON.parse(row.metadata) : {};

		const memory: StoredMemory = {
			id: row.id,
			namespace: row.namespace,
			type: row.type as 'episodic' | 'semantic' | 'working' | 'knowledge',
			content: row.content,
			embedding: JSON.parse(row.embedding),
			importance: row.importance,
			metadata,
			sessionId: row.session_id || undefined,
			ttl: row.ttl || undefined,
			createdAt: row.created_at,
			updatedAt: row.updated_at,
			lastAccessed: row.last_accessed || undefined,
			accessCount: row.access_count,
		};

		// Extract knowledge memory fields from metadata
		if (row.type === 'knowledge' && metadata) {
			memory.sourceId = metadata.sourceId;
			memory.sourceName = metadata.sourceName;
			memory.sourceType = metadata.sourceType;
			memory.chunkIndex = metadata.chunkIndex;
			memory.totalChunks = metadata.totalChunks;
		}

		return memory;
	}
}

interface MemoryRow {
	id: string;
	namespace: string;
	type: string;
	content: string;
	embedding: string;
	importance: number;
	metadata: string | null;
	session_id: string | null;
	ttl: number | null;
	created_at: number;
	updated_at: number;
	last_accessed: number | null;
	access_count: number;
}
