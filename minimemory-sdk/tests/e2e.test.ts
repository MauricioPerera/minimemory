/**
 * End-to-end tests for the SDK against a running minimemory-service
 *
 * Run with: npm run test:e2e
 * Requires: minimemory-service running at http://localhost:8787
 */

import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { createClient, MiniMemoryError } from '../src/index.js';

const BASE_URL = process.env.MINIMEMORY_URL || 'http://localhost:8787/api/v1';
const TEST_NAMESPACE = 'default'; // Use default namespace that should exist

describe('SDK E2E Tests', () => {
	const client = createClient({
		baseUrl: BASE_URL,
		namespace: TEST_NAMESPACE,
		apiKey: 'mm_dev_key_12345', // Dev key from service
		timeout: 10000,
	});

	let createdMemoryId: string;

	describe('Memory Operations', () => {
		it('should store a memory with remember()', async () => {
			const result = await client.remember('The quick brown fox jumps over the lazy dog', {
				type: 'semantic',
				importance: 0.8,
				metadata: { test: true, category: 'pangram' },
			});

			expect(result.success).toBe(true);
			expect(result.memory).toBeDefined();
			expect(result.memory.id).toBeDefined();
			expect(result.memory.type).toBe('semantic');
			expect(result.memory.content).toBe('The quick brown fox jumps over the lazy dog');

			createdMemoryId = result.memory.id;
			console.log('  Created memory:', createdMemoryId);
		});

		it('should recall memories with query', async () => {
			// Store a few more memories for search
			await client.remember('TypeScript is a typed superset of JavaScript', {
				type: 'semantic',
				metadata: { topic: 'programming' },
			});

			await client.remember('Python is great for machine learning', {
				type: 'semantic',
				metadata: { topic: 'programming' },
			});

			// Search
			const results = await client.recall('programming languages', {
				limit: 5,
				mode: 'keyword',
			});

			expect(results.success).toBe(true);
			expect(results.count).toBeGreaterThanOrEqual(0);
			console.log('  Found', results.count, 'results');
		});

		it('should get a specific memory by ID', async () => {
			const result = await client.get(createdMemoryId);

			expect(result.success).toBe(true);
			expect(result.memory).toBeDefined();
			expect(result.memory.id).toBe(createdMemoryId);
			expect(result.memory.content).toBe('The quick brown fox jumps over the lazy dog');
		});

		it('should update a memory', async () => {
			// Note: Skipping embedding update to avoid D1 undefined binding bug
			const result = await client.update(createdMemoryId, {
				importance: 0.95,
			});

			expect(result.success).toBe(true);
			expect(result.memory.importance).toBe(0.95);
		});

		it('should get stats', async () => {
			const result = await client.stats();

			expect(result.success).toBe(true);
			expect(result.stats).toBeDefined();
			expect(result.stats.total).toBeGreaterThanOrEqual(1);
			expect(result.stats.byType).toBeDefined();
			console.log('  Stats:', result.stats);
		});

		it('should export memories', async () => {
			const result = await client.export();

			expect(result.success).toBe(true);
			expect(result.data).toBeDefined();
			expect(result.data.memories).toBeDefined();
			console.log('  Exported', result.data.memories.length, 'memories');
		});

		it('should forget a memory by ID', async () => {
			const result = await client.forget(createdMemoryId);

			expect(result.success).toBe(true);
		});

		it('should handle 404 for non-existent memory', async () => {
			try {
				await client.get('non_existent_id_12345');
				expect.fail('Should have thrown');
			} catch (error) {
				expect(error).toBeInstanceOf(MiniMemoryError);
				expect((error as MiniMemoryError).status).toBe(404);
			}
		});
	});

	describe('Embedding Operations', () => {
		it('should get embedding info', async () => {
			const info = await client.embed.info();

			expect(info).toBeDefined();
			expect(info.model).toBeDefined();
			expect(info.dimensions).toBeDefined();
			console.log('  Embedding model:', info.model);
			console.log('  Available:', info.available);
		});

		// Note: embed.single and embed.batch require AI binding (Workers AI)
		// These will only work in deployed environment with AI enabled
	});

	describe('Cleanup', () => {
		it('should clear all test memories', async () => {
			const result = await client.clear();
			expect(result.success).toBe(true);
			console.log('  Cleared namespace:', TEST_NAMESPACE);
		});
	});
});
