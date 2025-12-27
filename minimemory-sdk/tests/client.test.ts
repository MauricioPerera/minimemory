/**
 * Tests for MiniMemoryClient
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { MiniMemoryClient, MiniMemoryError, createClient } from '../src/client.js';

// Mock fetch globally
const mockFetch = vi.fn();
global.fetch = mockFetch;

describe('MiniMemoryClient', () => {
	let client: MiniMemoryClient;

	beforeEach(() => {
		vi.clearAllMocks();
		client = createClient({
			baseUrl: 'https://test.workers.dev',
			apiKey: 'test_api_key',
			namespace: 'test-namespace',
		});
	});

	describe('constructor and configuration', () => {
		it('creates client with required config', () => {
			const c = createClient({ baseUrl: 'https://example.com' });
			expect(c).toBeInstanceOf(MiniMemoryClient);
		});

		it('strips trailing slash from baseUrl', () => {
			const c = createClient({ baseUrl: 'https://example.com/' });
			mockFetch.mockResolvedValueOnce({
				ok: true,
				json: () => Promise.resolve({ success: true, stats: {} }),
			});

			c.stats();

			expect(mockFetch).toHaveBeenCalledWith(
				'https://example.com/stats',
				expect.anything()
			);
		});

		it('setNamespace updates namespace', () => {
			client.setNamespace('new-namespace');

			mockFetch.mockResolvedValueOnce({
				ok: true,
				json: () => Promise.resolve({ success: true }),
			});

			client.stats();

			expect(mockFetch).toHaveBeenCalledWith(
				expect.anything(),
				expect.objectContaining({
					headers: expect.objectContaining({
						'X-Namespace': 'new-namespace',
					}),
				})
			);
		});

		it('setApiKey updates API key', () => {
			client.setApiKey('new_api_key');

			mockFetch.mockResolvedValueOnce({
				ok: true,
				json: () => Promise.resolve({ success: true }),
			});

			client.stats();

			expect(mockFetch).toHaveBeenCalledWith(
				expect.anything(),
				expect.objectContaining({
					headers: expect.objectContaining({
						'X-API-Key': 'new_api_key',
					}),
				})
			);
		});

		it('setAccessToken updates JWT token', () => {
			client.setAccessToken('jwt_token_123');

			mockFetch.mockResolvedValueOnce({
				ok: true,
				json: () => Promise.resolve({ success: true }),
			});

			client.stats();

			expect(mockFetch).toHaveBeenCalledWith(
				expect.anything(),
				expect.objectContaining({
					headers: expect.objectContaining({
						Authorization: 'Bearer jwt_token_123',
					}),
				})
			);
		});
	});

	describe('remember()', () => {
		it('stores a memory with content only', async () => {
			mockFetch.mockResolvedValueOnce({
				ok: true,
				json: () =>
					Promise.resolve({
						success: true,
						memory: { id: 'mem_123', type: 'semantic', content: 'test' },
						embeddingGenerated: true,
						persisted: true,
					}),
			});

			const result = await client.remember('test content');

			expect(mockFetch).toHaveBeenCalledWith(
				'https://test.workers.dev/remember',
				expect.objectContaining({
					method: 'POST',
					body: JSON.stringify({ content: 'test content' }),
				})
			);
			expect(result.success).toBe(true);
			expect(result.memory.id).toBe('mem_123');
		});

		it('stores a memory with options', async () => {
			mockFetch.mockResolvedValueOnce({
				ok: true,
				json: () =>
					Promise.resolve({
						success: true,
						memory: { id: 'mem_456', type: 'episodic', content: 'event' },
						embeddingGenerated: false,
						persisted: true,
					}),
			});

			const result = await client.remember('test event', {
				type: 'episodic',
				importance: 0.9,
				metadata: { category: 'test' },
			});

			expect(mockFetch).toHaveBeenCalledWith(
				expect.anything(),
				expect.objectContaining({
					body: JSON.stringify({
						content: 'test event',
						type: 'episodic',
						importance: 0.9,
						metadata: { category: 'test' },
					}),
				})
			);
			expect(result.memory.type).toBe('episodic');
		});
	});

	describe('recall()', () => {
		it('searches with query string', async () => {
			mockFetch.mockResolvedValueOnce({
				ok: true,
				json: () =>
					Promise.resolve({
						success: true,
						count: 2,
						embeddingGenerated: true,
						results: [
							{ id: 'mem_1', content: 'result 1', score: 0.95 },
							{ id: 'mem_2', content: 'result 2', score: 0.85 },
						],
					}),
			});

			const result = await client.recall('search query');

			expect(mockFetch).toHaveBeenCalledWith(
				'https://test.workers.dev/recall',
				expect.objectContaining({
					method: 'POST',
					body: JSON.stringify({ query: 'search query' }),
				})
			);
			expect(result.count).toBe(2);
			expect(result.results).toHaveLength(2);
		});

		it('searches with query and options', async () => {
			mockFetch.mockResolvedValueOnce({
				ok: true,
				json: () =>
					Promise.resolve({
						success: true,
						count: 1,
						embeddingGenerated: true,
						results: [{ id: 'mem_1', score: 0.9 }],
					}),
			});

			await client.recall('query', {
				type: 'semantic',
				limit: 5,
				mode: 'hybrid',
			});

			expect(mockFetch).toHaveBeenCalledWith(
				expect.anything(),
				expect.objectContaining({
					body: JSON.stringify({
						query: 'query',
						type: 'semantic',
						limit: 5,
						mode: 'hybrid',
					}),
				})
			);
		});

		it('searches with options object', async () => {
			mockFetch.mockResolvedValueOnce({
				ok: true,
				json: () =>
					Promise.resolve({
						success: true,
						count: 0,
						embeddingGenerated: false,
						results: [],
					}),
			});

			await client.recall({
				keywords: 'test keywords',
				mode: 'keyword',
				limit: 10,
			});

			expect(mockFetch).toHaveBeenCalledWith(
				expect.anything(),
				expect.objectContaining({
					body: JSON.stringify({
						keywords: 'test keywords',
						mode: 'keyword',
						limit: 10,
					}),
				})
			);
		});
	});

	describe('forget()', () => {
		it('deletes memory by ID', async () => {
			mockFetch.mockResolvedValueOnce({
				ok: true,
				json: () =>
					Promise.resolve({
						success: true,
						message: 'Memory forgotten',
					}),
			});

			const result = await client.forget('mem_123');

			expect(mockFetch).toHaveBeenCalledWith(
				'https://test.workers.dev/forget/mem_123',
				expect.objectContaining({ method: 'DELETE' })
			);
			expect(result.success).toBe(true);
		});
	});

	describe('forgetByFilter()', () => {
		it('deletes memories by filter', async () => {
			mockFetch.mockResolvedValueOnce({
				ok: true,
				json: () =>
					Promise.resolve({
						success: true,
						count: 5,
						message: 'Forgot 5 memories',
					}),
			});

			const result = await client.forgetByFilter({ type: 'working' });

			expect(mockFetch).toHaveBeenCalledWith(
				'https://test.workers.dev/forget',
				expect.objectContaining({
					method: 'POST',
					body: JSON.stringify({ filter: { type: 'working' } }),
				})
			);
			expect(result.count).toBe(5);
		});
	});

	describe('get()', () => {
		it('retrieves memory by ID', async () => {
			mockFetch.mockResolvedValueOnce({
				ok: true,
				json: () =>
					Promise.resolve({
						success: true,
						memory: { id: 'mem_123', content: 'test' },
						source: 'd1',
					}),
			});

			const result = await client.get('mem_123');

			expect(mockFetch).toHaveBeenCalledWith(
				'https://test.workers.dev/memory/mem_123',
				expect.objectContaining({ method: 'GET' })
			);
			expect(result.memory.id).toBe('mem_123');
		});
	});

	describe('update()', () => {
		it('updates memory fields', async () => {
			mockFetch.mockResolvedValueOnce({
				ok: true,
				json: () =>
					Promise.resolve({
						success: true,
						memory: { id: 'mem_123', content: 'updated', importance: 0.9 },
					}),
			});

			const result = await client.update('mem_123', {
				content: 'updated',
				importance: 0.9,
			});

			expect(mockFetch).toHaveBeenCalledWith(
				'https://test.workers.dev/memory/mem_123',
				expect.objectContaining({
					method: 'PATCH',
					body: JSON.stringify({ content: 'updated', importance: 0.9 }),
				})
			);
			expect(result.memory.content).toBe('updated');
		});
	});

	describe('stats()', () => {
		it('retrieves namespace statistics', async () => {
			mockFetch.mockResolvedValueOnce({
				ok: true,
				json: () =>
					Promise.resolve({
						success: true,
						namespace: 'test-namespace',
						stats: {
							total: 100,
							byType: { episodic: 50, semantic: 30, working: 20, knowledge: 0 },
							averageImportance: 0.7,
						},
						source: 'd1',
					}),
			});

			const result = await client.stats();

			expect(result.stats.total).toBe(100);
			expect(result.stats.byType.episodic).toBe(50);
		});
	});

	describe('cleanup()', () => {
		it('cleans up expired memories', async () => {
			mockFetch.mockResolvedValueOnce({
				ok: true,
				json: () =>
					Promise.resolve({
						success: true,
						count: 10,
						message: 'Cleaned up 10 expired memories',
					}),
			});

			const result = await client.cleanup();

			expect(mockFetch).toHaveBeenCalledWith(
				'https://test.workers.dev/cleanup',
				expect.objectContaining({ method: 'POST' })
			);
			expect(result.count).toBe(10);
		});
	});

	describe('export()', () => {
		it('exports all memories', async () => {
			mockFetch.mockResolvedValueOnce({
				ok: true,
				json: () =>
					Promise.resolve({
						success: true,
						namespace: 'test-namespace',
						data: { memories: [{ id: 'mem_1' }, { id: 'mem_2' }] },
						source: 'd1',
					}),
			});

			const result = await client.export();

			expect(result.data.memories).toHaveLength(2);
		});
	});

	describe('import()', () => {
		it('imports memories', async () => {
			mockFetch.mockResolvedValueOnce({
				ok: true,
				json: () =>
					Promise.resolve({
						success: true,
						count: 5,
						message: 'Imported 5 memories',
					}),
			});

			const memories = [
				{ id: 'mem_1', type: 'semantic' as const, content: 'test 1', importance: 0.5, metadata: {}, createdAt: Date.now(), updatedAt: Date.now(), accessedAt: Date.now(), accessCount: 0 },
				{ id: 'mem_2', type: 'semantic' as const, content: 'test 2', importance: 0.5, metadata: {}, createdAt: Date.now(), updatedAt: Date.now(), accessedAt: Date.now(), accessCount: 0 },
			];

			const result = await client.import(memories);

			expect(result.count).toBe(5);
		});
	});

	describe('clear()', () => {
		it('clears all memories', async () => {
			mockFetch.mockResolvedValueOnce({
				ok: true,
				json: () =>
					Promise.resolve({
						success: true,
						message: 'All memories cleared',
					}),
			});

			const result = await client.clear();

			expect(mockFetch).toHaveBeenCalledWith(
				'https://test.workers.dev/clear',
				expect.objectContaining({ method: 'DELETE' })
			);
			expect(result.success).toBe(true);
		});
	});

	describe('knowledge operations', () => {
		it('ingests document', async () => {
			mockFetch.mockResolvedValueOnce({
				ok: true,
				json: () =>
					Promise.resolve({
						success: true,
						sourceId: 'src_123',
						sourceName: 'test.md',
						chunksCreated: 10,
						embeddingsGenerated: true,
						totalCharacters: 5000,
						averageChunkSize: 500,
						durationMs: 1500,
					}),
			});

			const result = await client.knowledge.ingest({
				content: 'Document content...',
				name: 'test.md',
				type: 'document',
			});

			expect(mockFetch).toHaveBeenCalledWith(
				'https://test.workers.dev/knowledge/ingest',
				expect.objectContaining({ method: 'POST' })
			);
			expect(result.sourceId).toBe('src_123');
			expect(result.chunksCreated).toBe(10);
		});

		it('lists sources', async () => {
			mockFetch.mockResolvedValueOnce({
				ok: true,
				json: () =>
					Promise.resolve({
						success: true,
						sources: [{ id: 'src_1', name: 'doc1.md' }],
						total: 1,
						hasMore: false,
					}),
			});

			const result = await client.knowledge.listSources({ limit: 10 });

			expect(mockFetch).toHaveBeenCalledWith(
				'https://test.workers.dev/knowledge/sources?limit=10',
				expect.anything()
			);
			expect(result.sources).toHaveLength(1);
		});

		it('gets source by ID', async () => {
			mockFetch.mockResolvedValueOnce({
				ok: true,
				json: () =>
					Promise.resolve({
						success: true,
						source: { id: 'src_123', name: 'test.md' },
					}),
			});

			const result = await client.knowledge.getSource('src_123');

			expect(result.source.id).toBe('src_123');
		});

		it('deletes source', async () => {
			mockFetch.mockResolvedValueOnce({
				ok: true,
				json: () =>
					Promise.resolve({
						success: true,
						message: 'Source deleted',
					}),
			});

			const result = await client.knowledge.deleteSource('src_123');

			expect(mockFetch).toHaveBeenCalledWith(
				'https://test.workers.dev/knowledge/sources/src_123',
				expect.objectContaining({ method: 'DELETE' })
			);
			expect(result.success).toBe(true);
		});

		it('gets chunks for source', async () => {
			mockFetch.mockResolvedValueOnce({
				ok: true,
				json: () =>
					Promise.resolve({
						success: true,
						source: { id: 'src_123', name: 'test.md', type: 'document' },
						chunks: [{ id: 'chunk_1', content: 'text', chunkIndex: 0 }],
						total: 1,
						hasMore: false,
					}),
			});

			const result = await client.knowledge.getChunks('src_123');

			expect(result.chunks).toHaveLength(1);
		});

		it('gets knowledge stats', async () => {
			mockFetch.mockResolvedValueOnce({
				ok: true,
				json: () =>
					Promise.resolve({
						success: true,
						namespace: 'test-namespace',
						stats: { totalSources: 5, totalChunks: 100 },
					}),
			});

			const result = await client.knowledge.stats();

			expect(result.stats.totalSources).toBe(5);
		});

		it('previews chunking', async () => {
			mockFetch.mockResolvedValueOnce({
				ok: true,
				json: () =>
					Promise.resolve({
						success: true,
						totalChunks: 3,
						totalCharacters: 3000,
						averageChunkSize: 1000,
						chunks: [
							{ index: 0, length: 1000, preview: 'First...' },
							{ index: 1, length: 1000, preview: 'Second...' },
							{ index: 2, length: 1000, preview: 'Third...' },
						],
					}),
			});

			const result = await client.knowledge.previewChunking('Content...', {
				chunkSize: 1000,
			});

			expect(result.totalChunks).toBe(3);
		});
	});

	describe('embed operations', () => {
		it('generates single embedding', async () => {
			mockFetch.mockResolvedValueOnce({
				ok: true,
				json: () =>
					Promise.resolve({
						success: true,
						embedding: [0.1, 0.2, 0.3],
						dimensions: 768,
						model: '@cf/google/gemma-embedding-300m',
					}),
			});

			const result = await client.embed.single('Test text');

			expect(mockFetch).toHaveBeenCalledWith(
				'https://test.workers.dev/embed',
				expect.objectContaining({
					method: 'POST',
					body: JSON.stringify({ text: 'Test text' }),
				})
			);
			expect(result.embedding).toHaveLength(3);
		});

		it('generates batch embeddings', async () => {
			mockFetch.mockResolvedValueOnce({
				ok: true,
				json: () =>
					Promise.resolve({
						success: true,
						embeddings: [[0.1], [0.2]],
						dimensions: 768,
						model: '@cf/google/gemma-embedding-300m',
						count: 2,
					}),
			});

			const result = await client.embed.batch(['Text 1', 'Text 2'], {
				dimensions: 256,
			});

			expect(result.embeddings).toHaveLength(2);
			expect(result.count).toBe(2);
		});

		it('gets embed info', async () => {
			mockFetch.mockResolvedValueOnce({
				ok: true,
				json: () =>
					Promise.resolve({
						available: true,
						model: '@cf/google/gemma-embedding-300m',
						dimensions: { default: 768, available: [768, 512, 256, 128] },
						matryoshka: true,
					}),
			});

			const result = await client.embed.info();

			expect(result.available).toBe(true);
			expect(result.model).toBe('@cf/google/gemma-embedding-300m');
		});
	});

	describe('error handling', () => {
		it('throws MiniMemoryError on HTTP error', async () => {
			mockFetch.mockResolvedValueOnce({
				ok: false,
				status: 404,
				json: () => Promise.resolve({ error: 'Memory not found' }),
			});

			try {
				await client.get('invalid_id');
				expect.fail('Should have thrown');
			} catch (error) {
				expect(error).toBeInstanceOf(MiniMemoryError);
				expect((error as MiniMemoryError).message).toBe('Memory not found');
				expect((error as MiniMemoryError).status).toBe(404);
			}
		});

		it('throws MiniMemoryError on network error', async () => {
			mockFetch.mockRejectedValueOnce(new Error('Network error'));

			await expect(client.stats()).rejects.toThrow(MiniMemoryError);
		});

		it('throws MiniMemoryError on timeout', async () => {
			// Mock AbortController behavior
			mockFetch.mockImplementationOnce(
				(_url, options) =>
					new Promise((_resolve, reject) => {
						const signal = options?.signal as AbortSignal | undefined;
						if (signal) {
							signal.addEventListener('abort', () => {
								const error = new Error('Aborted');
								error.name = 'AbortError';
								reject(error);
							});
						}
					})
			);

			const clientWithShortTimeout = createClient({
				baseUrl: 'https://test.workers.dev',
				timeout: 10, // Very short timeout for test
			});

			try {
				await clientWithShortTimeout.stats();
				expect.fail('Should have thrown');
			} catch (error) {
				expect(error).toBeInstanceOf(MiniMemoryError);
				expect((error as MiniMemoryError).code).toBe('TIMEOUT');
			}
		});
	});
});

describe('createClient()', () => {
	it('creates a configured client', () => {
		const client = createClient({
			baseUrl: 'https://api.example.com',
			apiKey: 'test_key',
			namespace: 'prod',
			timeout: 5000,
		});

		expect(client).toBeInstanceOf(MiniMemoryClient);
	});
});
