/**
 * Memory API routes with D1 persistence and Workers AI embedding support
 */

import { Hono } from 'hono';
import type { D1Database } from '@cloudflare/workers-types';
import type { MemoryManager } from '../memory/MemoryManager.js';
import type { RememberOptions, RecallOptions } from '../memory/types.js';
import { D1Storage } from '../storage/index.js';
import { EmbeddingService, type EmbeddingDimensions, createAuditLogger, createWebhookTrigger } from '../services/index.js';
import type { ExecutionContext } from '@cloudflare/workers-types';

// Workers AI binding type
interface Ai {
	run(model: string, inputs: Record<string, unknown>): Promise<unknown>;
}

type Bindings = {
	DB?: D1Database;
	AI?: Ai;
};

type Variables = {
	executionCtx?: ExecutionContext;
};

// Audit context from request headers/auth
interface AuditContext {
	userId?: string;
	tenantId?: string;
	apiKey?: string;
	ipAddress?: string;
	userAgent?: string;
	requestId?: string;
}

// Default dimensions for EmbeddingGemma (Matryoshka-compatible)
const DEFAULT_EMBEDDING_DIMS: EmbeddingDimensions = 768;

export function createMemoryRoutes(getManager: (namespace: string, dimensions?: number) => MemoryManager) {
	const api = new Hono<{ Bindings: Bindings; Variables: Variables }>();

	/**
	 * Helper to get D1Storage if available
	 */
	function getStorage(c: { env?: Bindings }): D1Storage | null {
		return c.env?.DB ? new D1Storage(c.env.DB) : null;
	}

	/**
	 * Helper to extract audit context from request
	 */
	function getAuditContext(c: { req: { header: (name: string) => string | undefined } }): AuditContext {
		return {
			userId: c.req.header('X-User-Id'),
			tenantId: c.req.header('X-Tenant-Id'),
			apiKey: c.req.header('X-API-Key'),
			ipAddress: c.req.header('CF-Connecting-IP') || c.req.header('X-Forwarded-For'),
			userAgent: c.req.header('User-Agent'),
			requestId: c.req.header('X-Request-Id') || crypto.randomUUID(),
		};
	}

	/**
	 * Helper to create audit logger for request
	 */
	function getAuditLogger(c: { env?: Bindings; req: { header: (name: string) => string | undefined } }, namespace: string) {
		if (!c.env?.DB) return null;
		const context = getAuditContext(c);
		return createAuditLogger(c.env.DB, { ...context, namespace });
	}

	/**
	 * Helper to get webhook trigger for firing events
	 */
	function getWebhookTrigger(c: { env?: Bindings; executionCtx?: ExecutionContext }) {
		if (!c.env?.DB) return null;
		return createWebhookTrigger(c.env.DB, c.executionCtx);
	}

	/**
	 * Helper to get EmbeddingService if AI binding is available
	 */
	function getEmbeddingService(c: { env?: Bindings }, dimensions?: EmbeddingDimensions): EmbeddingService | null {
		if (!c.env?.AI) return null;
		return new EmbeddingService(c.env.AI, {
			defaultDimensions: dimensions || DEFAULT_EMBEDDING_DIMS,
		});
	}

	/**
	 * Helper to load namespace dimensions from D1
	 */
	async function getNamespaceDimensions(storage: D1Storage | null, namespace: string): Promise<number | undefined> {
		if (!storage) return undefined;
		const ns = await storage.getNamespace(namespace);
		return ns?.dimensions;
	}

	/**
	 * POST /remember - Store a new memory
	 *
	 * If embedding is not provided and AI binding is available,
	 * automatically generates embedding using EmbeddingGemma.
	 */
	api.post('/remember', async (c) => {
		const startTime = Date.now();
		const namespace = c.req.header('X-Namespace') || 'default';
		const auditLogger = getAuditLogger(c, namespace);

		try {
			const body = await c.req.json();
			const storage = getStorage(c);

			const { content, embedding, type, importance, metadata, sessionId, ttl, generateEmbedding } = body;

			if (!content || typeof content !== 'string') {
				return c.json({ error: 'content is required and must be a string' }, 400);
			}

			// Get dimensions from namespace config or default
			const namespaceDims = await getNamespaceDimensions(storage, namespace);
			const dimensions = namespaceDims || DEFAULT_EMBEDDING_DIMS;

			let vectorEmbedding: number[];
			let embeddingGenerated = false;

			if (embedding && Array.isArray(embedding) && embedding.length > 0) {
				// Use provided embedding
				vectorEmbedding = embedding;
			} else if (generateEmbedding !== false) {
				// Try to generate embedding using Workers AI
				const embeddingService = getEmbeddingService(c, dimensions as EmbeddingDimensions);
				if (embeddingService) {
					try {
						const result = await embeddingService.embed(content, {
							dimensions: dimensions as EmbeddingDimensions,
						});
						vectorEmbedding = result.embedding;
						embeddingGenerated = true;
					} catch (embError) {
						console.error('Failed to generate embedding:', embError);
						// Fall back to zero vector if AI fails
						vectorEmbedding = new Array(dimensions).fill(0);
					}
				} else {
					// No AI binding - use zero vector for keyword-only search
					vectorEmbedding = new Array(dimensions).fill(0);
				}
			} else {
				// generateEmbedding explicitly set to false
				vectorEmbedding = new Array(dimensions).fill(0);
			}

			const manager = getManager(namespace, dimensions);
			const options: RememberOptions = { type, importance, metadata, sessionId, ttl };

			const memory = await manager.remember(content, vectorEmbedding, options);

			// Persist to D1 if available
			if (storage) {
				await storage.saveMemory({
					id: memory.id,
					namespace,
					type: memory.type,
					content: memory.content,
					embedding: vectorEmbedding,
					importance: memory.importance,
					metadata: memory.metadata || {},
					sessionId: sessionId,
					ttl: ttl,
					createdAt: memory.createdAt,
					updatedAt: memory.createdAt,
					accessCount: 0,
				});
			}

			// Audit log
			await auditLogger?.logMemory('create', memory.id, {
				type: memory.type,
				importance: memory.importance,
				embeddingGenerated,
				contentLength: content.length,
			}, { durationMs: Date.now() - startTime });

			// Trigger webhook
			const tenantId = c.req.header('X-Tenant-Id');
			const webhookTrigger = getWebhookTrigger(c);
			webhookTrigger?.(namespace, 'memory.remembered', {
				memoryId: memory.id,
				type: memory.type,
				content: memory.content,
				importance: memory.importance,
				metadata: memory.metadata,
			}, tenantId);

			return c.json({
				success: true,
				memory: {
					id: memory.id,
					type: memory.type,
					content: memory.content,
					importance: memory.importance,
					createdAt: memory.createdAt,
				},
				embeddingGenerated,
				persisted: !!storage,
			});
		} catch (error) {
			const errorMessage = error instanceof Error ? error.message : 'Unknown error';
			await auditLogger?.logMemory('create', undefined, { error: errorMessage }, {
				success: false,
				errorMessage,
				durationMs: Date.now() - startTime,
			});
			return c.json({ error: errorMessage }, 500);
		}
	});

	/**
	 * POST /recall - Search for memories
	 *
	 * Supports:
	 * - query: text to search (generates embedding via Workers AI)
	 * - keywords: text for keyword search (BM25)
	 * - embedding: pre-computed vector for similarity search
	 * - mode: 'vector' | 'keyword' | 'hybrid'
	 */
	api.post('/recall', async (c) => {
		const startTime = Date.now();
		const namespace = c.req.header('X-Namespace') || 'default';
		const auditLogger = getAuditLogger(c, namespace);

		try {
			const body = await c.req.json();
			const storage = getStorage(c);

			const { query, keywords, embedding, type, limit, minImportance, minSimilarity, sessionId, mode, alpha } = body;

			// Get dimensions from D1 if available
			const namespaceDims = await getNamespaceDimensions(storage, namespace);
			const dimensions = namespaceDims || DEFAULT_EMBEDDING_DIMS;

			// Generate embedding from query if provided and no embedding given
			let queryEmbedding = embedding;
			let embeddingGenerated = false;

			if (!embedding && query) {
				const embeddingService = getEmbeddingService(c, dimensions as EmbeddingDimensions);
				if (embeddingService) {
					try {
						const result = await embeddingService.embed(query, {
							dimensions: dimensions as EmbeddingDimensions,
						});
						queryEmbedding = result.embedding;
						embeddingGenerated = true;
					} catch (embError) {
						console.error('Failed to generate query embedding:', embError);
						// Continue without embedding (keyword-only search)
					}
				}
			}

			// Determine search keywords
			const searchKeywords = keywords || query;

			if (!searchKeywords && !queryEmbedding) {
				return c.json({ error: 'Either query, keywords, or embedding is required' }, 400);
			}

			const manager = getManager(namespace, dimensions);

			// Load memories from D1 if manager is empty and D1 is available
			const currentStats = await manager.stats();
			if (storage && currentStats.total === 0) {
				const storedMemories = await storage.getAllMemories(namespace);
				if (storedMemories.length > 0) {
					manager.import({
						memories: storedMemories.map(m => ({
							id: m.id,
							type: m.type,
							content: m.content,
							embedding: m.embedding,
							importance: m.importance,
							metadata: m.metadata,
							createdAt: m.createdAt,
							updatedAt: m.updatedAt,
							accessedAt: m.lastAccessed || m.createdAt,
							accessCount: m.accessCount,
						})),
					});
				}
			}

			const options: RecallOptions = {
				type,
				limit,
				minImportance,
				minSimilarity,
				sessionId,
				mode,
				alpha,
			};

			let results;
			if (searchKeywords && queryEmbedding) {
				// Hybrid search with both
				results = await manager.recallWithEmbedding(searchKeywords, queryEmbedding, options);
			} else if (queryEmbedding) {
				options.mode = 'vector';
				results = await manager.recall(queryEmbedding, options);
			} else {
				options.mode = 'keyword';
				results = await manager.recall(searchKeywords, options);
			}

			// Audit log
			await auditLogger?.logMemory('search', undefined, {
				mode: mode || (queryEmbedding && searchKeywords ? 'hybrid' : queryEmbedding ? 'vector' : 'keyword'),
				resultCount: results.length,
				embeddingGenerated,
				limit,
			}, { durationMs: Date.now() - startTime });

			// Map results with source citation for knowledge memories
			const mappedResults = results.map(r => {
				const result: Record<string, unknown> = {
					id: r.memory.id,
					type: r.memory.type,
					content: r.memory.content,
					score: r.score,
					vectorSimilarity: r.vectorSimilarity,
					keywordScore: r.keywordScore,
					importance: r.memory.importance,
					metadata: r.memory.metadata,
					createdAt: r.memory.createdAt,
				};

				// Add source citation for knowledge memories
				if (r.memory.type === 'knowledge' && r.memory.metadata) {
					const meta = r.memory.metadata as Record<string, unknown>;
					if (meta.sourceId) {
						result.source = {
							id: meta.sourceId,
							name: meta.sourceName,
							type: meta.sourceType,
							url: meta.sourceUrl,
							chunkIndex: meta.chunkIndex,
							totalChunks: meta.totalChunks,
						};
					}
				}

				return result;
			});

			return c.json({
				success: true,
				count: results.length,
				embeddingGenerated,
				results: mappedResults,
			});
		} catch (error) {
			const errorMessage = error instanceof Error ? error.message : 'Unknown error';
			await auditLogger?.logMemory('search', undefined, { error: errorMessage }, {
				success: false,
				errorMessage,
				durationMs: Date.now() - startTime,
			});
			return c.json({ error: errorMessage }, 500);
		}
	});

	/**
	 * DELETE /forget/:id - Delete a specific memory
	 */
	api.delete('/forget/:id', async (c) => {
		const startTime = Date.now();
		const id = c.req.param('id');
		const namespace = c.req.header('X-Namespace') || 'default';
		const auditLogger = getAuditLogger(c, namespace);

		try {
			const storage = getStorage(c);

			const manager = getManager(namespace);
			const deleted = await manager.forget(id);

			// Also delete from D1
			if (storage) {
				await storage.deleteMemory(namespace, id);
			}

			// Audit log
			await auditLogger?.logMemory('delete', id, { deleted }, { durationMs: Date.now() - startTime });

			// Trigger webhook if memory was deleted
			if (deleted) {
				const tenantId = c.req.header('X-Tenant-Id');
				const webhookTrigger = getWebhookTrigger(c);
				webhookTrigger?.(namespace, 'memory.forgotten', {
					memoryId: id,
				}, tenantId);
			}

			return c.json({
				success: deleted,
				message: deleted ? 'Memory forgotten' : 'Memory not found',
			});
		} catch (error) {
			const errorMessage = error instanceof Error ? error.message : 'Unknown error';
			await auditLogger?.logMemory('delete', id, { error: errorMessage }, {
				success: false,
				errorMessage,
				durationMs: Date.now() - startTime,
			});
			return c.json({ error: errorMessage }, 500);
		}
	});

	/**
	 * POST /forget - Delete memories by filter
	 */
	api.post('/forget', async (c) => {
		const startTime = Date.now();
		const namespace = c.req.header('X-Namespace') || 'default';
		const auditLogger = getAuditLogger(c, namespace);

		try {
			const body = await c.req.json();
			const storage = getStorage(c);

			const { filter } = body;

			if (!filter || typeof filter !== 'object') {
				return c.json({ error: 'filter is required and must be an object' }, 400);
			}

			const manager = getManager(namespace);
			const count = await manager.forgetByFilter(filter);

			// Also delete from D1 by type if specified
			if (storage && filter.type) {
				await storage.deleteMemoriesByType(namespace, filter.type);
			}

			// Audit log
			await auditLogger?.logMemory('delete', undefined, { filter, count }, { durationMs: Date.now() - startTime });

			return c.json({
				success: true,
				count,
				message: `Forgot ${count} memories`,
			});
		} catch (error) {
			const errorMessage = error instanceof Error ? error.message : 'Unknown error';
			await auditLogger?.logMemory('delete', undefined, { error: errorMessage }, {
				success: false,
				errorMessage,
				durationMs: Date.now() - startTime,
			});
			return c.json({ error: errorMessage }, 500);
		}
	});

	/**
	 * GET /memory/:id - Get a specific memory
	 */
	api.get('/memory/:id', async (c) => {
		const startTime = Date.now();
		const id = c.req.param('id');
		const namespace = c.req.header('X-Namespace') || 'default';
		const auditLogger = getAuditLogger(c, namespace);

		try {
			const storage = getStorage(c);

			// Try D1 first if available
			if (storage) {
				const stored = await storage.getMemory(namespace, id);
				if (stored) {
					// Audit log
					await auditLogger?.logMemory('read', id, { source: 'd1' }, { durationMs: Date.now() - startTime });

					return c.json({
						success: true,
						memory: {
							id: stored.id,
							type: stored.type,
							content: stored.content,
							importance: stored.importance,
							metadata: stored.metadata,
							createdAt: stored.createdAt,
							lastAccessed: stored.lastAccessed,
							accessCount: stored.accessCount,
						},
						source: 'd1',
					});
				}
			}

			// Fallback to memory
			const manager = getManager(namespace);
			const memory = await manager.get(id);

			if (!memory) {
				return c.json({ error: 'Memory not found' }, 404);
			}

			// Audit log
			await auditLogger?.logMemory('read', id, { source: 'memory' }, { durationMs: Date.now() - startTime });

			return c.json({
				success: true,
				memory,
				source: 'memory',
			});
		} catch (error) {
			const errorMessage = error instanceof Error ? error.message : 'Unknown error';
			await auditLogger?.logMemory('read', id, { error: errorMessage }, {
				success: false,
				errorMessage,
				durationMs: Date.now() - startTime,
			});
			return c.json({ error: errorMessage }, 500);
		}
	});

	/**
	 * PATCH /memory/:id - Update a memory
	 */
	api.patch('/memory/:id', async (c) => {
		const startTime = Date.now();
		const id = c.req.param('id');
		const namespace = c.req.header('X-Namespace') || 'default';
		const auditLogger = getAuditLogger(c, namespace);

		try {
			const body = await c.req.json();
			const storage = getStorage(c);

			const { content, importance, metadata, embedding } = body;

			const manager = getManager(namespace);
			const memory = await manager.update(id, { content, importance, metadata }, embedding);

			if (!memory) {
				return c.json({ error: 'Memory not found' }, 404);
			}

			// Update in D1 if available
			if (storage) {
				await storage.updateMemory(namespace, id, {
					content: memory.content,
					importance: memory.importance,
					metadata: memory.metadata,
					embedding: embedding,
				});
			}

			// Audit log
			const updatedFields = Object.keys(body).filter(k => body[k] !== undefined);
			await auditLogger?.logMemory('update', id, {
				updatedFields,
			}, { durationMs: Date.now() - startTime });

			// Trigger webhook
			const tenantId = c.req.header('X-Tenant-Id');
			const webhookTrigger = getWebhookTrigger(c);
			webhookTrigger?.(namespace, 'memory.updated', {
				memoryId: id,
				updatedFields,
				content: memory.content,
				importance: memory.importance,
				metadata: memory.metadata,
			}, tenantId);

			return c.json({
				success: true,
				memory,
			});
		} catch (error) {
			const errorMessage = error instanceof Error ? error.message : 'Unknown error';
			await auditLogger?.logMemory('update', id, { error: errorMessage }, {
				success: false,
				errorMessage,
				durationMs: Date.now() - startTime,
			});
			return c.json({ error: errorMessage }, 500);
		}
	});

	/**
	 * GET /stats - Get memory statistics
	 */
	api.get('/stats', async (c) => {
		try {
			const namespace = c.req.header('X-Namespace') || 'default';
			const storage = getStorage(c);

			// Use D1 stats if available (more accurate for persistent data)
			if (storage) {
				const stats = await storage.getStats(namespace);
				return c.json({
					success: true,
					namespace,
					stats,
					source: 'd1',
				});
			}

			// Fallback to memory stats
			const manager = getManager(namespace);
			const stats = await manager.stats();

			return c.json({
				success: true,
				namespace,
				stats,
				source: 'memory',
			});
		} catch (error) {
			return c.json({
				error: error instanceof Error ? error.message : 'Unknown error',
			}, 500);
		}
	});

	/**
	 * POST /cleanup - Clean up expired working memories
	 */
	api.post('/cleanup', async (c) => {
		try {
			const namespace = c.req.header('X-Namespace') || 'default';
			const storage = getStorage(c);

			const manager = getManager(namespace);
			let count = await manager.cleanupExpired();

			// Also cleanup in D1
			if (storage) {
				const d1Count = await storage.cleanupExpired(namespace);
				count = Math.max(count, d1Count);
			}

			return c.json({
				success: true,
				count,
				message: `Cleaned up ${count} expired memories`,
			});
		} catch (error) {
			return c.json({
				error: error instanceof Error ? error.message : 'Unknown error',
			}, 500);
		}
	});

	/**
	 * POST /decay - Apply importance decay
	 */
	api.post('/decay', async (c) => {
		try {
			const namespace = c.req.header('X-Namespace') || 'default';
			const storage = getStorage(c);

			const manager = getManager(namespace);
			await manager.applyDecay();

			// Also apply decay in D1
			if (storage) {
				await storage.applyDecay(namespace);
			}

			return c.json({
				success: true,
				message: 'Decay applied successfully',
			});
		} catch (error) {
			return c.json({
				error: error instanceof Error ? error.message : 'Unknown error',
			}, 500);
		}
	});

	/**
	 * POST /export - Export all memories
	 */
	api.post('/export', async (c) => {
		const startTime = Date.now();
		const namespace = c.req.header('X-Namespace') || 'default';
		const auditLogger = getAuditLogger(c, namespace);

		try {
			const storage = getStorage(c);

			// Export from D1 if available
			if (storage) {
				const memories = await storage.getAllMemories(namespace);

				// Audit log
				await auditLogger?.logBulk('export', {
					count: memories.length,
					source: 'd1',
				}, { durationMs: Date.now() - startTime });

				return c.json({
					success: true,
					namespace,
					data: { memories },
					source: 'd1',
				});
			}

			// Fallback to memory export
			const manager = getManager(namespace);
			const data = manager.export();

			// Audit log
			await auditLogger?.logBulk('export', {
				count: data.memories?.length || 0,
				source: 'memory',
			}, { durationMs: Date.now() - startTime });

			return c.json({
				success: true,
				namespace,
				data,
				source: 'memory',
			});
		} catch (error) {
			const errorMessage = error instanceof Error ? error.message : 'Unknown error';
			await auditLogger?.logBulk('export', { error: errorMessage }, {
				success: false,
				errorMessage,
				durationMs: Date.now() - startTime,
			});
			return c.json({ error: errorMessage }, 500);
		}
	});

	/**
	 * POST /import - Import memories
	 */
	api.post('/import', async (c) => {
		const startTime = Date.now();
		const namespace = c.req.header('X-Namespace') || 'default';
		const auditLogger = getAuditLogger(c, namespace);

		try {
			const body = await c.req.json();
			const storage = getStorage(c);

			const { memories } = body;

			if (!memories || !Array.isArray(memories)) {
				return c.json({ error: 'memories array is required' }, 400);
			}

			const manager = getManager(namespace);
			const count = manager.import({ memories });

			// Also import to D1
			if (storage) {
				for (const mem of memories) {
					await storage.saveMemory({
						id: mem.id,
						namespace,
						type: mem.type || 'semantic',
						content: mem.content,
						embedding: mem.embedding,
						importance: mem.importance || 0.5,
						metadata: mem.metadata || {},
						sessionId: mem.sessionId,
						ttl: mem.ttl,
						createdAt: mem.createdAt || Date.now(),
						updatedAt: Date.now(),
						accessCount: 0,
					});
				}
			}

			// Audit log
			await auditLogger?.logBulk('import', { count }, { durationMs: Date.now() - startTime });

			return c.json({
				success: true,
				count,
				message: `Imported ${count} memories`,
			});
		} catch (error) {
			const errorMessage = error instanceof Error ? error.message : 'Unknown error';
			await auditLogger?.logBulk('import', { error: errorMessage }, {
				success: false,
				errorMessage,
				durationMs: Date.now() - startTime,
			});
			return c.json({ error: errorMessage }, 500);
		}
	});

	/**
	 * DELETE /clear - Clear all memories
	 */
	api.delete('/clear', async (c) => {
		const startTime = Date.now();
		const namespace = c.req.header('X-Namespace') || 'default';
		const auditLogger = getAuditLogger(c, namespace);

		try {
			const storage = getStorage(c);

			const manager = getManager(namespace);
			manager.clear();

			// Also clear D1
			if (storage) {
				await storage.clearNamespace(namespace);
			}

			// Audit log
			await auditLogger?.logBulk('clear', { namespace }, { durationMs: Date.now() - startTime });

			return c.json({
				success: true,
				message: 'All memories cleared',
			});
		} catch (error) {
			const errorMessage = error instanceof Error ? error.message : 'Unknown error';
			await auditLogger?.logBulk('clear', { error: errorMessage }, {
				success: false,
				errorMessage,
				durationMs: Date.now() - startTime,
			});
			return c.json({ error: errorMessage }, 500);
		}
	});

	/**
	 * POST /embed - Generate embeddings using Workers AI
	 *
	 * Uses EmbeddingGemma-300m (768 dimensions, Matryoshka-compatible)
	 */
	api.post('/embed', async (c) => {
		try {
			const body = await c.req.json();
			const { text, texts, dimensions } = body;

			// Validate input
			const inputTexts = texts || (text ? [text] : null);
			if (!inputTexts || !Array.isArray(inputTexts) || inputTexts.length === 0) {
				return c.json({ error: 'text or texts array is required' }, 400);
			}

			// Validate dimensions
			const targetDims = (dimensions || DEFAULT_EMBEDDING_DIMS) as EmbeddingDimensions;
			if (![768, 512, 256, 128].includes(targetDims)) {
				return c.json({ error: 'dimensions must be one of: 768, 512, 256, 128' }, 400);
			}

			const embeddingService = getEmbeddingService(c, targetDims);
			if (!embeddingService) {
				return c.json({ error: 'AI binding not available. Configure AI in wrangler.toml' }, 503);
			}

			// Generate embeddings
			if (inputTexts.length === 1) {
				const result = await embeddingService.embed(inputTexts[0], { dimensions: targetDims });
				return c.json({
					success: true,
					embedding: result.embedding,
					dimensions: result.dimensions,
					model: result.model,
					truncated: result.truncated,
				});
			} else {
				const result = await embeddingService.embedBatch(inputTexts, { dimensions: targetDims });
				return c.json({
					success: true,
					embeddings: result.embeddings,
					dimensions: result.dimensions,
					model: result.model,
					count: result.count,
				});
			}
		} catch (error) {
			return c.json({
				error: error instanceof Error ? error.message : 'Unknown error',
			}, 500);
		}
	});

	/**
	 * GET /embed/info - Get embedding service info
	 */
	api.get('/embed/info', (c) => {
		const hasAI = !!c.env?.AI;
		return c.json({
			available: hasAI,
			model: '@cf/google/gemma-embedding-300m',
			dimensions: {
				default: 768,
				available: [768, 512, 256, 128],
			},
			matryoshka: true,
			pricing: {
				perThousandNeurons: 0.011,
				freeDaily: 10000,
			},
			estimatedCosts: {
				'10K embeddings (768d)': EmbeddingService.estimateCost(10000, 768).toFixed(2),
				'10K embeddings (256d)': EmbeddingService.estimateCost(10000, 256).toFixed(2),
			},
		});
	});

	return api;
}
