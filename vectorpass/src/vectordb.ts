/**
 * VectorPass - Vector Database Operations
 *
 * Wraps minimemory WASM bindings with EmbeddingGemma integration
 */

import { Env, User, SearchResult, KeywordResult, TIER_LIMITS } from './types';

// WASM module will be imported after build
// import init, { WasmVectorDB } from '../pkg/minimemory.js';

// Configuration
const CONFIG = {
    dimensions: 256,        // Matryoshka truncated (768 -> 256)
    distance: "cosine",
    index: "hnsw",
    quantization: "int8",   // 4x compression
    model: "@cf/google/embeddinggemma-300m",
    hnswM: 16,
    hnswEf: 200,
    maxChunkChars: 1500,    // ~375 tokens, safe limit for embedding
    chunkOverlap: 200       // Overlap between chunks for context continuity
};

/**
 * Split text into chunks with overlap
 * Returns array of { text, startPos, endPos }
 */
function chunkText(text: string, maxChars: number = CONFIG.maxChunkChars, overlap: number = CONFIG.chunkOverlap): Array<{ text: string; start: number; end: number }> {
    // If text fits in one chunk, return as-is
    if (text.length <= maxChars) {
        return [{ text, start: 0, end: text.length }];
    }

    const chunks: Array<{ text: string; start: number; end: number }> = [];
    let pos = 0;

    while (pos < text.length) {
        let end = Math.min(pos + maxChars, text.length);

        // Try to break at sentence boundary (. ! ? followed by space)
        if (end < text.length) {
            const searchStart = Math.max(pos + maxChars - 200, pos);
            const searchText = text.slice(searchStart, end);
            const sentenceEnd = searchText.search(/[.!?]\s+[A-Z]/);

            if (sentenceEnd > 0) {
                end = searchStart + sentenceEnd + 2; // Include the punctuation and space
            } else {
                // Fall back to word boundary
                const spacePos = text.lastIndexOf(' ', end);
                if (spacePos > pos + maxChars / 2) {
                    end = spacePos;
                }
            }
        }

        chunks.push({
            text: text.slice(pos, end).trim(),
            start: pos,
            end
        });

        // Move position, accounting for overlap
        pos = end - overlap;
        if (pos >= text.length - overlap) {
            break;
        }
    }

    return chunks;
}

// Flag to track if we're using real WASM or fallback
let useWasm = false;
let WasmVectorDB: any = null;

/**
 * Initialize WASM module (call once at startup)
 */
export async function initWasm(): Promise<boolean> {
    try {
        // Dynamic import of WASM module
        const wasm = await import('../pkg/minimemory.js');
        await wasm.default();  // Initialize WASM
        WasmVectorDB = wasm.WasmVectorDB;
        useWasm = true;
        console.log('WASM initialized successfully');
        return true;
    } catch (e) {
        console.warn('WASM not available, using JS fallback:', e);
        useWasm = false;
        return false;
    }
}

/**
 * VectorDB wrapper with embedding generation
 */
export class VectorDB {
    private db: any;
    private ai: any;
    private kv: KVNamespace;
    private userId: string;
    private dbId: string;
    private vectorCount: number = 0;
    private isWasm: boolean;

    private constructor(
        db: any,
        ai: any,
        kv: KVNamespace,
        userId: string,
        dbId: string,
        vectorCount: number,
        isWasm: boolean
    ) {
        this.db = db;
        this.ai = ai;
        this.kv = kv;
        this.userId = userId;
        this.dbId = dbId;
        this.vectorCount = vectorCount;
        this.isWasm = isWasm;
    }

    /**
     * Creates or loads a vector database for user
     */
    static async create(
        user: User,
        dbId: string,
        env: Env
    ): Promise<VectorDB> {
        let db: any;
        let isWasm = false;

        // Try to use WASM if available
        if (useWasm && WasmVectorDB) {
            try {
                db = WasmVectorDB.new_int8(
                    CONFIG.dimensions,
                    CONFIG.distance,
                    CONFIG.index
                );
                isWasm = true;
            } catch (e) {
                console.warn('Failed to create WASM DB:', e);
            }
        }

        // Fallback to JS implementation
        if (!db) {
            db = new InMemoryVectorDB(CONFIG.dimensions);
        }

        const kvKey = `db:${user.id}:${dbId}`;

        // Try to restore from KV
        const saved = await env.VECTORS.get(kvKey);
        if (saved) {
            try {
                if (isWasm) {
                    db.import_json(saved);
                } else {
                    db.import(JSON.parse(saved));
                }
            } catch (e) {
                console.warn('Failed to restore DB from KV:', e);
            }
        }

        const count = isWasm ? db.len() : db.len();
        return new VectorDB(db, env.AI, env.VECTORS, user.id, dbId, count, isWasm);
    }

    /**
     * Generates embedding using EmbeddingGemma
     */
    private async embed(text: string): Promise<Float32Array> {
        const result = await this.ai.run(CONFIG.model, { text });
        return new Float32Array(result.data);
    }

    /**
     * Batch embedding generation
     */
    private async embedBatch(texts: string[]): Promise<Float32Array[]> {
        const result = await this.ai.run(CONFIG.model, { text: texts });

        // Handle different response formats
        if (Array.isArray(result.data[0])) {
            return result.data.map((arr: number[]) => new Float32Array(arr));
        }

        // Flat array - split by dimensions
        const embeddings: Float32Array[] = [];
        const fullDims = 768;
        for (let i = 0; i < texts.length; i++) {
            const start = i * fullDims;
            embeddings.push(new Float32Array(result.data.slice(start, start + fullDims)));
        }
        return embeddings;
    }

    /**
     * Index a document (with automatic chunking for long texts)
     */
    async index(id: string, text: string, metadata?: Record<string, any>): Promise<void> {
        const chunks = chunkText(text);

        // If single chunk, use original ID
        if (chunks.length === 1) {
            const embedding = await this.embed(text);
            const meta = metadata ? { ...metadata, _snippet: text.slice(0, 500) } : { _snippet: text.slice(0, 500) };

            if (this.isWasm) {
                this.db.insert_auto_with_metadata(id, embedding, JSON.stringify(meta));
            } else {
                const truncated = this.truncateAndNormalize(embedding);
                this.db.insert(id, truncated, meta);
            }
        } else {
            // Multiple chunks: store each with id:chunk_N format
            for (let i = 0; i < chunks.length; i++) {
                const chunk = chunks[i];
                const chunkId = `${id}:chunk_${i}`;
                const embedding = await this.embed(chunk.text);
                const meta = {
                    ...(metadata || {}),
                    _snippet: chunk.text.slice(0, 500),
                    _parentId: id,
                    _chunkIndex: i,
                    _totalChunks: chunks.length,
                    _charRange: [chunk.start, chunk.end]
                };

                if (this.isWasm) {
                    this.db.insert_auto_with_metadata(chunkId, embedding, JSON.stringify(meta));
                } else {
                    const truncated = this.truncateAndNormalize(embedding);
                    this.db.insert(chunkId, truncated, meta);
                }
            }
        }

        this.vectorCount = this.db.len();
    }

    /**
     * Batch index documents (with chunking support)
     */
    async indexBatch(items: Array<{ id: string; text: string; metadata?: Record<string, any> }>): Promise<number> {
        let totalVectors = 0;

        // Process each document - chunking if needed
        for (const item of items) {
            await this.index(item.id, item.text, item.metadata);
            totalVectors++;
        }

        this.vectorCount = this.db.len();
        return totalVectors;
    }

    /**
     * Semantic search
     */
    async search(query: string, k: number = 10): Promise<SearchResult[]> {
        const embedding = await this.embed(query);

        if (this.isWasm) {
            const resultsJson = this.db.search_auto(embedding, k);
            return JSON.parse(resultsJson);
        } else {
            const truncated = this.truncateAndNormalize(embedding);
            return this.db.search(truncated, k);
        }
    }

    /**
     * Keyword search (BM25)
     */
    keywordSearch(query: string, k: number = 10): KeywordResult[] {
        if (this.isWasm) {
            const resultsJson = this.db.keyword_search(query, k);
            return JSON.parse(resultsJson);
        } else {
            return this.db.keywordSearch(query, k);
        }
    }

    /**
     * Update a document
     */
    async update(id: string, text: string, metadata?: Record<string, any>): Promise<boolean> {
        if (!this.contains(id)) {
            return false;
        }

        const embedding = await this.embed(text);
        const meta = { ...(metadata || {}), _snippet: text.slice(0, 500) };

        if (this.isWasm) {
            this.db.update_auto_with_metadata(id, embedding, JSON.stringify(meta));
        } else {
            const truncated = this.truncateAndNormalize(embedding);
            this.db.update(id, truncated, meta);
        }

        return true;
    }

    /**
     * Delete a document (and all its chunks)
     */
    delete(id: string): boolean {
        let deleted = false;

        // Delete main document if exists
        if (this.db.contains(id)) {
            this.db.delete(id);
            deleted = true;
        }

        // Delete all chunks (id:chunk_0, id:chunk_1, etc.)
        for (let i = 0; i < 1000; i++) {
            const chunkId = `${id}:chunk_${i}`;
            if (this.db.contains(chunkId)) {
                this.db.delete(chunkId);
                deleted = true;
            } else {
                break; // No more chunks
            }
        }

        if (deleted) {
            this.vectorCount = this.db.len();
        }
        return deleted;
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

        if (this.isWasm) {
            const json = this.db.export_json();
            await this.kv.put(kvKey, json);
        } else {
            const data = this.db.export();
            await this.kv.put(kvKey, JSON.stringify(data));
        }
    }

    /**
     * Clear all data
     */
    clear(): void {
        this.db.clear();
        this.vectorCount = 0;
    }

    /**
     * Get database info
     */
    info(): { isWasm: boolean; dimensions: number; config: typeof CONFIG } {
        return {
            isWasm: this.isWasm,
            dimensions: CONFIG.dimensions,
            config: CONFIG
        };
    }

    /**
     * Truncates vector to target dimensions and L2 normalizes (for JS fallback)
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
}

/**
 * Simple in-memory vector database (JavaScript fallback)
 * Used when WASM module is not available
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
        const results: SearchResult[] = [];

        for (const [id, data] of this.vectors) {
            const distance = this.cosineDistance(query, data.vector);
            results.push({ id, distance, metadata: data.metadata });
        }

        results.sort((a, b) => a.distance - b.distance);
        return results.slice(0, k);
    }

    keywordSearch(query: string, k: number): KeywordResult[] {
        const terms = query.toLowerCase().split(/\s+/).filter(t => t.length > 0);
        const results: KeywordResult[] = [];

        for (const [id, text] of this.texts) {
            const lowerText = text.toLowerCase();
            let score = 0;

            for (const term of terms) {
                // Simple term frequency
                const regex = new RegExp(term, 'gi');
                const matches = lowerText.match(regex);
                if (matches) {
                    score += matches.length;
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
        this.clear();
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
        const denom = Math.sqrt(normA) * Math.sqrt(normB);
        if (denom < 1e-10) return 1;
        const similarity = dot / denom;
        return 1 - similarity;  // Convert to distance
    }
}
