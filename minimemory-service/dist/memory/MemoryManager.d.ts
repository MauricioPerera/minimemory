/**
 * MemoryManager - Orchestrates memory operations
 */
import type { Memory, RememberOptions, RecallOptions, RecallResult, MemoryStats } from './types.js';
export interface MemoryManagerOptions {
    dimensions: number;
    textFields?: string[];
    decayRate?: number;
    workingMemoryTTL?: number;
}
/**
 * MemoryManager handles all memory operations
 */
export declare class MemoryManager {
    private db;
    private dimensions;
    private textFields;
    private decayRate;
    private workingMemoryTTL;
    constructor(options: MemoryManagerOptions);
    /**
     * Remember - Store a new memory
     */
    remember(content: string, embedding: number[], options?: RememberOptions): Promise<Memory>;
    /**
     * Recall - Search for relevant memories
     */
    recall(query: string | number[], options?: RecallOptions): Promise<RecallResult[]>;
    /**
     * Recall with embedding for hybrid search
     */
    recallWithEmbedding(keywords: string, embedding: number[], options?: RecallOptions): Promise<RecallResult[]>;
    /**
     * Forget - Delete a memory
     */
    forget(id: string): Promise<boolean>;
    /**
     * Forget by filter - Delete multiple memories
     */
    forgetByFilter(filter: Record<string, unknown>): Promise<number>;
    /**
     * Get a specific memory by ID
     */
    get(id: string): Promise<Memory | null>;
    /**
     * Update a memory
     */
    update(id: string, updates: Partial<{
        content: string;
        importance: number;
        metadata: Record<string, unknown>;
    }>, newEmbedding?: number[]): Promise<Memory | null>;
    /**
     * Apply decay to all memories
     */
    applyDecay(): Promise<void>;
    /**
     * Clean up expired working memories
     */
    cleanupExpired(): Promise<number>;
    /**
     * Get memory statistics
     */
    stats(): Promise<MemoryStats>;
    /**
     * Export all memories
     */
    export(): {
        version: string;
        memories: Memory[];
    };
    /**
     * Import memories
     */
    import(data: {
        memories: Memory[];
    }): number;
    /**
     * Clear all memories
     */
    clear(): void;
    /**
     * Convert stored result to Memory object
     */
    private resultToMemory;
}
//# sourceMappingURL=MemoryManager.d.ts.map