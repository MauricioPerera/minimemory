/**
 * minimemory — TypeScript idiomatic wrapper
 *
 * Wraps WasmVectorDB with:
 * - Automatic JSON parsing (no manual JSON.parse)
 * - Native TypeScript types
 * - Async init() helper
 * - Builder pattern for config
 */

// Re-export raw WASM bindings for advanced users
export { WasmVectorDB } from "../pkg/minimemory.js";
export { default as initWasm } from "../pkg/minimemory.js";

import init, { WasmVectorDB } from "../pkg/minimemory.js";

// ─── Types ───────────────────────────────────────────────────────────────────

export interface SearchResult {
  id: string;
  distance: number;
  metadata?: Record<string, any>;
}

export interface HybridResult {
  id: string;
  score: number;
  metadata?: Record<string, any>;
}

export interface PagedResult<T> {
  items: T[];
  total: number;
  offset: number;
  limit: number;
  has_more: boolean;
}

export interface DocumentEntry {
  id: string;
  vector?: number[];
  metadata?: Record<string, any>;
}

export type Distance = "cosine" | "euclidean" | "dot" | "manhattan";
export type IndexType = "flat" | "hnsw";
export type Quantization = "none" | "int8" | "int3" | "binary" | "polar";

export interface MiniMemoryConfig {
  dimensions: number;
  distance?: Distance;
  index?: IndexType;
  quantization?: Quantization;
  hnsw_m?: number;
  hnsw_ef?: number;
}

// ─── Main Class ──────────────────────────────────────────────────────────────

export class MiniMemory {
  private db: WasmVectorDB;
  readonly dimensions: number;

  private constructor(db: WasmVectorDB) {
    this.db = db;
    this.dimensions = db.dimensions();
  }

  /**
   * Create a new MiniMemory instance.
   * Call this instead of using the constructor directly.
   *
   * ```ts
   * const db = await MiniMemory.create({ dimensions: 384 });
   * ```
   */
  static async create(
    config: MiniMemoryConfig,
    wasmUrl?: any,
  ): Promise<MiniMemory> {
    await init(wasmUrl);

    const dist = config.distance || "cosine";
    const idx = config.index || "flat";
    const quant = config.quantization || "none";

    let db: WasmVectorDB;
    if (quant !== "none" || config.hnsw_m || config.hnsw_ef) {
      db = WasmVectorDB.new_with_config(
        config.dimensions,
        dist,
        idx,
        quant,
        config.hnsw_m ?? null,
        config.hnsw_ef ?? null,
      );
    } else if (idx === "hnsw") {
      db = WasmVectorDB.new_hnsw(config.dimensions, dist, 16, 200);
    } else {
      db = new WasmVectorDB(config.dimensions, dist, idx);
    }

    return new MiniMemory(db);
  }

  // ─── CRUD ────────────────────────────────────────────────────────────────

  /** Insert a vector with optional metadata. */
  insert(id: string, vector: Float32Array | number[], metadata?: Record<string, any>): void {
    const v = vector instanceof Float32Array ? vector : new Float32Array(vector);
    if (metadata) {
      this.db.insert_with_metadata(id, v, JSON.stringify(metadata));
    } else {
      this.db.insert(id, v);
    }
  }

  /** Insert a document (vector optional — works as document store). */
  insertDocument(id: string, metadata: Record<string, any>, vector?: Float32Array | number[]): void {
    const v = vector
      ? vector instanceof Float32Array ? vector : new Float32Array(vector)
      : undefined;
    this.db.insert_document(id, v ? Array.from(v) : null, JSON.stringify(metadata));
  }

  /** Get a document by ID. Returns null if not found. */
  get(id: string): DocumentEntry | null {
    const raw = this.db.get(id);
    if (!raw) return null;
    const parsed = typeof raw === "string" ? JSON.parse(raw) : raw;
    return { id, vector: parsed.vector, metadata: parsed.metadata };
  }

  /** Delete a document by ID. */
  delete(id: string): boolean {
    return this.db.delete(id);
  }

  /** Update a vector with optional metadata. */
  update(id: string, vector: Float32Array | number[], metadata?: Record<string, any>): void {
    const v = vector instanceof Float32Array ? vector : new Float32Array(vector);
    if (metadata) {
      this.db.update_with_metadata(id, v, JSON.stringify(metadata));
    } else {
      this.db.update(id, v);
    }
  }

  /** Check if a document exists. */
  has(id: string): boolean {
    return this.db.contains(id);
  }

  /** Get all document IDs. */
  ids(): string[] {
    return JSON.parse(this.db.ids());
  }

  /** Number of documents. */
  get count(): number {
    return this.db.len();
  }

  /** Whether the database is empty. */
  get empty(): boolean {
    return this.db.is_empty();
  }

  /** Clear all documents. */
  clear(): void {
    this.db.clear();
  }

  // ─── Search ──────────────────────────────────────────────────────────────

  /** Semantic similarity search. Returns nearest neighbors sorted by distance. */
  search(query: Float32Array | number[], k: number = 10): SearchResult[] {
    const q = query instanceof Float32Array ? query : new Float32Array(query);
    return JSON.parse(this.db.search(q, k));
  }

  /** Full-text keyword search (BM25). */
  keywordSearch(query: string, k: number = 10): HybridResult[] {
    return JSON.parse(this.db.keyword_search(query, k));
  }

  /** Filter search by metadata (MongoDB-style). */
  filterSearch(filter: Record<string, any>, limit: number = 100): HybridResult[] {
    return JSON.parse(this.db.filter_search(JSON.stringify(filter), limit));
  }

  /** Vector search with metadata filter. */
  searchWithFilter(query: Float32Array | number[], k: number, filter: Record<string, any>): SearchResult[] {
    const q = query instanceof Float32Array ? query : new Float32Array(query);
    return JSON.parse(this.db.search_with_filter(q, k, JSON.stringify(filter)));
  }

  /**
   * List documents with optional filter, ordering, and pagination.
   * Like: SELECT * WHERE filter ORDER BY field LIMIT n OFFSET m
   */
  list(options: {
    filter?: Record<string, any>;
    orderBy?: string;
    desc?: boolean;
    limit?: number;
    offset?: number;
  } = {}): PagedResult<{ id: string; metadata?: Record<string, any> }> {
    return JSON.parse(
      this.db.list_documents(
        options.filter ? JSON.stringify(options.filter) : "{}",
        options.orderBy || "",
        options.desc ?? false,
        options.limit ?? 50,
        options.offset ?? 0,
      ),
    );
  }

  /** Paginated vector search. */
  searchPaged(query: Float32Array | number[], limit: number = 10, offset: number = 0): PagedResult<SearchResult> {
    const q = query instanceof Float32Array ? query : new Float32Array(query);
    return JSON.parse(this.db.search_paged(q, limit, offset));
  }

  // ─── Matryoshka ──────────────────────────────────────────────────────────

  /** Insert with auto-truncation (for Matryoshka embeddings). */
  insertAuto(id: string, fullVector: Float32Array | number[], metadata?: Record<string, any>): void {
    const v = fullVector instanceof Float32Array ? fullVector : new Float32Array(fullVector);
    if (metadata) {
      this.db.insert_auto_with_metadata(id, v, JSON.stringify(metadata));
    } else {
      this.db.insert_auto(id, v);
    }
  }

  /** Search with auto-truncation. */
  searchAuto(fullQuery: Float32Array | number[], k: number = 10): SearchResult[] {
    const q = fullQuery instanceof Float32Array ? fullQuery : new Float32Array(fullQuery);
    return JSON.parse(this.db.search_auto(q, k));
  }

  // ─── Persistence ─────────────────────────────────────────────────────────

  /** Export entire database as JSON string (for IndexedDB, localStorage, etc.) */
  export(): string {
    return this.db.export_snapshot();
  }

  /** Import from a JSON snapshot. Returns number of documents imported. */
  import(snapshot: string): number {
    return this.db.import_snapshot(snapshot);
  }

  /** Free WASM memory. Call when done with the database. */
  dispose(): void {
    this.db.free();
  }
}

export default MiniMemory;
