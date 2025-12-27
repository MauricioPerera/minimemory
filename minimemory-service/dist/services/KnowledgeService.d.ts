/**
 * KnowledgeService - Document ingestion and RAG knowledge management
 *
 * Handles:
 * - Document chunking with overlap
 * - Source tracking and citation
 * - Integration with EmbeddingService for auto-embeddings
 * - Knowledge search with source attribution
 */
import type { D1Database } from '@cloudflare/workers-types';
import type { KnowledgeSource, KnowledgeSourceType, ChunkingOptions, IngestRequest, IngestResult } from '../memory/types.js';
/**
 * Represents a chunk of text from a document
 */
export interface TextChunk {
    text: string;
    index: number;
    startOffset: number;
    endOffset: number;
    metadata?: Record<string, unknown>;
}
/**
 * Result of a knowledge search with source attribution
 */
export interface KnowledgeSearchResult {
    id: string;
    content: string;
    score: number;
    source: {
        id: string;
        name: string;
        type: KnowledgeSourceType;
        url?: string;
        chunkIndex: number;
        totalChunks: number;
    };
    metadata: Record<string, unknown>;
}
/**
 * Service configuration
 */
export interface KnowledgeServiceConfig {
    enabled?: boolean;
    defaultChunkSize?: number;
    defaultChunkOverlap?: number;
    maxChunksPerDocument?: number;
}
/**
 * KnowledgeService class for managing RAG knowledge banks
 */
export declare class KnowledgeService {
    private db;
    private config;
    constructor(db: D1Database | null, config?: KnowledgeServiceConfig);
    /**
     * Check if the service is available
     */
    isAvailable(): boolean;
    /**
     * Chunk text into overlapping segments
     */
    chunkText(text: string, options?: ChunkingOptions): TextChunk[];
    /**
     * Create a new knowledge source
     */
    createSource(namespace: string, source: Omit<KnowledgeSource, 'id' | 'createdAt' | 'updatedAt'>): Promise<KnowledgeSource>;
    /**
     * Get a knowledge source by ID
     */
    getSource(id: string): Promise<KnowledgeSource | null>;
    /**
     * List all knowledge sources in a namespace
     */
    listSources(namespace: string, options?: {
        type?: KnowledgeSourceType;
        limit?: number;
        offset?: number;
    }): Promise<{
        sources: KnowledgeSource[];
        total: number;
    }>;
    /**
     * Delete a knowledge source and all its chunks
     */
    deleteSource(id: string): Promise<boolean>;
    /**
     * Update chunk count for a source
     */
    updateSourceChunkCount(id: string, chunkCount: number): Promise<void>;
    /**
     * Get knowledge stats for a namespace
     */
    getStats(namespace: string): Promise<{
        totalSources: number;
        totalChunks: number;
        byType: Record<KnowledgeSourceType, number>;
        totalSize: number;
    }>;
    private rowToSource;
}
export { KnowledgeSourceType, ChunkingOptions, IngestRequest, IngestResult };
//# sourceMappingURL=KnowledgeService.d.ts.map