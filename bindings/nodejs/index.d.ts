/**
 * minimemory - Embedded vector database for Node.js
 * Like SQLite, but for vector similarity search.
 */

export interface SearchResult {
  /** Vector ID */
  id: string;
  /** Distance to query vector (lower = more similar) */
  distance: number;
}

export interface VectorDBOptions {
  /** Number of dimensions for vectors */
  dimensions: number;
  /** Distance metric: "cosine" | "euclidean" | "dot" */
  distance?: string;
  /** Index type: "flat" | "hnsw" */
  indexType?: string;
}

export class VectorDB {
  /**
   * Create a new vector database
   * @param options Database configuration
   */
  constructor(options: VectorDBOptions);

  /**
   * Insert a vector with optional metadata
   * @param id Unique identifier for the vector
   * @param vector Array of float32 values
   * @param metadata Optional JSON metadata
   */
  insert(id: string, vector: number[] | Float32Array, metadata?: Record<string, unknown>): void;

  /**
   * Search for k nearest neighbors
   * @param query Query vector
   * @param k Number of results to return
   * @returns Array of search results sorted by distance
   */
  search(query: number[] | Float32Array, k: number): SearchResult[];

  /**
   * Get a vector by its ID
   * @param id Vector ID
   * @returns Tuple of [vector, metadata] or null if not found
   */
  get(id: string): [number[], Record<string, unknown> | null] | null;

  /**
   * Delete a vector by its ID
   * @param id Vector ID
   * @returns true if deleted, false if not found
   */
  delete(id: string): boolean;

  /**
   * Check if a vector exists
   * @param id Vector ID
   */
  contains(id: string): boolean;

  /**
   * Update an existing vector
   * @param id Vector ID
   * @param vector New vector values
   * @param metadata New metadata (replaces existing)
   */
  update(id: string, vector: number[] | Float32Array, metadata?: Record<string, unknown>): void;

  /**
   * Save the database to a file
   * @param path File path (.mmdb extension recommended)
   */
  save(path: string): void;

  /**
   * Load a database from a file
   * @param path Path to .mmdb file
   */
  static load(path: string): VectorDB;

  /**
   * Remove all vectors from the database
   */
  clear(): void;

  /**
   * Get the number of vectors in the database
   */
  get length(): number;

  /**
   * Get the configured dimensions
   */
  get dimensions(): number;
}
