/**
 * VectorPass - Vector Database Operations
 *
 * Wraps minimemory WASM bindings with EmbeddingGemma integration
 */

import { Env, User, DatabaseInfo, SearchResult, KeywordResult, TIER_LIMITS } from './types';

// Import WASM module (will be built from minimemory)
// import init, { WasmVectorDB } from '../pkg/minimemory.js';

// Configuration
const CONFIG = {
    dimensions: 256,        // Matryoshka truncated (768 -> 256)
    distance: "cosine",
    index: "hnsw",
    quantization: "int8",   // 4x compression
    model: "@cf/google/embeddinggemma-300m"
};

/**
 * VectorDB wrapper with embedding generation
 */
export class VectorDB {
    private db: any;  // WasmVectorDB
    private ai: any;
    private kv: KVNamespace;
    private userId: string;
    private dbId: string;
    private vectorCount: number = 0;

    private constructor(
        db: any,
        ai: any,
        kv: KVNamespace,
        userId: string,
        dbId: string,
        vectorCount: number
    ) {
        this.db = db;
        this.ai = ai;
        this.kv = kv;
        this.userId = userId;
        this.dbId = dbId;
        this.vectorCount = vectorCount;
    }

    /**
     * Creates or loads a vector database for user
     */
    static async create(
        user: User,
        dbId: string,
        env: Env
    ): Promise<VectorDB> {
        // TODO: Initialize WASM
        // await init();

        // For now, use a simple in-memory structure
        // In production, this will use WasmVectorDB
        const db = new InMemoryVectorDB(CONFIG.dimensions);

        const kvKey = `db:${user.id}:${dbId}`;

        // Try to restore from KV
        const saved = await env.VECTORS.get(kvKey);
        if (saved) {
            const data = JSON.parse(saved);
            db.import(data);
        }

        return new VectorDB(db, env.AI, env.VECTORS, user.id, dbId, db.len());
    }

    /**
     * Generates embedding using EmbeddingGemma
     */
    private async embed(text: string): Promise<Float32Array> {
        const result = await this.ai.run(CONFIG.model, { text });
        const fullEmbedding = new Float32Array(result.data);

        // Truncate to target dimensions and normalize
        return this.truncateAndNormalize(fullEmbedding);
    }

    /**
     * Batch embedding generation
     */
    private async embedBatch(texts: string[]): Promise<Float32Array[]> {
        const result = await this.ai.run(CONFIG.model, { text: texts });

        // Handle different response formats
        if (Array.isArray(result.data[0])) {
            return result.data.map((arr: number[]) =>
                this.truncateAndNormalize(new Float32Array(arr))
            );
        }

        // Flat array - split by dimensions
        const embeddings: Float32Array[] = [];
        const fullDims = 768;
        for (let i = 0; i < texts.length; i++) {
            const start = i * fullDims;
            const full = new Float32Array(result.data.slice(start, start + fullDims));
            embeddings.push(this.truncateAndNormalize(full));
        }
        return embeddings;
    }

    /**
     * Truncates vector to target dimensions and L2 normalizes
     */
    private truncateAndNormalize(vector: Float32Array): Float32Array {
        const truncated = vector.slice(0, CONFIG.dimensions);

        // L2 normalize
        let norm = 0;
        for (let i = 0; i < truncated.length; i++) {
            norm += truncated[i] * truncated[i];
        }
        norm = Math.sqrt(norm);

        if (norm > 1e-10) {
            for (let i = 0; i < truncated.length; i++) {
                truncated[i] /= norm;
            }
        }

        return truncated;
    }

    /**
     * Index a document
     */
    async index(id: string, text: string, metadata?: Record<string, any>): Promise<void> {
        const embedding = await this.embed(text);
        const meta = metadata ? { ...metadata, _snippet: text.slice(0, 500) } : undefined;

        this.db.insert(id, embedding, meta);
        this.vectorCount = this.db.len();
    }

    /**
     * Batch index documents
     */
    async indexBatch(items: Array<{ id: string; text: string; metadata?: Record<string, any> }>): Promise<number> {
        const texts = items.map(i => i.text);
        const embeddings = await this.embedBatch(texts);

        for (let i = 0; i < items.length; i++) {
            const item = items[i];
            const meta = item.metadata
                ? { ...item.metadata, _snippet: item.text.slice(0, 500) }
                : undefined;

            this.db.insert(item.id, embeddings[i], meta);
        }

        this.vectorCount = this.db.len();
        return items.length;
    }

    /**
     * Semantic search
     */
    async search(query: string, k: number = 10): Promise<SearchResult[]> {
        const embedding = await this.embed(query);
        return this.db.search(embedding, k);
    }

    /**
     * Keyword search (BM25)
     */
    keywordSearch(query: string, k: number = 10): KeywordResult[] {
        return this.db.keywordSearch(query, k);
    }

    /**
     * Update a document
     */
    async update(id: string, text: string, metadata?: Record<string, any>): Promise<boolean> {
        const embedding = await this.embed(text);
        const meta = metadata ? { ...metadata, _snippet: text.slice(0, 500) } : undefined;
        return this.db.update(id, embedding, meta);
    }

    /**
     * Delete a document
     */
    delete(id: string): boolean {
        const result = this.db.delete(id);
        if (result) {
            this.vectorCount = this.db.len();
        }
        return result;
    }

    /**
     * Check if document exists
     */
    contains(id: string): boolean {
        return this.db.contains(id);
    }

    /**
     * Get vector count
     */
    len(): number {
        return this.vectorCount;
    }

    /**
     * Save to KV
     */
    async save(): Promise<void> {
        const kvKey = `db:${this.userId}:${this.dbId}`;
        const data = this.db.export();
        await this.kv.put(kvKey, JSON.stringify(data));
    }

    /**
     * Clear all data
     */
    clear(): void {
        this.db.clear();
        this.vectorCount = 0;
    }
}

/**
 * Simple in-memory vector database (placeholder until WASM is integrated)
 * This will be replaced with WasmVectorDB in production
 */
class InMemoryVectorDB {
    private dimensions: number;
    private vectors: Map<string, { vector: Float32Array; metadata?: any }> = new Map();
    private texts: Map<string, string> = new Map();

    constructor(dimensions: number) {
        this.dimensions = dimensions;
    }

    insert(id: string, vector: Float32Array, metadata?: any): void {
        this.vectors.set(id, { vector, metadata });
        if (metadata?._snippet) {
            this.texts.set(id, metadata._snippet);
        }
    }

    search(query: Float32Array, k: number): SearchResult[] {
        const results: Array<{ id: string; distance: number; metadata?: any }> = [];

        for (const [id, data] of this.vectors) {
            const distance = this.cosineDistance(query, data.vector);
            results.push({ id, distance, metadata: data.metadata });
        }

        results.sort((a, b) => a.distance - b.distance);
        return results.slice(0, k);
    }

    keywordSearch(query: string, k: number): KeywordResult[] {
        const terms = query.toLowerCase().split(/\s+/);
        const results: KeywordResult[] = [];

        for (const [id, text] of this.texts) {
            const lowerText = text.toLowerCase();
            let score = 0;
            for (const term of terms) {
                if (lowerText.includes(term)) {
                    score += 1;
                }
            }
            if (score > 0) {
                results.push({ id, score });
            }
        }

        results.sort((a, b) => b.score - a.score);
        return results.slice(0, k);
    }

    update(id: string, vector: Float32Array, metadata?: any): boolean {
        if (!this.vectors.has(id)) return false;
        this.insert(id, vector, metadata);
        return true;
    }

    delete(id: string): boolean {
        this.texts.delete(id);
        return this.vectors.delete(id);
    }

    contains(id: string): boolean {
        return this.vectors.has(id);
    }

    len(): number {
        return this.vectors.size;
    }

    clear(): void {
        this.vectors.clear();
        this.texts.clear();
    }

    export(): any {
        const data: any = { vectors: {}, texts: {} };
        for (const [id, { vector, metadata }] of this.vectors) {
            data.vectors[id] = { vector: Array.from(vector), metadata };
        }
        for (const [id, text] of this.texts) {
            data.texts[id] = text;
        }
        return data;
    }

    import(data: any): void {
        if (data.vectors) {
            for (const [id, { vector, metadata }] of Object.entries(data.vectors) as any) {
                this.vectors.set(id, { vector: new Float32Array(vector), metadata });
            }
        }
        if (data.texts) {
            for (const [id, text] of Object.entries(data.texts) as any) {
                this.texts.set(id, text);
            }
        }
    }

    private cosineDistance(a: Float32Array, b: Float32Array): number {
        let dot = 0, normA = 0, normB = 0;
        for (let i = 0; i < a.length; i++) {
            dot += a[i] * b[i];
            normA += a[i] * a[i];
            normB += b[i] * b[i];
        }
        const similarity = dot / (Math.sqrt(normA) * Math.sqrt(normB));
        return 1 - similarity;  // Convert to distance
    }
}
