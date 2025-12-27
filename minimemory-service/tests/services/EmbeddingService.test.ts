import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  EmbeddingService,
  createMockEmbeddingService,
} from '../../src/services/EmbeddingService.js';

describe('EmbeddingService', () => {
  describe('with mock AI', () => {
    let service: EmbeddingService;

    beforeEach(() => {
      service = createMockEmbeddingService(768);
    });

    describe('embed', () => {
      it('should generate embedding for text', async () => {
        const result = await service.embed('Hello world');

        expect(result.embedding).toBeDefined();
        expect(result.embedding.length).toBe(768);
        expect(result.model).toBe('@cf/google/gemma-embedding-300m');
        expect(result.truncated).toBe(false);
      });

      it('should normalize embeddings by default', async () => {
        const result = await service.embed('Test text');

        // Check normalization: magnitude should be ~1
        let magnitude = 0;
        for (const v of result.embedding) {
          magnitude += v * v;
        }
        magnitude = Math.sqrt(magnitude);

        expect(magnitude).toBeCloseTo(1.0, 2);
      });

      it('should truncate to specified dimensions (Matryoshka)', async () => {
        const service256 = createMockEmbeddingService(768);
        const result = await service256.embed('Test text', { dimensions: 256 });

        expect(result.embedding.length).toBe(256);
        expect(result.dimensions).toBe(256);
        expect(result.truncated).toBe(true);
      });

      it('should truncate to 128 dimensions', async () => {
        const result = await service.embed('Test text', { dimensions: 128 });

        expect(result.embedding.length).toBe(128);
        expect(result.truncated).toBe(true);
      });

      it('should generate deterministic embeddings for same text', async () => {
        const result1 = await service.embed('Same text');
        const result2 = await service.embed('Same text');

        expect(result1.embedding).toEqual(result2.embedding);
      });

      it('should generate different embeddings for different text', async () => {
        const result1 = await service.embed('Text one');
        const result2 = await service.embed('Text two');

        expect(result1.embedding).not.toEqual(result2.embedding);
      });
    });

    describe('embedBatch', () => {
      it('should generate embeddings for multiple texts', async () => {
        const result = await service.embedBatch(['Hello', 'World', 'Test']);

        expect(result.embeddings.length).toBe(3);
        expect(result.count).toBe(3);
        expect(result.embeddings[0].length).toBe(768);
      });

      it('should truncate batch embeddings', async () => {
        const result = await service.embedBatch(['Hello', 'World'], {
          dimensions: 256,
        });

        expect(result.embeddings[0].length).toBe(256);
        expect(result.embeddings[1].length).toBe(256);
        expect(result.dimensions).toBe(256);
      });
    });

    describe('getEmbedding', () => {
      it('should return embedding for valid text', async () => {
        const embedding = await service.getEmbedding('Valid text');

        expect(embedding).not.toBeNull();
        expect(embedding!.length).toBe(768);
      });

      it('should return null for empty text', async () => {
        const embedding = await service.getEmbedding('');
        expect(embedding).toBeNull();
      });

      it('should return null for whitespace-only text', async () => {
        const embedding = await service.getEmbedding('   \n\t  ');
        expect(embedding).toBeNull();
      });
    });
  });

  describe('cost estimation', () => {
    it('should calculate cost for 768 dimensions', () => {
      const cost = EmbeddingService.estimateCost(10000, 768);
      // 10000 * 768 = 7,680,000 neurons
      // 7,680,000 / 1000 * 0.011 = 84.48
      expect(cost).toBeCloseTo(84.48, 1);
    });

    it('should calculate cost for 256 dimensions', () => {
      const cost = EmbeddingService.estimateCost(10000, 256);
      // 10000 * 256 = 2,560,000 neurons
      // 2,560,000 / 1000 * 0.011 = 28.16
      expect(cost).toBeCloseTo(28.16, 1);
    });

    it('should calculate daily free limit for 768 dimensions', () => {
      const limit = EmbeddingService.getDailyFreeLimit(768);
      // 10000 / 768 = 13
      expect(limit).toBe(13);
    });

    it('should calculate daily free limit for 128 dimensions', () => {
      const limit = EmbeddingService.getDailyFreeLimit(128);
      // 10000 / 128 = 78
      expect(limit).toBe(78);
    });
  });

  describe('properties', () => {
    it('should expose dimensions', () => {
      const service = createMockEmbeddingService(512);
      expect(service.dimensions).toBe(512);
    });

    it('should expose model name', () => {
      const service = createMockEmbeddingService(768);
      expect(service.modelName).toBe('@cf/google/gemma-embedding-300m');
    });
  });

  describe('with real AI mock', () => {
    it('should handle AI errors gracefully', async () => {
      const errorAi = {
        run: vi.fn().mockRejectedValue(new Error('AI service unavailable')),
      };

      const service = new EmbeddingService(errorAi);

      await expect(service.embed('test')).rejects.toThrow('AI service unavailable');
    });

    it('should handle empty response', async () => {
      const emptyAi = {
        run: vi.fn().mockResolvedValue({ data: [] }),
      };

      const service = new EmbeddingService(emptyAi);

      await expect(service.embed('test')).rejects.toThrow('empty response');
    });
  });
});
