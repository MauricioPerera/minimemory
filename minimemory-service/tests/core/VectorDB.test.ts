import { describe, it, expect, beforeEach } from 'vitest';
import { VectorDB } from '../../src/core/VectorDB.js';

describe('VectorDB', () => {
  let db: VectorDB;

  beforeEach(() => {
    db = new VectorDB({ dimensions: 4 });
  });

  describe('insert', () => {
    it('should insert a vector with id', () => {
      db.insert('test-1', [1, 0, 0, 0]);
      expect(db.length).toBe(1);
      expect(db.contains('test-1')).toBe(true);
    });

    it('should insert a vector with metadata', () => {
      db.insert('test-1', [1, 0, 0, 0], { type: 'test', score: 0.8 });
      const result = db.get('test-1');
      expect(result).not.toBeNull();
      expect(result?.metadata).toEqual({ type: 'test', score: 0.8 });
    });

    it('should throw error for duplicate id', () => {
      db.insert('test-1', [1, 0, 0, 0]);
      expect(() => db.insert('test-1', [0, 1, 0, 0])).toThrow('already exists');
    });

    it('should throw error for wrong dimensions', () => {
      expect(() => db.insert('test', [1, 0, 0])).toThrow();
    });
  });

  describe('upsert', () => {
    it('should insert a new vector', () => {
      db.upsert('test-1', [1, 0, 0, 0]);
      expect(db.length).toBe(1);
    });

    it('should update existing vector', () => {
      db.insert('test-1', [1, 0, 0, 0], { old: true });
      db.upsert('test-1', [0, 1, 0, 0], { new: true });

      const result = db.get('test-1');
      expect(result?.vector).toEqual([0, 1, 0, 0]);
      expect(result?.metadata).toEqual({ new: true });
    });
  });

  describe('get', () => {
    it('should return null for non-existent id', () => {
      const result = db.get('non-existent');
      expect(result).toBeNull();
    });

    it('should return vector data for existing id', () => {
      db.insert('test-1', [1, 2, 3, 4], { key: 'value' });
      const result = db.get('test-1');
      expect(result).not.toBeNull();
      expect(result?.vector).toEqual([1, 2, 3, 4]);
      expect(result?.metadata).toEqual({ key: 'value' });
    });
  });

  describe('delete', () => {
    it('should delete existing vector', () => {
      db.insert('test-1', [1, 0, 0, 0]);
      expect(db.length).toBe(1);

      const deleted = db.delete('test-1');
      expect(deleted).toBe(true);
      expect(db.length).toBe(0);
    });

    it('should return false for non-existent id', () => {
      const deleted = db.delete('non-existent');
      expect(deleted).toBe(false);
    });
  });

  describe('search', () => {
    beforeEach(() => {
      // Insert some test vectors
      db.insert('v1', [1, 0, 0, 0]);
      db.insert('v2', [0, 1, 0, 0]);
      db.insert('v3', [0, 0, 1, 0]);
      db.insert('v4', [0.7, 0.7, 0, 0]); // Similar to v1 and v2
    });

    it('should find most similar vectors', () => {
      const results = db.search([1, 0, 0, 0], 2);
      expect(results.length).toBe(2);
      expect(results[0].id).toBe('v1'); // Exact match
      expect(results[0].similarity).toBeCloseTo(1.0, 5);
    });

    it('should return empty array for empty db', () => {
      const emptyDb = new VectorDB({ dimensions: 4 });
      const results = emptyDb.search([1, 0, 0, 0], 5);
      expect(results).toEqual([]);
    });

    it('should respect limit parameter', () => {
      const results = db.search([1, 0, 0, 0], 2);
      expect(results.length).toBe(2);
    });

    it('should filter by metadata', () => {
      db.insert('filtered', [0.9, 0.1, 0, 0], { type: 'special' });

      const results = db.search([1, 0, 0, 0], 10, {
        filter: { type: 'special' }
      });

      expect(results.length).toBe(1);
      expect(results[0].id).toBe('filtered');
    });
  });

  describe('clear', () => {
    it('should remove all vectors', () => {
      db.insert('v1', [1, 0, 0, 0]);
      db.insert('v2', [0, 1, 0, 0]);
      expect(db.length).toBe(2);

      db.clear();
      expect(db.length).toBe(0);
    });
  });

  describe('serialize/deserialize', () => {
    it('should serialize and deserialize correctly', () => {
      db.insert('v1', [1, 0, 0, 0], { key: 'value1' });
      db.insert('v2', [0, 1, 0, 0], { key: 'value2' });

      const serialized = db.export();
      expect(serialized).toBeDefined();
      expect(serialized.version).toBe('3.0.0');

      const newDb = VectorDB.import(serialized);
      expect(newDb.length).toBe(2);
      expect(newDb.get('v1')?.metadata).toEqual({ key: 'value1' });
      expect(newDb.get('v2')?.metadata).toEqual({ key: 'value2' });
    });
  });

  describe('distance metrics', () => {
    it('should work with euclidean distance', () => {
      const euclideanDb = new VectorDB({ dimensions: 4, distance: 'euclidean' });
      euclideanDb.insert('v1', [0, 0, 0, 0]);
      euclideanDb.insert('v2', [1, 0, 0, 0]);
      euclideanDb.insert('v3', [2, 0, 0, 0]);

      const results = euclideanDb.search([0, 0, 0, 0], 3);
      expect(results[0].id).toBe('v1');
      expect(results[1].id).toBe('v2');
      expect(results[2].id).toBe('v3');
    });

    it('should work with dot product distance', () => {
      const dotDb = new VectorDB({ dimensions: 4, distance: 'dot' });
      dotDb.insert('v1', [1, 0, 0, 0]);
      dotDb.insert('v2', [0.5, 0.5, 0, 0]);

      const results = dotDb.search([1, 0, 0, 0], 2);
      expect(results).toHaveLength(2);
    });
  });
});
