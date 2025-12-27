/**
 * D1 Storage Adapter for minimemory-service
 * Provides persistent storage for memories using Cloudflare D1
 */
import type { D1Database } from '@cloudflare/workers-types';
export interface StoredMemory {
    id: string;
    namespace: string;
    type: 'episodic' | 'semantic' | 'working' | 'knowledge';
    content: string;
    embedding: number[];
    importance: number;
    metadata: Record<string, unknown>;
    sessionId?: string;
    ttl?: number;
    createdAt: number;
    updatedAt: number;
    lastAccessed?: number;
    accessCount: number;
    sourceId?: string;
    sourceName?: string;
    sourceType?: string;
    chunkIndex?: number;
    totalChunks?: number;
}
export interface NamespaceConfig {
    name: string;
    dimensions: number;
    createdAt: number;
    updatedAt: number;
}
export declare class D1Storage {
    private db;
    constructor(db: D1Database);
    getNamespace(name: string): Promise<NamespaceConfig | null>;
    createNamespace(name: string, dimensions: number): Promise<NamespaceConfig>;
    listNamespaces(): Promise<NamespaceConfig[]>;
    deleteNamespace(name: string): Promise<boolean>;
    saveMemory(memory: StoredMemory): Promise<void>;
    getMemory(namespace: string, id: string): Promise<StoredMemory | null>;
    getAllMemories(namespace: string): Promise<StoredMemory[]>;
    getMemoriesByType(namespace: string, type: string): Promise<StoredMemory[]>;
    deleteMemory(namespace: string, id: string): Promise<boolean>;
    deleteMemoriesByType(namespace: string, type: string): Promise<number>;
    clearNamespace(namespace: string): Promise<number>;
    updateMemory(namespace: string, id: string, updates: Partial<StoredMemory>): Promise<boolean>;
    getStats(namespace: string): Promise<{
        total: number;
        byType: Record<string, number>;
        averageImportance: number;
        oldestMemory?: number;
        newestMemory?: number;
        knowledgeSources?: number;
    }>;
    cleanupExpired(namespace: string): Promise<number>;
    applyDecay(namespace: string, decayRate?: number): Promise<number>;
    validateApiKey(key: string): Promise<{
        valid: boolean;
        userId?: string;
        namespace?: string;
        permissions?: string[];
        rateLimit?: {
            limit: number;
            window: number;
        };
    } | null>;
    private rowToMemory;
}
//# sourceMappingURL=D1Storage.d.ts.map