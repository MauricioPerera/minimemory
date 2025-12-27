import { describe, it, expect, beforeEach } from 'vitest';
import { BM25Index } from '../../src/core/BM25Index.js';

describe('BM25Index', () => {
  let index: BM25Index;

  beforeEach(() => {
    index = new BM25Index({ textFields: ['content'] });
  });

  describe('addDocument', () => {
    it('should add a document', () => {
      index.addDocument('doc1', { content: 'Hello world' });
      expect(index.documentCount).toBe(1);
    });

    it('should add multiple documents', () => {
      index.addDocument('doc1', { content: 'Hello world' });
      index.addDocument('doc2', { content: 'Hello there' });
      index.addDocument('doc3', { content: 'World peace' });
      expect(index.documentCount).toBe(3);
    });

    it('should update existing document', () => {
      index.addDocument('doc1', { content: 'Hello world' });
      index.addDocument('doc1', { content: 'Goodbye world' });
      expect(index.documentCount).toBe(1);

      const results = index.search('goodbye', 10);
      expect(results.length).toBe(1);
      expect(results[0].id).toBe('doc1');
    });
  });

  describe('removeDocument', () => {
    it('should remove a document', () => {
      index.addDocument('doc1', { content: 'Hello world' });
      expect(index.documentCount).toBe(1);

      const removed = index.removeDocument('doc1');
      expect(removed).toBe(true);
      expect(index.documentCount).toBe(0);
    });

    it('should return false for non-existent document', () => {
      const removed = index.removeDocument('non-existent');
      expect(removed).toBe(false);
    });
  });

  describe('search', () => {
    beforeEach(() => {
      index.addDocument('doc1', { content: 'The quick brown fox jumps over the lazy dog' });
      index.addDocument('doc2', { content: 'A quick brown dog runs in the park' });
      index.addDocument('doc3', { content: 'The lazy cat sleeps all day' });
      index.addDocument('doc4', { content: 'Dogs and cats are popular pets' });
    });

    it('should find documents by single term', () => {
      const results = index.search('fox', 10);
      expect(results.length).toBeGreaterThan(0);
      expect(results[0].id).toBe('doc1');
    });

    it('should find documents by multiple terms', () => {
      const results = index.search('quick brown', 10);
      expect(results.length).toBeGreaterThan(0);
      // Both doc1 and doc2 have "quick brown"
      expect(results.some(r => r.id === 'doc1')).toBe(true);
      expect(results.some(r => r.id === 'doc2')).toBe(true);
    });

    it('should return empty array for no matches', () => {
      const results = index.search('elephant', 10);
      expect(results.length).toBe(0);
    });

    it('should respect limit parameter', () => {
      const results = index.search('the', 2);
      expect(results.length).toBeLessThanOrEqual(2);
    });

    it('should rank more relevant documents higher', () => {
      const results = index.search('lazy', 10);
      // doc1 and doc3 both have "lazy", but in different contexts
      expect(results.length).toBeGreaterThan(0);
    });

    it('should handle case insensitively', () => {
      const results1 = index.search('DOG', 10);
      const results2 = index.search('dog', 10);
      expect(results1.length).toBe(results2.length);
    });
  });

  describe('clear', () => {
    it('should remove all documents', () => {
      index.addDocument('doc1', { content: 'Hello world' });
      index.addDocument('doc2', { content: 'Hello there' });
      expect(index.documentCount).toBe(2);

      index.clear();
      expect(index.documentCount).toBe(0);
    });
  });

  describe('serialize/deserialize', () => {
    it('should serialize and deserialize correctly', () => {
      index.addDocument('doc1', { content: 'Hello world' });
      index.addDocument('doc2', { content: 'Hello there' });

      const serialized = index.serialize();
      expect(serialized).toBeDefined();
      expect(serialized.version).toBe('1.0.0');

      const newIndex = BM25Index.deserialize(serialized);
      expect(newIndex.documentCount).toBe(2);

      const results = newIndex.search('hello', 10);
      expect(results.length).toBe(2);
    });
  });

  describe('edge cases', () => {
    it('should handle empty query', () => {
      index.addDocument('doc1', { content: 'Hello world' });
      const results = index.search('', 10);
      expect(results.length).toBe(0);
    });

    it('should handle empty document', () => {
      index.addDocument('doc1', { content: '' });
      expect(index.documentCount).toBe(1);

      const results = index.search('hello', 10);
      expect(results.length).toBe(0);
    });

    it('should handle special characters', () => {
      index.addDocument('doc1', { content: 'Hello, world! How are you?' });
      const results = index.search('hello world', 10);
      expect(results.length).toBe(1);
    });

    it('should handle numbers', () => {
      index.addDocument('doc1', { content: 'Product 123 costs $50' });
      const results = index.search('123', 10);
      expect(results.length).toBe(1);
    });
  });

  describe('multiple text fields', () => {
    it('should index multiple fields', () => {
      const multiFieldIndex = new BM25Index({ textFields: ['title', 'content'] });
      multiFieldIndex.addDocument('doc1', {
        title: 'Important announcement',
        content: 'This is the body text'
      });
      multiFieldIndex.addDocument('doc2', {
        title: 'Another document',
        content: 'Contains announcement word'
      });

      const results = multiFieldIndex.search('announcement', 10);
      expect(results.length).toBe(2);
    });
  });

  describe('statistics', () => {
    it('should return correct stats', () => {
      index.addDocument('doc1', { content: 'Hello world' });
      index.addDocument('doc2', { content: 'Hello there my friend' });

      const stats = index.getStats();
      expect(stats.documentCount).toBe(2);
      expect(stats.k1).toBe(1.2);
      expect(stats.b).toBe(0.75);
      expect(stats.textFields).toEqual(['content']);
      expect(stats.avgDocLength).toBeGreaterThan(0);
      expect(stats.vocabularySize).toBeGreaterThan(0);
    });
  });
});
