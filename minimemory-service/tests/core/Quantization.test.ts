import { describe, it, expect } from 'vitest';
import {
  quantizeScalar,
  dequantizeScalar,
  quantizeBinary,
  hammingDistance,
  hammingToSimilarity,
  cosineSimilarityInt8,
  dotProductInt8,
  int8ToBase64,
  base64ToInt8,
  uint8ToBase64,
  base64ToUint8,
  truncateDimensions,
  getQuantizedSize,
  calculateSavings,
} from '../../src/core/Quantization.js';
import { VectorDB } from '../../src/core/VectorDB.js';

describe('Quantization', () => {
  describe('Scalar Quantization (int8)', () => {
    it('should quantize float32 to int8', () => {
      const vector = [1.0, -1.0, 0.5, -0.5, 0];
      const quantized = quantizeScalar(vector);

      expect(quantized).toBeInstanceOf(Int8Array);
      expect(quantized.length).toBe(5);
      expect(quantized[0]).toBe(127);   // 1.0 -> 127
      expect(quantized[1]).toBe(-127);  // -1.0 -> -127
      // 0.5 * 127 = 63.5, rounds to 64 or 63 depending on implementation
      expect(Math.abs(quantized[2] - 64)).toBeLessThanOrEqual(1);
      expect(Math.abs(quantized[3] + 64)).toBeLessThanOrEqual(1);
      expect(quantized[4]).toBe(0);     // 0 -> 0
    });

    it('should clamp values outside [-1, 1]', () => {
      const vector = [2.0, -2.0, 1.5];
      const quantized = quantizeScalar(vector);

      expect(quantized[0]).toBe(127);   // 2.0 clamped to 1.0
      expect(quantized[1]).toBe(-127);  // -2.0 clamped to -1.0
      expect(quantized[2]).toBe(127);   // 1.5 clamped to 1.0
    });

    it('should dequantize int8 back to float32', () => {
      const original = [1.0, -1.0, 0.5, -0.5, 0];
      const quantized = quantizeScalar(original);
      const restored = dequantizeScalar(quantized);

      // Should be close to original (within quantization error)
      expect(restored[0]).toBeCloseTo(1.0, 1);
      expect(restored[1]).toBeCloseTo(-1.0, 1);
      expect(restored[2]).toBeCloseTo(0.5, 1);
      expect(restored[3]).toBeCloseTo(-0.5, 1);
      expect(restored[4]).toBeCloseTo(0, 1);
    });

    it('should compute dot product for int8 vectors', () => {
      const a = quantizeScalar([1.0, 0, 0, 0]);
      const b = quantizeScalar([1.0, 0, 0, 0]);
      const c = quantizeScalar([0, 1.0, 0, 0]);

      // Same direction should have high dot product
      expect(dotProductInt8(a, b)).toBeGreaterThan(0);
      // Orthogonal should have ~0 dot product
      expect(dotProductInt8(a, c)).toBe(0);
    });

    it('should compute cosine similarity for int8 vectors', () => {
      const a = quantizeScalar([1.0, 0, 0, 0]);
      const b = quantizeScalar([1.0, 0, 0, 0]);
      const c = quantizeScalar([-1.0, 0, 0, 0]);
      const d = quantizeScalar([0, 1.0, 0, 0]);

      expect(cosineSimilarityInt8(a, b)).toBeCloseTo(1.0, 1);  // Same
      expect(cosineSimilarityInt8(a, c)).toBeCloseTo(-1.0, 1); // Opposite
      expect(cosineSimilarityInt8(a, d)).toBeCloseTo(0, 1);    // Orthogonal
    });
  });

  describe('Binary Quantization', () => {
    it('should quantize to binary (1-bit per value)', () => {
      const vector = [0.5, -0.5, 0.1, -0.1, 0.9, -0.9, 0.01, -0.01];
      const quantized = quantizeBinary(vector);

      expect(quantized).toBeInstanceOf(Uint8Array);
      expect(quantized.length).toBe(1);  // 8 values fit in 1 byte

      // Check individual bits: positive = 1, negative = 0
      // vector[0] = 0.5 (positive) -> bit 0 = 1
      // vector[1] = -0.5 (negative) -> bit 1 = 0
      // vector[2] = 0.1 (positive) -> bit 2 = 1
      // etc.
      // Binary: 01010101 = 0x55 = 85
      const expected = 0b01010101;  // Positive at indices 0, 2, 4, 6
      expect(quantized[0]).toBe(expected);
    });

    it('should handle vectors larger than 8 bits', () => {
      const vector = new Array(16).fill(0).map((_, i) => i % 2 === 0 ? 1 : -1);
      const quantized = quantizeBinary(vector);

      expect(quantized.length).toBe(2);  // 16 values need 2 bytes
    });

    it('should compute Hamming distance correctly', () => {
      const a = quantizeBinary([1, 1, 1, 1, -1, -1, -1, -1]);
      const b = quantizeBinary([1, 1, 1, 1, -1, -1, -1, -1]);  // Same
      const c = quantizeBinary([-1, -1, -1, -1, 1, 1, 1, 1]);  // All opposite

      expect(hammingDistance(a, b)).toBe(0);  // Same vectors
      expect(hammingDistance(a, c)).toBe(8);  // All bits different
    });

    it('should convert Hamming distance to similarity', () => {
      expect(hammingToSimilarity(0, 8)).toBe(1);      // Perfect match
      expect(hammingToSimilarity(8, 8)).toBe(0);      // All different
      expect(hammingToSimilarity(4, 8)).toBe(0.5);    // Half different
    });
  });

  describe('Serialization', () => {
    it('should serialize int8 to base64 and back', () => {
      const original = quantizeScalar([1.0, -1.0, 0.5, -0.5]);
      const base64 = int8ToBase64(original);
      const restored = base64ToInt8(base64);

      expect(restored).toBeInstanceOf(Int8Array);
      expect(restored.length).toBe(original.length);
      for (let i = 0; i < original.length; i++) {
        expect(restored[i]).toBe(original[i]);
      }
    });

    it('should serialize uint8 to base64 and back', () => {
      const original = quantizeBinary([1, -1, 1, -1, 1, -1, 1, -1]);
      const base64 = uint8ToBase64(original);
      const restored = base64ToUint8(base64);

      expect(restored).toBeInstanceOf(Uint8Array);
      expect(restored.length).toBe(original.length);
      for (let i = 0; i < original.length; i++) {
        expect(restored[i]).toBe(original[i]);
      }
    });
  });

  describe('Matryoshka Truncation', () => {
    it('should truncate to target dimensions', () => {
      const vector = [1, 2, 3, 4, 5, 6, 7, 8];
      const truncated = truncateDimensions(vector, 4);

      expect(truncated.length).toBe(4);
      expect(truncated).toEqual([1, 2, 3, 4]);
    });

    it('should return original if already smaller', () => {
      const vector = [1, 2, 3];
      const truncated = truncateDimensions(vector, 8);

      expect(truncated.length).toBe(3);
      expect(truncated).toEqual([1, 2, 3]);
    });
  });

  describe('Memory Calculations', () => {
    it('should calculate quantized size correctly', () => {
      const dims = 256;

      expect(getQuantizedSize(dims, 'none')).toBe(256 * 4);  // float32
      expect(getQuantizedSize(dims, 'int8')).toBe(256);       // 1 byte each
      expect(getQuantizedSize(dims, 'binary')).toBe(32);      // 256/8 bytes
    });

    it('should calculate savings percentage', () => {
      const dims = 256;

      expect(calculateSavings(dims, 'none')).toBe(0);
      expect(calculateSavings(dims, 'int8')).toBe(75);   // 4x reduction
      expect(calculateSavings(dims, 'binary')).toBeCloseTo(96.875, 1);  // 32x reduction
    });
  });
});

describe('VectorDB with Quantization', () => {
  describe('int8 quantization', () => {
    it('should create DB with int8 quantization', () => {
      const db = new VectorDB({ dimensions: 4, quantization: 'int8' });
      expect(db.quantization).toBe('int8');
    });

    it('should search with int8 quantization', () => {
      const db = new VectorDB({ dimensions: 4, quantization: 'int8' });

      db.insert('v1', [1, 0, 0, 0]);
      db.insert('v2', [0, 1, 0, 0]);
      db.insert('v3', [0.9, 0.1, 0, 0]);

      const results = db.search([1, 0, 0, 0], 2);

      expect(results.length).toBe(2);
      expect(results[0].id).toBe('v1');  // Exact match
      expect(results[1].id).toBe('v3');  // Close match
    });

    it('should show memory savings in stats', () => {
      const db = new VectorDB({ dimensions: 256, quantization: 'int8' });

      db.insert('v1', new Array(256).fill(0.5));

      const stats = db.stats();
      expect(stats.quantization).toBe('int8');
      expect((stats.memoryEstimate as any).savingsPercent).toBe(75);
    });
  });

  describe('binary quantization', () => {
    it('should create DB with binary quantization', () => {
      const db = new VectorDB({ dimensions: 8, quantization: 'binary' });
      expect(db.quantization).toBe('binary');
    });

    it('should search with binary quantization and rescoring', () => {
      const db = new VectorDB({
        dimensions: 8,
        quantization: 'binary',
        rescoreOversample: 4,
      });

      db.insert('v1', [1, 1, 1, 1, 0, 0, 0, 0]);
      db.insert('v2', [0, 0, 0, 0, 1, 1, 1, 1]);
      db.insert('v3', [1, 1, 0, 0, 0, 0, 0, 0]);

      const results = db.search([1, 1, 1, 1, 0, 0, 0, 0], 2);

      expect(results.length).toBe(2);
      expect(results[0].id).toBe('v1');  // Exact match
    });
  });

  describe('serialization with quantization', () => {
    it('should export and import with int8 quantization', () => {
      const db = new VectorDB({ dimensions: 4, quantization: 'int8' });
      db.insert('v1', [1, 0, 0, 0], { name: 'test' });

      const exported = db.export();
      expect(exported.version).toBe('3.0.0');
      expect(exported.quantization).toBe('int8');

      const imported = VectorDB.import(exported);
      expect(imported.quantization).toBe('int8');
      expect(imported.length).toBe(1);

      const results = imported.search([1, 0, 0, 0], 1);
      expect(results[0].id).toBe('v1');
    });
  });
});
