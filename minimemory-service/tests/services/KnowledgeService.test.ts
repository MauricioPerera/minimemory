/**
 * Tests for KnowledgeService
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import {
	KnowledgeService,
	type TextChunk,
} from '../../src/services/KnowledgeService.js';
import type { KnowledgeSource } from '../../src/memory/types.js';

// Mock D1Database
function createMockD1(): {
	db: ReturnType<typeof createMockD1Database>;
	sources: Map<string, Record<string, unknown>>;
} {
	const sources = new Map<string, Record<string, unknown>>();
	const db = createMockD1Database(sources);
	return { db, sources };
}

function createMockD1Database(sources: Map<string, Record<string, unknown>>) {
	return {
		prepare: vi.fn((sql: string) => {
			return {
				bind: vi.fn((...params: unknown[]) => {
					return {
						run: vi.fn(async () => {
							// INSERT
							if (sql.includes('INSERT INTO knowledge_sources')) {
								const [id, namespace, name, type, url, mimeType, size, chunkCount, metadata, createdAt, updatedAt] = params;
								sources.set(id as string, {
									id,
									namespace,
									name,
									type,
									url,
									mime_type: mimeType,
									size,
									chunk_count: chunkCount,
									metadata,
									created_at: createdAt,
									updated_at: updatedAt,
								});
								return { meta: { changes: 1 } };
							}

							// UPDATE
							if (sql.includes('UPDATE knowledge_sources')) {
								const [chunkCount, updatedAt, id] = params;
								const source = sources.get(id as string);
								if (source) {
									source.chunk_count = chunkCount;
									source.updated_at = updatedAt;
								}
								return { meta: { changes: source ? 1 : 0 } };
							}

							// DELETE
							if (sql.includes('DELETE FROM knowledge_sources')) {
								const id = params[0] as string;
								const existed = sources.has(id);
								sources.delete(id);
								return { meta: { changes: existed ? 1 : 0 } };
							}

							// DELETE memories (for cascade)
							if (sql.includes('DELETE FROM memories')) {
								return { meta: { changes: 0 } };
							}

							return { meta: { changes: 0 } };
						}),
						first: vi.fn(async <T>() => {
							// SELECT by ID
							if (sql.includes('WHERE id = ?')) {
								const id = params[0] as string;
								return sources.get(id) as T | null;
							}

							// COUNT
							if (sql.includes('COUNT(*)')) {
								return { count: sources.size, total: sources.size } as T;
							}

							// Stats query
							if (sql.includes('SUM(chunk_count)')) {
								let totalChunks = 0;
								let documents = 0;
								let urls = 0;
								let apis = 0;
								let manuals = 0;
								let totalSize = 0;

								for (const source of sources.values()) {
									totalChunks += (source.chunk_count as number) || 0;
									totalSize += (source.size as number) || 0;
									switch (source.type) {
										case 'document': documents++; break;
										case 'url': urls++; break;
										case 'api': apis++; break;
										case 'manual': manuals++; break;
									}
								}

								return {
									total_sources: sources.size,
									total_chunks: totalChunks,
									documents,
									urls,
									apis,
									manuals,
									total_size: totalSize,
								} as T;
							}

							return null;
						}),
						all: vi.fn(async () => {
							const results = Array.from(sources.values());
							return { results };
						}),
					};
				}),
			};
		}),
	} as unknown as ReturnType<typeof createMockD1>;
}

describe('KnowledgeService', () => {
	describe('chunkText', () => {
		it('should chunk text into segments', () => {
			const service = new KnowledgeService(null);
			const text = 'This is a test. Another sentence. And one more.';

			const chunks = service.chunkText(text, { chunkSize: 20, chunkOverlap: 5 });

			expect(chunks.length).toBeGreaterThan(0);
			expect(chunks[0].text).toBeTruthy();
			expect(chunks[0].index).toBe(0);
			expect(chunks[0].startOffset).toBe(0);
		});

		it('should preserve paragraph boundaries', () => {
			const service = new KnowledgeService(null);
			const text = 'First paragraph here.\n\nSecond paragraph here.\n\nThird paragraph here.';

			const chunks = service.chunkText(text, {
				chunkSize: 40,
				chunkOverlap: 0,
				preserveParagraphs: true,
			});

			// Should split at paragraph boundaries
			expect(chunks.length).toBeGreaterThanOrEqual(2);
		});

		it('should handle short text', () => {
			const service = new KnowledgeService(null);
			const text = 'Short text.';

			const chunks = service.chunkText(text, { chunkSize: 1000 });

			expect(chunks.length).toBe(1);
			expect(chunks[0].text).toBe('Short text.');
		});

		it('should handle empty text', () => {
			const service = new KnowledgeService(null);
			const text = '';

			const chunks = service.chunkText(text);

			expect(chunks.length).toBe(0);
		});

		it('should create overlapping chunks', () => {
			const service = new KnowledgeService(null);
			const text = 'Word1 Word2 Word3 Word4 Word5 Word6 Word7 Word8 Word9 Word10';

			const chunks = service.chunkText(text, {
				chunkSize: 25,
				chunkOverlap: 10,
			});

			expect(chunks.length).toBeGreaterThan(1);
			// With overlap, chunks should share some content
		});

		it('should use custom separators', () => {
			const service = new KnowledgeService(null);
			const text = 'Section1|Section2|Section3';

			const chunks = service.chunkText(text, {
				chunkSize: 10,
				chunkOverlap: 0,
				separators: ['|'],
			});

			expect(chunks.length).toBeGreaterThanOrEqual(2);
		});

		it('should respect maxChunksPerDocument', () => {
			const service = new KnowledgeService(null, { maxChunksPerDocument: 5 });
			const text = 'A'.repeat(10000); // Long text

			const chunks = service.chunkText(text, { chunkSize: 100, chunkOverlap: 0 });

			expect(chunks.length).toBeLessThanOrEqual(5);
		});
	});

	describe('isAvailable', () => {
		it('should return false when db is null', () => {
			const service = new KnowledgeService(null);

			expect(service.isAvailable()).toBe(false);
		});

		it('should return true when db is configured', () => {
			const { db } = createMockD1();
			const service = new KnowledgeService(db as any);

			expect(service.isAvailable()).toBe(true);
		});

		it('should return false when disabled', () => {
			const { db } = createMockD1();
			const service = new KnowledgeService(db as any, { enabled: false });

			expect(service.isAvailable()).toBe(false);
		});
	});

	describe('createSource', () => {
		let mockD1: ReturnType<typeof createMockD1>;
		let service: KnowledgeService;

		beforeEach(() => {
			mockD1 = createMockD1();
			service = new KnowledgeService(mockD1.db as any);
		});

		it('should create a knowledge source', async () => {
			const source = await service.createSource('default', {
				name: 'test-doc.pdf',
				type: 'document',
				chunkCount: 10,
				namespace: 'default',
				metadata: { author: 'Test' },
			});

			expect(source.id).toMatch(/^src_/);
			expect(source.name).toBe('test-doc.pdf');
			expect(source.type).toBe('document');
			expect(source.chunkCount).toBe(10);
			expect(mockD1.sources.size).toBe(1);
		});

		it('should set optional fields', async () => {
			const source = await service.createSource('default', {
				name: 'example.com/page',
				type: 'url',
				url: 'https://example.com/page',
				mimeType: 'text/html',
				size: 5000,
				chunkCount: 5,
				namespace: 'default',
				metadata: {},
			});

			expect(source.url).toBe('https://example.com/page');
			expect(source.mimeType).toBe('text/html');
			expect(source.size).toBe(5000);
		});

		it('should throw when db is null', async () => {
			const service = new KnowledgeService(null);

			await expect(service.createSource('default', {
				name: 'test',
				type: 'document',
				chunkCount: 1,
				namespace: 'default',
				metadata: {},
			})).rejects.toThrow('D1 database not configured');
		});
	});

	describe('getSource', () => {
		let mockD1: ReturnType<typeof createMockD1>;
		let service: KnowledgeService;

		beforeEach(() => {
			mockD1 = createMockD1();
			service = new KnowledgeService(mockD1.db as any);
		});

		it('should return source by id', async () => {
			// Create a source first
			const created = await service.createSource('default', {
				name: 'test.pdf',
				type: 'document',
				chunkCount: 5,
				namespace: 'default',
				metadata: {},
			});

			const source = await service.getSource(created.id);

			expect(source).not.toBeNull();
			expect(source?.name).toBe('test.pdf');
		});

		it('should return null for non-existent id', async () => {
			const source = await service.getSource('non-existent');

			expect(source).toBeNull();
		});

		it('should return null when db is null', async () => {
			const service = new KnowledgeService(null);

			const source = await service.getSource('any-id');

			expect(source).toBeNull();
		});
	});

	describe('listSources', () => {
		let mockD1: ReturnType<typeof createMockD1>;
		let service: KnowledgeService;

		beforeEach(async () => {
			mockD1 = createMockD1();
			service = new KnowledgeService(mockD1.db as any);

			// Create some sources
			await service.createSource('default', {
				name: 'doc1.pdf',
				type: 'document',
				chunkCount: 5,
				namespace: 'default',
				metadata: {},
			});
			await service.createSource('default', {
				name: 'page.html',
				type: 'url',
				chunkCount: 3,
				namespace: 'default',
				metadata: {},
			});
		});

		it('should list all sources', async () => {
			const { sources, total } = await service.listSources('default');

			expect(sources.length).toBe(2);
			expect(total).toBe(2);
		});

		it('should return empty when no db', async () => {
			const service = new KnowledgeService(null);

			const { sources, total } = await service.listSources('default');

			expect(sources).toEqual([]);
			expect(total).toBe(0);
		});
	});

	describe('deleteSource', () => {
		let mockD1: ReturnType<typeof createMockD1>;
		let service: KnowledgeService;

		beforeEach(() => {
			mockD1 = createMockD1();
			service = new KnowledgeService(mockD1.db as any);
		});

		it('should delete a source', async () => {
			const source = await service.createSource('default', {
				name: 'test.pdf',
				type: 'document',
				chunkCount: 5,
				namespace: 'default',
				metadata: {},
			});

			const deleted = await service.deleteSource(source.id);

			expect(deleted).toBe(true);
			expect(mockD1.sources.has(source.id)).toBe(false);
		});

		it('should return false for non-existent source', async () => {
			const deleted = await service.deleteSource('non-existent');

			expect(deleted).toBe(false);
		});

		it('should return false when db is null', async () => {
			const service = new KnowledgeService(null);

			const deleted = await service.deleteSource('any-id');

			expect(deleted).toBe(false);
		});
	});

	describe('updateSourceChunkCount', () => {
		let mockD1: ReturnType<typeof createMockD1>;
		let service: KnowledgeService;

		beforeEach(() => {
			mockD1 = createMockD1();
			service = new KnowledgeService(mockD1.db as any);
		});

		it('should update chunk count', async () => {
			const source = await service.createSource('default', {
				name: 'test.pdf',
				type: 'document',
				chunkCount: 5,
				namespace: 'default',
				metadata: {},
			});

			await service.updateSourceChunkCount(source.id, 10);

			const stored = mockD1.sources.get(source.id);
			expect(stored?.chunk_count).toBe(10);
		});
	});

	describe('getStats', () => {
		it('should return stats structure', async () => {
			const mockD1 = createMockD1();
			const service = new KnowledgeService(mockD1.db as any);

			const stats = await service.getStats('default');

			// Verify stats structure is correct
			expect(stats).toHaveProperty('totalSources');
			expect(stats).toHaveProperty('totalChunks');
			expect(stats).toHaveProperty('totalSize');
			expect(stats).toHaveProperty('byType');
			expect(stats.byType).toHaveProperty('document');
			expect(stats.byType).toHaveProperty('url');
			expect(stats.byType).toHaveProperty('api');
			expect(stats.byType).toHaveProperty('manual');
		});

		it('should return empty stats when no db', async () => {
			const service = new KnowledgeService(null);

			const stats = await service.getStats('default');

			expect(stats.totalSources).toBe(0);
			expect(stats.totalChunks).toBe(0);
			expect(stats.totalSize).toBe(0);
			expect(stats.byType.document).toBe(0);
			expect(stats.byType.url).toBe(0);
		});
	});
});
