/**
 * MemoryManager - Orchestrates memory operations
 */
import { VectorDB } from '../core/VectorDB.js';
/**
 * Generates a unique ID
 */
function generateId() {
    return `mem_${Date.now().toString(36)}_${Math.random().toString(36).substring(2, 9)}`;
}
/**
 * MemoryManager handles all memory operations
 */
export class MemoryManager {
    db;
    dimensions;
    textFields;
    decayRate;
    workingMemoryTTL;
    constructor(options) {
        this.dimensions = options.dimensions;
        this.textFields = options.textFields || ['content', 'event', 'fact', 'context'];
        this.decayRate = options.decayRate ?? 0.01;
        this.workingMemoryTTL = options.workingMemoryTTL ?? 3600000; // 1 hour
        this.db = new VectorDB({
            dimensions: options.dimensions,
            distance: 'cosine',
            indexType: 'flat',
        });
        // Configure BM25 for keyword search
        this.db.configureBM25({ textFields: this.textFields });
    }
    /**
     * Remember - Store a new memory
     */
    async remember(content, embedding, options = {}) {
        const now = Date.now();
        const id = generateId();
        const type = options.type || 'episodic';
        const importance = options.importance ?? 0.5;
        // Build the memory object
        const baseMemory = {
            id,
            type,
            content,
            embedding,
            metadata: options.metadata || {},
            importance,
            createdAt: now,
            updatedAt: now,
            accessedAt: now,
            accessCount: 0,
        };
        // Add type-specific fields to metadata
        const fullMetadata = {
            ...baseMemory.metadata,
            type,
            content,
            importance,
            createdAt: now,
            accessCount: 0,
        };
        if (type === 'working' && options.sessionId) {
            const ttl = options.ttl || this.workingMemoryTTL;
            fullMetadata.sessionId = options.sessionId;
            fullMetadata.ttl = ttl;
            fullMetadata.expiresAt = now + ttl;
        }
        // Store in vector database
        this.db.insert(id, embedding, fullMetadata);
        return baseMemory;
    }
    /**
     * Recall - Search for relevant memories
     */
    async recall(query, options = {}) {
        const limit = options.limit || 10;
        const mode = options.mode || 'hybrid';
        // Build search options
        const searchOptions = {
            mode,
            k: limit,
            minSimilarity: options.minSimilarity,
            alpha: options.alpha ?? 0.7, // Favor vector similarity by default
        };
        // Set query based on type
        if (typeof query === 'string') {
            searchOptions.keywords = query;
            // For keyword-only, we need keywords
            if (mode === 'keyword') {
                // Just keyword search
            }
            else {
                // For hybrid, we need an embedding - caller should provide
                throw new Error('Hybrid/vector search requires an embedding. Use recallWithEmbedding() or provide mode: "keyword"');
            }
        }
        else {
            searchOptions.queryVector = query;
            if (mode === 'hybrid') {
                throw new Error('Hybrid search requires keywords. Use recallWithEmbedding() with keywords option');
            }
        }
        // Build filter
        const filter = {};
        if (options.type) {
            filter.type = options.type;
        }
        if (options.minImportance !== undefined) {
            filter.importance = { $gte: options.minImportance };
        }
        if (options.sessionId) {
            filter.sessionId = options.sessionId;
        }
        // Filter out expired working memories
        filter.$or = [
            { type: { $ne: 'working' } },
            { expiresAt: { $gt: Date.now() } },
        ];
        if (Object.keys(filter).length > 1) {
            searchOptions.filter = filter;
        }
        const results = this.db.hybridSearch(searchOptions);
        // Update access counts
        const now = Date.now();
        for (const result of results) {
            const stored = this.db.get(result.id);
            if (stored?.metadata) {
                const newMetadata = {
                    ...stored.metadata,
                    accessedAt: now,
                    accessCount: (stored.metadata.accessCount || 0) + 1,
                };
                this.db.upsert(result.id, stored.vector, newMetadata);
            }
        }
        return results.map(r => ({
            memory: this.resultToMemory(r.id, r.metadata || null),
            score: r.score,
            vectorSimilarity: r.vectorSimilarity,
            keywordScore: r.keywordScore,
        }));
    }
    /**
     * Recall with embedding for hybrid search
     */
    async recallWithEmbedding(keywords, embedding, options = {}) {
        const limit = options.limit || 10;
        const mode = options.mode || 'hybrid';
        const searchOptions = {
            mode,
            k: limit,
            queryVector: embedding,
            keywords,
            minSimilarity: options.minSimilarity,
            alpha: options.alpha ?? 0.7,
        };
        // Build filter
        const filter = {};
        if (options.type) {
            filter.type = options.type;
        }
        if (options.minImportance !== undefined) {
            filter.importance = { $gte: options.minImportance };
        }
        if (options.sessionId) {
            filter.sessionId = options.sessionId;
        }
        if (Object.keys(filter).length > 0) {
            searchOptions.filter = filter;
        }
        const results = this.db.hybridSearch(searchOptions);
        return results.map(r => ({
            memory: this.resultToMemory(r.id, r.metadata || null),
            score: r.score,
            vectorSimilarity: r.vectorSimilarity,
            keywordScore: r.keywordScore,
        }));
    }
    /**
     * Forget - Delete a memory
     */
    async forget(id) {
        return this.db.delete(id);
    }
    /**
     * Forget by filter - Delete multiple memories
     */
    async forgetByFilter(filter) {
        const ids = this.db.getIds();
        let count = 0;
        for (const id of ids) {
            const stored = this.db.get(id);
            if (stored?.metadata) {
                // Simple filter matching
                let matches = true;
                for (const [key, value] of Object.entries(filter)) {
                    if (stored.metadata[key] !== value) {
                        matches = false;
                        break;
                    }
                }
                if (matches) {
                    this.db.delete(id);
                    count++;
                }
            }
        }
        return count;
    }
    /**
     * Get a specific memory by ID
     */
    async get(id) {
        const stored = this.db.get(id);
        if (!stored)
            return null;
        return this.resultToMemory(id, stored.metadata);
    }
    /**
     * Update a memory
     */
    async update(id, updates, newEmbedding) {
        const stored = this.db.get(id);
        if (!stored)
            return null;
        const now = Date.now();
        const newMetadata = {
            ...stored.metadata,
            ...updates.metadata,
            updatedAt: now,
        };
        if (updates.content !== undefined) {
            newMetadata.content = updates.content;
        }
        if (updates.importance !== undefined) {
            newMetadata.importance = updates.importance;
        }
        const embedding = newEmbedding || stored.vector;
        this.db.upsert(id, embedding, newMetadata);
        return this.resultToMemory(id, newMetadata);
    }
    /**
     * Apply decay to all memories
     */
    async applyDecay() {
        const ids = this.db.getIds();
        const now = Date.now();
        const dayInMs = 86400000;
        for (const id of ids) {
            const stored = this.db.get(id);
            if (!stored?.metadata)
                continue;
            const createdAt = stored.metadata.createdAt || now;
            const daysOld = (now - createdAt) / dayInMs;
            const currentImportance = stored.metadata.importance || 0.5;
            // Decay formula: importance * (1 - decayRate)^days
            const newImportance = Math.max(0, currentImportance * Math.pow(1 - this.decayRate, daysOld));
            if (newImportance !== currentImportance) {
                this.db.upsert(id, stored.vector, {
                    ...stored.metadata,
                    importance: newImportance,
                });
            }
        }
    }
    /**
     * Clean up expired working memories
     */
    async cleanupExpired() {
        const ids = this.db.getIds();
        const now = Date.now();
        let count = 0;
        for (const id of ids) {
            const stored = this.db.get(id);
            if (!stored?.metadata)
                continue;
            if (stored.metadata.type === 'working') {
                const expiresAt = stored.metadata.expiresAt;
                if (expiresAt && expiresAt < now) {
                    this.db.delete(id);
                    count++;
                }
            }
        }
        return count;
    }
    /**
     * Get memory statistics
     */
    async stats() {
        const ids = this.db.getIds();
        let episodic = 0;
        let semantic = 0;
        let working = 0;
        let totalImportance = 0;
        let oldest = Infinity;
        let newest = 0;
        let knowledge = 0;
        for (const id of ids) {
            const stored = this.db.get(id);
            if (!stored?.metadata)
                continue;
            const type = stored.metadata.type;
            if (type === 'episodic')
                episodic++;
            else if (type === 'semantic')
                semantic++;
            else if (type === 'working')
                working++;
            else if (type === 'knowledge')
                knowledge++;
            totalImportance += stored.metadata.importance || 0;
            const createdAt = stored.metadata.createdAt || Date.now();
            if (createdAt < oldest)
                oldest = createdAt;
            if (createdAt > newest)
                newest = createdAt;
        }
        return {
            total: ids.length,
            byType: { episodic, semantic, working, knowledge },
            averageImportance: ids.length > 0 ? totalImportance / ids.length : 0,
            oldestMemory: oldest !== Infinity ? oldest : undefined,
            newestMemory: newest > 0 ? newest : undefined,
        };
    }
    /**
     * Export all memories
     */
    export() {
        const ids = this.db.getIds();
        const memories = [];
        for (const id of ids) {
            const stored = this.db.get(id);
            if (stored?.metadata) {
                memories.push({
                    ...this.resultToMemory(id, stored.metadata),
                    embedding: stored.vector,
                });
            }
        }
        return { version: '1.0.0', memories };
    }
    /**
     * Import memories
     */
    import(data) {
        let count = 0;
        for (const memory of data.memories) {
            if (memory.embedding) {
                this.db.upsert(memory.id, memory.embedding, {
                    type: memory.type,
                    content: memory.content,
                    importance: memory.importance,
                    createdAt: memory.createdAt,
                    updatedAt: memory.updatedAt,
                    accessedAt: memory.accessedAt,
                    accessCount: memory.accessCount,
                    ...memory.metadata,
                });
                count++;
            }
        }
        return count;
    }
    /**
     * Clear all memories
     */
    clear() {
        this.db.clear();
    }
    /**
     * Convert stored result to Memory object
     */
    resultToMemory(id, metadata) {
        if (!metadata) {
            return {
                id,
                type: 'episodic',
                content: '',
                metadata: {},
                importance: 0,
                createdAt: Date.now(),
                updatedAt: Date.now(),
                accessedAt: Date.now(),
                accessCount: 0,
            };
        }
        return {
            id,
            type: metadata.type || 'episodic',
            content: metadata.content || '',
            metadata: Object.fromEntries(Object.entries(metadata).filter(([k]) => !['type', 'content', 'importance', 'createdAt', 'updatedAt', 'accessedAt', 'accessCount'].includes(k))),
            importance: metadata.importance || 0,
            createdAt: metadata.createdAt || Date.now(),
            updatedAt: metadata.updatedAt || Date.now(),
            accessedAt: metadata.accessedAt || Date.now(),
            accessCount: metadata.accessCount || 0,
        };
    }
}
//# sourceMappingURL=MemoryManager.js.map