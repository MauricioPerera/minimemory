/**
 * Tests for D1Storage Adapter
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { D1Storage, StoredMemory, NamespaceConfig } from '../../src/storage/D1Storage.js';

interface MockDbData {
	namespaces: Map<string, {
		name: string;
		dimensions: number;
		created_at: number;
		updated_at: number;
		tenant_id?: string;
	}>;
	memories: Map<string, {
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
	}>;
	apiKeys: Map<string, {
		key: string;
		user_id: string;
		namespace: string | null;
		permissions: string;
		rate_limit: number;
		rate_window: number;
		is_active: number;
		last_used: number | null;
	}>;
	knowledgeSources: Map<string, {
		id: string;
		namespace: string;
		name: string;
	}>;
}

function createMockD1(data: MockDbData) {
	const createMethods = (sql: string, params: unknown[]) => ({
		run: vi.fn(async () => {
			// INSERT INTO namespaces
			if (sql.includes('INSERT INTO namespaces')) {
				const [name, dimensions, created_at, updated_at] = params;
				data.namespaces.set(name as string, {
					name: name as string,
					dimensions: dimensions as number,
					created_at: created_at as number,
					updated_at: updated_at as number,
				});
				return { meta: { changes: 1 } };
			}

			// DELETE FROM namespaces
			if (sql.includes('DELETE FROM namespaces WHERE name')) {
				const name = params[0] as string;
				const deleted = data.namespaces.delete(name);
				return { meta: { changes: deleted ? 1 : 0 } };
			}

			// INSERT OR REPLACE INTO memories
			if (sql.includes('INSERT OR REPLACE INTO memories')) {
				const [id, namespace, type, content, embedding, importance, metadata, session_id, ttl, created_at, updated_at, last_accessed, access_count] = params;
				data.memories.set(id as string, {
					id: id as string,
					namespace: namespace as string,
					type: type as string,
					content: content as string,
					embedding: embedding as string,
					importance: importance as number,
					metadata: metadata as string | null,
					session_id: session_id as string | null,
					ttl: ttl as number | null,
					created_at: created_at as number,
					updated_at: updated_at as number,
					last_accessed: last_accessed as number | null,
					access_count: access_count as number,
				});
				return { meta: { changes: 1 } };
			}

			// UPDATE memories access count
			if (sql.includes('UPDATE memories SET access_count')) {
				const [last_accessed, id] = params;
				const memory = data.memories.get(id as string);
				if (memory) {
					memory.access_count++;
					memory.last_accessed = last_accessed as number;
				}
				return { meta: { changes: memory ? 1 : 0 } };
			}

			// DELETE FROM memories WHERE namespace = ? AND id = ?
			if (sql.includes('DELETE FROM memories WHERE namespace = ? AND id = ?')) {
				const [namespace, id] = params;
				const memory = data.memories.get(id as string);
				if (memory && memory.namespace === namespace) {
					data.memories.delete(id as string);
					return { meta: { changes: 1 } };
				}
				return { meta: { changes: 0 } };
			}

			// DELETE FROM memories WHERE namespace = ? AND type = ?
			if (sql.includes('DELETE FROM memories WHERE namespace = ? AND type = ?')) {
				const [namespace, type] = params;
				let count = 0;
				for (const [id, memory] of data.memories) {
					if (memory.namespace === namespace && memory.type === type) {
						data.memories.delete(id);
						count++;
					}
				}
				return { meta: { changes: count } };
			}

			// DELETE FROM memories WHERE namespace = ?
			if (sql.includes('DELETE FROM memories WHERE namespace = ?') && !sql.includes('type') && !sql.includes('working')) {
				const namespace = params[0] as string;
				let count = 0;
				for (const [id, memory] of data.memories) {
					if (memory.namespace === namespace) {
						data.memories.delete(id);
						count++;
					}
				}
				return { meta: { changes: count } };
			}

			// DELETE expired working memories
			if (sql.includes('DELETE FROM memories') && sql.includes('working') && sql.includes('ttl')) {
				const [namespace, now] = params;
				let count = 0;
				for (const [id, memory] of data.memories) {
					if (
						memory.namespace === namespace &&
						memory.type === 'working' &&
						memory.ttl !== null &&
						memory.created_at + memory.ttl < (now as number)
					) {
						data.memories.delete(id);
						count++;
					}
				}
				return { meta: { changes: count } };
			}

			// UPDATE memories SET importance (decay)
			if (sql.includes('UPDATE memories') && sql.includes('importance') && sql.includes('MAX')) {
				const [decayRate, updated_at, namespace] = params;
				let count = 0;
				for (const memory of data.memories.values()) {
					if (memory.namespace === namespace) {
						memory.importance = Math.max(0.01, memory.importance * (1 - (decayRate as number)));
						memory.updated_at = updated_at as number;
						count++;
					}
				}
				return { meta: { changes: count } };
			}

			// UPDATE api_keys last_used
			if (sql.includes('UPDATE api_keys SET last_used')) {
				const [last_used, key] = params;
				const apiKey = data.apiKeys.get(key as string);
				if (apiKey) {
					apiKey.last_used = last_used as number;
				}
				return { meta: { changes: apiKey ? 1 : 0 } };
			}

			return { meta: { changes: 0 } };
		}),

		first: vi.fn(async <T>(): Promise<T | null> => {
			// SELECT * FROM namespaces WHERE name
			if (sql.includes('SELECT * FROM namespaces WHERE name')) {
				const name = params[0] as string;
				const ns = data.namespaces.get(name);
				return ns as T | null;
			}

			// SELECT * FROM memories WHERE namespace = ? AND id = ?
			if (sql.includes('SELECT * FROM memories WHERE namespace = ? AND id = ?')) {
				const [namespace, id] = params;
				const memory = data.memories.get(id as string);
				if (memory && memory.namespace === namespace) {
					return memory as T;
				}
				return null;
			}

			// SELECT stats
			if (sql.includes('COUNT(*) as total') && sql.includes('FROM memories')) {
				const namespace = params[0] as string;
				let total = 0;
				let episodic = 0;
				let semantic = 0;
				let working = 0;
				let knowledge = 0;
				let sumImportance = 0;
				let oldest: number | null = null;
				let newest: number | null = null;

				for (const memory of data.memories.values()) {
					if (memory.namespace === namespace) {
						total++;
						sumImportance += memory.importance;
						if (oldest === null || memory.created_at < oldest) oldest = memory.created_at;
						if (newest === null || memory.created_at > newest) newest = memory.created_at;

						switch (memory.type) {
							case 'episodic': episodic++; break;
							case 'semantic': semantic++; break;
							case 'working': working++; break;
							case 'knowledge': knowledge++; break;
						}
					}
				}

				return {
					total,
					episodic,
					semantic,
					working,
					knowledge,
					avg_importance: total > 0 ? sumImportance / total : 0,
					oldest,
					newest,
				} as T;
			}

			// SELECT COUNT(*) FROM knowledge_sources
			if (sql.includes('COUNT(*) as count FROM knowledge_sources')) {
				const namespace = params[0] as string;
				let count = 0;
				for (const source of data.knowledgeSources.values()) {
					if (source.namespace === namespace) count++;
				}
				return { count } as T;
			}

			// SELECT * FROM api_keys
			if (sql.includes('SELECT * FROM api_keys')) {
				const key = params[0] as string;
				const apiKey = data.apiKeys.get(key);
				if (apiKey && apiKey.is_active === 1) {
					return apiKey as T;
				}
				return null;
			}

			return null;
		}),

		all: vi.fn(async <T>(): Promise<{ results: T[] }> => {
			// SELECT * FROM namespaces ORDER BY name
			if (sql.includes('SELECT * FROM namespaces ORDER BY name')) {
				const results = Array.from(data.namespaces.values())
					.sort((a, b) => a.name.localeCompare(b.name));
				return { results: results as T[] };
			}

			// SELECT * FROM memories WHERE namespace = ? AND type = ?
			if (sql.includes('SELECT * FROM memories WHERE namespace = ?') && sql.includes('type = ?')) {
				const [namespace, type] = params;
				const results = Array.from(data.memories.values())
					.filter(m => m.namespace === namespace && m.type === type)
					.sort((a, b) => b.created_at - a.created_at);
				return { results: results as T[] };
			}

			// SELECT * FROM memories WHERE namespace = ?
			if (sql.includes('SELECT * FROM memories WHERE namespace = ?') && !sql.includes('type = ?')) {
				const namespace = params[0] as string;
				const results = Array.from(data.memories.values())
					.filter(m => m.namespace === namespace)
					.sort((a, b) => b.created_at - a.created_at);
				return { results: results as T[] };
			}

			return { results: [] };
		}),
	});

	return {
		prepare: vi.fn((sql: string) => ({
			bind: vi.fn((...params: unknown[]) => createMethods(sql, params)),
			// Direct all() without bind() for listNamespaces
			all: vi.fn(async <T>(): Promise<{ results: T[] }> => {
				if (sql.includes('SELECT * FROM namespaces ORDER BY name')) {
					const results = Array.from(data.namespaces.values())
						.sort((a, b) => a.name.localeCompare(b.name));
					return { results: results as T[] };
				}
				return { results: [] };
			}),
		})),
	} as unknown as D1Database;
}

describe('D1Storage', () => {
	let mockData: MockDbData;
	let db: D1Database;
	let storage: D1Storage;

	beforeEach(() => {
		mockData = {
			namespaces: new Map(),
			memories: new Map(),
			apiKeys: new Map(),
			knowledgeSources: new Map(),
		};
		db = createMockD1(mockData);
		storage = new D1Storage(db);
	});

	describe('Namespace Operations', () => {
		describe('createNamespace()', () => {
			it('should create a namespace with correct values', async () => {
				const result = await storage.createNamespace('test-ns', 768);

				expect(result.name).toBe('test-ns');
				expect(result.dimensions).toBe(768);
				expect(result.createdAt).toBeGreaterThan(0);
				expect(result.updatedAt).toBeGreaterThan(0);
			});

			it('should store namespace in database', async () => {
				await storage.createNamespace('stored-ns', 1536);

				expect(mockData.namespaces.has('stored-ns')).toBe(true);
				expect(mockData.namespaces.get('stored-ns')?.dimensions).toBe(1536);
			});
		});

		describe('getNamespace()', () => {
			it('should return namespace if exists', async () => {
				mockData.namespaces.set('existing-ns', {
					name: 'existing-ns',
					dimensions: 768,
					created_at: Date.now() - 1000,
					updated_at: Date.now(),
				});

				const result = await storage.getNamespace('existing-ns');

				expect(result).not.toBeNull();
				expect(result?.name).toBe('existing-ns');
				expect(result?.dimensions).toBe(768);
			});

			it('should return null if namespace does not exist', async () => {
				const result = await storage.getNamespace('nonexistent');

				expect(result).toBeNull();
			});
		});

		describe('listNamespaces()', () => {
			it('should return all namespaces sorted by name', async () => {
				mockData.namespaces.set('z-ns', { name: 'z-ns', dimensions: 768, created_at: 1, updated_at: 1 });
				mockData.namespaces.set('a-ns', { name: 'a-ns', dimensions: 768, created_at: 2, updated_at: 2 });
				mockData.namespaces.set('m-ns', { name: 'm-ns', dimensions: 768, created_at: 3, updated_at: 3 });

				const results = await storage.listNamespaces();

				expect(results).toHaveLength(3);
				expect(results[0].name).toBe('a-ns');
				expect(results[1].name).toBe('m-ns');
				expect(results[2].name).toBe('z-ns');
			});

			it('should return empty array when no namespaces exist', async () => {
				const results = await storage.listNamespaces();

				expect(results).toHaveLength(0);
			});
		});

		describe('deleteNamespace()', () => {
			it('should delete existing namespace', async () => {
				mockData.namespaces.set('to-delete', { name: 'to-delete', dimensions: 768, created_at: 1, updated_at: 1 });

				const result = await storage.deleteNamespace('to-delete');

				expect(result).toBe(true);
				expect(mockData.namespaces.has('to-delete')).toBe(false);
			});

			it('should return false for non-existent namespace', async () => {
				const result = await storage.deleteNamespace('nonexistent');

				expect(result).toBe(false);
			});
		});
	});

	describe('Memory Operations', () => {
		const testMemory: StoredMemory = {
			id: 'mem-1',
			namespace: 'test-ns',
			type: 'episodic',
			content: 'Test content',
			embedding: [0.1, 0.2, 0.3],
			importance: 0.8,
			metadata: { key: 'value' },
			createdAt: Date.now(),
			updatedAt: Date.now(),
			accessCount: 0,
		};

		describe('saveMemory()', () => {
			it('should save memory to database', async () => {
				await storage.saveMemory(testMemory);

				expect(mockData.memories.has('mem-1')).toBe(true);
				const saved = mockData.memories.get('mem-1');
				expect(saved?.content).toBe('Test content');
				expect(saved?.type).toBe('episodic');
				expect(saved?.importance).toBe(0.8);
			});

			it('should serialize embedding as JSON', async () => {
				await storage.saveMemory(testMemory);

				const saved = mockData.memories.get('mem-1');
				expect(saved?.embedding).toBe('[0.1,0.2,0.3]');
			});

			it('should serialize metadata as JSON', async () => {
				await storage.saveMemory(testMemory);

				const saved = mockData.memories.get('mem-1');
				expect(saved?.metadata).toBe('{"key":"value"}');
			});

			it('should handle memory with ttl', async () => {
				await storage.saveMemory({ ...testMemory, ttl: 3600000 });

				const saved = mockData.memories.get('mem-1');
				expect(saved?.ttl).toBe(3600000);
			});

			it('should handle memory with sessionId', async () => {
				await storage.saveMemory({ ...testMemory, sessionId: 'session-123' });

				const saved = mockData.memories.get('mem-1');
				expect(saved?.session_id).toBe('session-123');
			});
		});

		describe('getMemory()', () => {
			beforeEach(() => {
				mockData.memories.set('mem-1', {
					id: 'mem-1',
					namespace: 'test-ns',
					type: 'episodic',
					content: 'Test content',
					embedding: '[0.1,0.2,0.3]',
					importance: 0.8,
					metadata: '{"key":"value"}',
					session_id: null,
					ttl: null,
					created_at: Date.now() - 1000,
					updated_at: Date.now(),
					last_accessed: null,
					access_count: 5,
				});
			});

			it('should return memory if exists', async () => {
				const result = await storage.getMemory('test-ns', 'mem-1');

				expect(result).not.toBeNull();
				expect(result?.id).toBe('mem-1');
				expect(result?.content).toBe('Test content');
			});

			it('should parse embedding from JSON', async () => {
				const result = await storage.getMemory('test-ns', 'mem-1');

				expect(result?.embedding).toEqual([0.1, 0.2, 0.3]);
			});

			it('should parse metadata from JSON', async () => {
				const result = await storage.getMemory('test-ns', 'mem-1');

				expect(result?.metadata).toEqual({ key: 'value' });
			});

			it('should return null if memory does not exist', async () => {
				const result = await storage.getMemory('test-ns', 'nonexistent');

				expect(result).toBeNull();
			});

			it('should return null for wrong namespace', async () => {
				const result = await storage.getMemory('wrong-ns', 'mem-1');

				expect(result).toBeNull();
			});

			it('should increment access count on get', async () => {
				const initialCount = mockData.memories.get('mem-1')!.access_count;

				await storage.getMemory('test-ns', 'mem-1');

				expect(mockData.memories.get('mem-1')!.access_count).toBe(initialCount + 1);
			});
		});

		describe('getAllMemories()', () => {
			beforeEach(() => {
				mockData.memories.set('mem-1', {
					id: 'mem-1', namespace: 'test-ns', type: 'episodic', content: 'Content 1',
					embedding: '[]', importance: 0.5, metadata: null, session_id: null, ttl: null,
					created_at: Date.now() - 2000, updated_at: Date.now(), last_accessed: null, access_count: 0,
				});
				mockData.memories.set('mem-2', {
					id: 'mem-2', namespace: 'test-ns', type: 'semantic', content: 'Content 2',
					embedding: '[]', importance: 0.7, metadata: null, session_id: null, ttl: null,
					created_at: Date.now() - 1000, updated_at: Date.now(), last_accessed: null, access_count: 0,
				});
				mockData.memories.set('mem-3', {
					id: 'mem-3', namespace: 'other-ns', type: 'episodic', content: 'Content 3',
					embedding: '[]', importance: 0.3, metadata: null, session_id: null, ttl: null,
					created_at: Date.now(), updated_at: Date.now(), last_accessed: null, access_count: 0,
				});
			});

			it('should return all memories for namespace', async () => {
				const results = await storage.getAllMemories('test-ns');

				expect(results).toHaveLength(2);
			});

			it('should not include memories from other namespaces', async () => {
				const results = await storage.getAllMemories('test-ns');

				expect(results.every(m => m.namespace === 'test-ns')).toBe(true);
			});

			it('should order by created_at descending', async () => {
				const results = await storage.getAllMemories('test-ns');

				expect(results[0].id).toBe('mem-2'); // Newer
				expect(results[1].id).toBe('mem-1'); // Older
			});
		});

		describe('getMemoriesByType()', () => {
			beforeEach(() => {
				mockData.memories.set('ep-1', {
					id: 'ep-1', namespace: 'test-ns', type: 'episodic', content: 'Episodic 1',
					embedding: '[]', importance: 0.5, metadata: null, session_id: null, ttl: null,
					created_at: Date.now(), updated_at: Date.now(), last_accessed: null, access_count: 0,
				});
				mockData.memories.set('sem-1', {
					id: 'sem-1', namespace: 'test-ns', type: 'semantic', content: 'Semantic 1',
					embedding: '[]', importance: 0.7, metadata: null, session_id: null, ttl: null,
					created_at: Date.now(), updated_at: Date.now(), last_accessed: null, access_count: 0,
				});
			});

			it('should return only memories of specified type', async () => {
				const results = await storage.getMemoriesByType('test-ns', 'episodic');

				expect(results).toHaveLength(1);
				expect(results[0].type).toBe('episodic');
			});

			it('should return empty array if no memories of type', async () => {
				const results = await storage.getMemoriesByType('test-ns', 'working');

				expect(results).toHaveLength(0);
			});
		});

		describe('deleteMemory()', () => {
			beforeEach(() => {
				mockData.memories.set('to-delete', {
					id: 'to-delete', namespace: 'test-ns', type: 'episodic', content: 'Delete me',
					embedding: '[]', importance: 0.5, metadata: null, session_id: null, ttl: null,
					created_at: Date.now(), updated_at: Date.now(), last_accessed: null, access_count: 0,
				});
			});

			it('should delete existing memory', async () => {
				const result = await storage.deleteMemory('test-ns', 'to-delete');

				expect(result).toBe(true);
				expect(mockData.memories.has('to-delete')).toBe(false);
			});

			it('should return false for non-existent memory', async () => {
				const result = await storage.deleteMemory('test-ns', 'nonexistent');

				expect(result).toBe(false);
			});

			it('should return false for wrong namespace', async () => {
				const result = await storage.deleteMemory('wrong-ns', 'to-delete');

				expect(result).toBe(false);
			});
		});

		describe('deleteMemoriesByType()', () => {
			beforeEach(() => {
				mockData.memories.set('ep-1', { id: 'ep-1', namespace: 'test-ns', type: 'episodic', content: '', embedding: '[]', importance: 0.5, metadata: null, session_id: null, ttl: null, created_at: Date.now(), updated_at: Date.now(), last_accessed: null, access_count: 0 });
				mockData.memories.set('ep-2', { id: 'ep-2', namespace: 'test-ns', type: 'episodic', content: '', embedding: '[]', importance: 0.5, metadata: null, session_id: null, ttl: null, created_at: Date.now(), updated_at: Date.now(), last_accessed: null, access_count: 0 });
				mockData.memories.set('sem-1', { id: 'sem-1', namespace: 'test-ns', type: 'semantic', content: '', embedding: '[]', importance: 0.5, metadata: null, session_id: null, ttl: null, created_at: Date.now(), updated_at: Date.now(), last_accessed: null, access_count: 0 });
			});

			it('should delete all memories of specified type', async () => {
				const count = await storage.deleteMemoriesByType('test-ns', 'episodic');

				expect(count).toBe(2);
				expect(mockData.memories.has('ep-1')).toBe(false);
				expect(mockData.memories.has('ep-2')).toBe(false);
			});

			it('should not delete memories of other types', async () => {
				await storage.deleteMemoriesByType('test-ns', 'episodic');

				expect(mockData.memories.has('sem-1')).toBe(true);
			});
		});

		describe('clearNamespace()', () => {
			beforeEach(() => {
				mockData.memories.set('mem-1', { id: 'mem-1', namespace: 'test-ns', type: 'episodic', content: '', embedding: '[]', importance: 0.5, metadata: null, session_id: null, ttl: null, created_at: Date.now(), updated_at: Date.now(), last_accessed: null, access_count: 0 });
				mockData.memories.set('mem-2', { id: 'mem-2', namespace: 'test-ns', type: 'semantic', content: '', embedding: '[]', importance: 0.5, metadata: null, session_id: null, ttl: null, created_at: Date.now(), updated_at: Date.now(), last_accessed: null, access_count: 0 });
				mockData.memories.set('other', { id: 'other', namespace: 'other-ns', type: 'episodic', content: '', embedding: '[]', importance: 0.5, metadata: null, session_id: null, ttl: null, created_at: Date.now(), updated_at: Date.now(), last_accessed: null, access_count: 0 });
			});

			it('should delete all memories in namespace', async () => {
				const count = await storage.clearNamespace('test-ns');

				expect(count).toBe(2);
			});

			it('should not affect other namespaces', async () => {
				await storage.clearNamespace('test-ns');

				expect(mockData.memories.has('other')).toBe(true);
			});
		});

		describe('updateMemory()', () => {
			beforeEach(() => {
				mockData.memories.set('mem-1', {
					id: 'mem-1', namespace: 'test-ns', type: 'episodic', content: 'Original content',
					embedding: '[0.1]', importance: 0.5, metadata: '{}', session_id: null, ttl: null,
					created_at: Date.now() - 1000, updated_at: Date.now() - 1000, last_accessed: null, access_count: 0,
				});
			});

			it('should update memory fields', async () => {
				const result = await storage.updateMemory('test-ns', 'mem-1', {
					content: 'Updated content',
					importance: 0.9,
				});

				expect(result).toBe(true);
				expect(mockData.memories.get('mem-1')?.content).toBe('Updated content');
				expect(mockData.memories.get('mem-1')?.importance).toBe(0.9);
			});

			it('should return false for non-existent memory', async () => {
				const result = await storage.updateMemory('test-ns', 'nonexistent', {
					content: 'Updated',
				});

				expect(result).toBe(false);
			});
		});
	});

	describe('Stats', () => {
		describe('getStats()', () => {
			beforeEach(() => {
				mockData.memories.set('ep-1', { id: 'ep-1', namespace: 'test-ns', type: 'episodic', content: '', embedding: '[]', importance: 0.5, metadata: null, session_id: null, ttl: null, created_at: 1000, updated_at: Date.now(), last_accessed: null, access_count: 0 });
				mockData.memories.set('ep-2', { id: 'ep-2', namespace: 'test-ns', type: 'episodic', content: '', embedding: '[]', importance: 0.7, metadata: null, session_id: null, ttl: null, created_at: 2000, updated_at: Date.now(), last_accessed: null, access_count: 0 });
				mockData.memories.set('sem-1', { id: 'sem-1', namespace: 'test-ns', type: 'semantic', content: '', embedding: '[]', importance: 0.9, metadata: null, session_id: null, ttl: null, created_at: 3000, updated_at: Date.now(), last_accessed: null, access_count: 0 });
				mockData.memories.set('know-1', { id: 'know-1', namespace: 'test-ns', type: 'knowledge', content: '', embedding: '[]', importance: 0.6, metadata: '{"sourceId":"src-1"}', session_id: null, ttl: null, created_at: 4000, updated_at: Date.now(), last_accessed: null, access_count: 0 });
				mockData.knowledgeSources.set('src-1', { id: 'src-1', namespace: 'test-ns', name: 'Source 1' });
			});

			it('should return total count', async () => {
				const stats = await storage.getStats('test-ns');

				expect(stats.total).toBe(4);
			});

			it('should return counts by type', async () => {
				const stats = await storage.getStats('test-ns');

				expect(stats.byType.episodic).toBe(2);
				expect(stats.byType.semantic).toBe(1);
				expect(stats.byType.working).toBe(0);
				expect(stats.byType.knowledge).toBe(1);
			});

			it('should calculate average importance', async () => {
				const stats = await storage.getStats('test-ns');

				expect(stats.averageImportance).toBeCloseTo(0.675, 2);
			});

			it('should return oldest and newest memory timestamps', async () => {
				const stats = await storage.getStats('test-ns');

				expect(stats.oldestMemory).toBe(1000);
				expect(stats.newestMemory).toBe(4000);
			});

			it('should return knowledge sources count', async () => {
				const stats = await storage.getStats('test-ns');

				expect(stats.knowledgeSources).toBe(1);
			});

			it('should return zeros for empty namespace', async () => {
				const stats = await storage.getStats('empty-ns');

				expect(stats.total).toBe(0);
				expect(stats.byType.episodic).toBe(0);
				expect(stats.averageImportance).toBe(0);
			});
		});
	});

	describe('Cleanup Operations', () => {
		describe('cleanupExpired()', () => {
			it('should delete expired working memories', async () => {
				const now = Date.now();
				mockData.memories.set('expired', {
					id: 'expired', namespace: 'test-ns', type: 'working', content: '',
					embedding: '[]', importance: 0.5, metadata: null, session_id: null,
					ttl: 1000, // 1 second TTL
					created_at: now - 2000, // Created 2 seconds ago (expired)
					updated_at: now, last_accessed: null, access_count: 0,
				});
				mockData.memories.set('valid', {
					id: 'valid', namespace: 'test-ns', type: 'working', content: '',
					embedding: '[]', importance: 0.5, metadata: null, session_id: null,
					ttl: 10000, // 10 second TTL
					created_at: now - 1000, // Created 1 second ago (still valid)
					updated_at: now, last_accessed: null, access_count: 0,
				});

				const count = await storage.cleanupExpired('test-ns');

				expect(count).toBe(1);
				expect(mockData.memories.has('expired')).toBe(false);
				expect(mockData.memories.has('valid')).toBe(true);
			});

			it('should not delete working memories without TTL', async () => {
				mockData.memories.set('no-ttl', {
					id: 'no-ttl', namespace: 'test-ns', type: 'working', content: '',
					embedding: '[]', importance: 0.5, metadata: null, session_id: null,
					ttl: null, // No TTL
					created_at: Date.now() - 100000,
					updated_at: Date.now(), last_accessed: null, access_count: 0,
				});

				const count = await storage.cleanupExpired('test-ns');

				expect(count).toBe(0);
				expect(mockData.memories.has('no-ttl')).toBe(true);
			});

			it('should not delete non-working memories with TTL', async () => {
				mockData.memories.set('episodic-ttl', {
					id: 'episodic-ttl', namespace: 'test-ns', type: 'episodic', content: '',
					embedding: '[]', importance: 0.5, metadata: null, session_id: null,
					ttl: 1, // Very short TTL
					created_at: Date.now() - 100000, // Long ago
					updated_at: Date.now(), last_accessed: null, access_count: 0,
				});

				const count = await storage.cleanupExpired('test-ns');

				expect(count).toBe(0);
			});
		});

		describe('applyDecay()', () => {
			beforeEach(() => {
				mockData.memories.set('mem-1', {
					id: 'mem-1', namespace: 'test-ns', type: 'episodic', content: '',
					embedding: '[]', importance: 1.0, metadata: null, session_id: null, ttl: null,
					created_at: Date.now(), updated_at: Date.now(), last_accessed: null, access_count: 0,
				});
				mockData.memories.set('mem-2', {
					id: 'mem-2', namespace: 'test-ns', type: 'semantic', content: '',
					embedding: '[]', importance: 0.5, metadata: null, session_id: null, ttl: null,
					created_at: Date.now(), updated_at: Date.now(), last_accessed: null, access_count: 0,
				});
			});

			it('should reduce importance by decay rate', async () => {
				await storage.applyDecay('test-ns', 0.1);

				expect(mockData.memories.get('mem-1')?.importance).toBeCloseTo(0.9, 2);
				expect(mockData.memories.get('mem-2')?.importance).toBeCloseTo(0.45, 2);
			});

			it('should not reduce importance below 0.01', async () => {
				mockData.memories.get('mem-2')!.importance = 0.01;

				await storage.applyDecay('test-ns', 0.5);

				expect(mockData.memories.get('mem-2')?.importance).toBeGreaterThanOrEqual(0.01);
			});

			it('should return count of decayed memories', async () => {
				const count = await storage.applyDecay('test-ns', 0.1);

				expect(count).toBe(2);
			});

			it('should update updated_at timestamp', async () => {
				const before = mockData.memories.get('mem-1')!.updated_at;

				await new Promise(r => setTimeout(r, 10));
				await storage.applyDecay('test-ns', 0.1);

				expect(mockData.memories.get('mem-1')!.updated_at).toBeGreaterThan(before);
			});
		});
	});

	describe('API Key Operations', () => {
		describe('validateApiKey()', () => {
			beforeEach(() => {
				mockData.apiKeys.set('valid-key', {
					key: 'valid-key',
					user_id: 'user-1',
					namespace: 'default-ns',
					permissions: '["read","write"]',
					rate_limit: 100,
					rate_window: 60,
					is_active: 1,
					last_used: null,
				});
				mockData.apiKeys.set('inactive-key', {
					key: 'inactive-key',
					user_id: 'user-2',
					namespace: null,
					permissions: '["read"]',
					rate_limit: 50,
					rate_window: 60,
					is_active: 0,
					last_used: null,
				});
			});

			it('should return auth info for valid key', async () => {
				const result = await storage.validateApiKey('valid-key');

				expect(result).not.toBeNull();
				expect(result?.valid).toBe(true);
				expect(result?.userId).toBe('user-1');
				expect(result?.namespace).toBe('default-ns');
			});

			it('should parse permissions from JSON', async () => {
				const result = await storage.validateApiKey('valid-key');

				expect(result?.permissions).toEqual(['read', 'write']);
			});

			it('should include rate limit config', async () => {
				const result = await storage.validateApiKey('valid-key');

				expect(result?.rateLimit?.limit).toBe(100);
				expect(result?.rateLimit?.window).toBe(60);
			});

			it('should return null for inactive key', async () => {
				const result = await storage.validateApiKey('inactive-key');

				expect(result).toBeNull();
			});

			it('should return null for non-existent key', async () => {
				const result = await storage.validateApiKey('nonexistent');

				expect(result).toBeNull();
			});

			it('should update last_used on validation', async () => {
				await storage.validateApiKey('valid-key');

				expect(mockData.apiKeys.get('valid-key')?.last_used).toBeGreaterThan(0);
			});
		});
	});

	describe('Knowledge Memory Support', () => {
		it('should extract knowledge fields from metadata', async () => {
			mockData.memories.set('know-1', {
				id: 'know-1', namespace: 'test-ns', type: 'knowledge', content: 'Chunk content',
				embedding: '[0.1,0.2]', importance: 0.8,
				metadata: JSON.stringify({
					sourceId: 'src-123',
					sourceName: 'document.pdf',
					sourceType: 'document',
					chunkIndex: 2,
					totalChunks: 10,
				}),
				session_id: null, ttl: null,
				created_at: Date.now(), updated_at: Date.now(), last_accessed: null, access_count: 0,
			});

			const result = await storage.getMemory('test-ns', 'know-1');

			expect(result?.type).toBe('knowledge');
			expect(result?.sourceId).toBe('src-123');
			expect(result?.sourceName).toBe('document.pdf');
			expect(result?.sourceType).toBe('document');
			expect(result?.chunkIndex).toBe(2);
			expect(result?.totalChunks).toBe(10);
		});

		it('should not extract knowledge fields for non-knowledge types', async () => {
			mockData.memories.set('ep-1', {
				id: 'ep-1', namespace: 'test-ns', type: 'episodic', content: 'Episode',
				embedding: '[0.1]', importance: 0.5,
				metadata: JSON.stringify({ sourceId: 'should-not-extract' }),
				session_id: null, ttl: null,
				created_at: Date.now(), updated_at: Date.now(), last_accessed: null, access_count: 0,
			});

			const result = await storage.getMemory('test-ns', 'ep-1');

			expect(result?.type).toBe('episodic');
			expect(result?.sourceId).toBeUndefined();
		});
	});
});
