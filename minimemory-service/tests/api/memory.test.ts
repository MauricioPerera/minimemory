import { describe, it, expect, beforeEach } from 'vitest';
import { Hono } from 'hono';
import { createMemoryRoutes } from '../../src/api/memory.js';
import { MemoryManager } from '../../src/memory/MemoryManager.js';

describe('Memory API', () => {
  let app: Hono;
  let managers: Map<string, MemoryManager>;

  beforeEach(() => {
    managers = new Map();

    const getManager = (namespace: string, dimensions = 4) => {
      if (!managers.has(namespace)) {
        managers.set(namespace, new MemoryManager({ dimensions }));
      }
      return managers.get(namespace)!;
    };

    app = new Hono();
    app.route('/api/v1', createMemoryRoutes(getManager));
  });

  describe('POST /api/v1/remember', () => {
    it('should create a memory', async () => {
      const res = await app.request('/api/v1/remember', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Namespace': 'test',
        },
        body: JSON.stringify({
          content: 'Test memory content',
          type: 'semantic',
          importance: 0.8,
        }),
      });

      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data.success).toBe(true);
      expect(data.memory).toBeDefined();
      expect(data.memory.content).toBe('Test memory content');
    });

    it('should reject missing content', async () => {
      const res = await app.request('/api/v1/remember', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Namespace': 'test',
        },
        body: JSON.stringify({
          type: 'semantic',
        }),
      });

      expect(res.status).toBe(400);
      const data = await res.json();
      expect(data.error).toContain('content');
    });

    it('should use default namespace if not provided', async () => {
      const res = await app.request('/api/v1/remember', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          content: 'Test memory',
        }),
      });

      expect(res.status).toBe(200);
    });

    it('should work without embedding (keyword-only memory)', async () => {
      // Test that memories can be created without providing an embedding
      // This is useful for keyword-only search
      const res = await app.request('/api/v1/remember', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Namespace': 'test-no-embed',
        },
        body: JSON.stringify({
          content: 'This is a keyword-only memory',
          type: 'semantic',
          importance: 0.7,
        }),
      });

      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data.success).toBe(true);
      expect(data.memory).toBeDefined();
      expect(data.memory.content).toBe('This is a keyword-only memory');
    });
  });

  describe('POST /api/v1/recall', () => {
    beforeEach(async () => {
      // Add some test memories
      await app.request('/api/v1/remember', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Namespace': 'test',
        },
        body: JSON.stringify({
          content: 'The user prefers dark mode',
          type: 'semantic',
          importance: 0.8,
        }),
      });

      await app.request('/api/v1/remember', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Namespace': 'test',
        },
        body: JSON.stringify({
          content: 'User asked about pricing',
          type: 'episodic',
          importance: 0.6,
        }),
      });
    });

    it('should search by keywords', async () => {
      const res = await app.request('/api/v1/recall', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Namespace': 'test',
        },
        body: JSON.stringify({
          keywords: 'dark mode',
          mode: 'keyword',
        }),
      });

      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data.success).toBe(true);
      expect(data.results).toBeDefined();
    });

    it('should reject missing keywords and embedding', async () => {
      const res = await app.request('/api/v1/recall', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Namespace': 'test',
        },
        body: JSON.stringify({
          limit: 10,
        }),
      });

      expect(res.status).toBe(400);
      const data = await res.json();
      expect(data.error).toContain('keywords');
    });

    it('should filter by type', async () => {
      const res = await app.request('/api/v1/recall', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Namespace': 'test',
        },
        body: JSON.stringify({
          keywords: 'user',
          type: 'semantic',
        }),
      });

      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data.results.every((r: any) => r.type === 'semantic')).toBe(true);
    });
  });

  describe('GET /api/v1/stats', () => {
    it('should return stats for namespace', async () => {
      // Add a memory first
      await app.request('/api/v1/remember', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Namespace': 'test',
        },
        body: JSON.stringify({
          content: 'Test memory',
          type: 'semantic',
        }),
      });

      const res = await app.request('/api/v1/stats', {
        headers: {
          'X-Namespace': 'test',
        },
      });

      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data.success).toBe(true);
      expect(data.stats).toBeDefined();
      expect(data.stats.total).toBeGreaterThanOrEqual(1);
    });

    it('should return empty stats for new namespace', async () => {
      const res = await app.request('/api/v1/stats', {
        headers: {
          'X-Namespace': 'empty-namespace',
        },
      });

      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data.stats.total).toBe(0);
    });
  });

  describe('DELETE /api/v1/forget/:id', () => {
    it('should delete a memory', async () => {
      // Create a memory
      const createRes = await app.request('/api/v1/remember', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Namespace': 'test',
        },
        body: JSON.stringify({
          content: 'Memory to delete',
        }),
      });

      const createData = await createRes.json();
      const memoryId = createData.memory.id;

      // Delete it
      const deleteRes = await app.request(`/api/v1/forget/${memoryId}`, {
        method: 'DELETE',
        headers: {
          'X-Namespace': 'test',
        },
      });

      expect(deleteRes.status).toBe(200);
      const deleteData = await deleteRes.json();
      expect(deleteData.success).toBe(true);
    });

    it('should handle non-existent memory', async () => {
      const res = await app.request('/api/v1/forget/non-existent-id', {
        method: 'DELETE',
        headers: {
          'X-Namespace': 'test',
        },
      });

      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data.success).toBe(false);
    });
  });

  describe('DELETE /api/v1/clear', () => {
    it('should clear all memories in namespace', async () => {
      // Add memories
      await app.request('/api/v1/remember', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Namespace': 'test-clear',
        },
        body: JSON.stringify({ content: 'Memory 1' }),
      });

      await app.request('/api/v1/remember', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Namespace': 'test-clear',
        },
        body: JSON.stringify({ content: 'Memory 2' }),
      });

      // Clear
      const clearRes = await app.request('/api/v1/clear', {
        method: 'DELETE',
        headers: {
          'X-Namespace': 'test-clear',
        },
      });

      expect(clearRes.status).toBe(200);

      // Verify empty
      const statsRes = await app.request('/api/v1/stats', {
        headers: {
          'X-Namespace': 'test-clear',
        },
      });

      const stats = await statsRes.json();
      expect(stats.stats.total).toBe(0);
    });
  });
});
