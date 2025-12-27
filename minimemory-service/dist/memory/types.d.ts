/**
 * Memory types for agentic memory service
 */
export type MemoryType = 'episodic' | 'semantic' | 'working' | 'knowledge';
/**
 * Source types for knowledge memory
 */
export type KnowledgeSourceType = 'document' | 'url' | 'api' | 'manual';
/**
 * Base memory interface
 */
export interface Memory {
    id: string;
    type: MemoryType;
    content: string;
    embedding?: number[];
    metadata: Record<string, unknown>;
    importance: number;
    createdAt: number;
    updatedAt: number;
    accessedAt: number;
    accessCount: number;
}
/**
 * Episodic memory - specific events/experiences
 */
export interface EpisodicMemory extends Memory {
    type: 'episodic';
    event: string;
    context?: string;
    participants?: string[];
    outcome?: string;
}
/**
 * Semantic memory - facts and knowledge
 */
export interface SemanticMemory extends Memory {
    type: 'semantic';
    fact: string;
    category?: string;
    confidence: number;
    sources: string[];
}
/**
 * Working memory - temporary context for current task
 */
export interface WorkingMemory extends Memory {
    type: 'working';
    sessionId: string;
    ttl: number;
    expiresAt: number;
}
/**
 * Knowledge memory - RAG knowledge bank chunks
 */
export interface KnowledgeMemory extends Memory {
    type: 'knowledge';
    sourceId: string;
    sourceType: KnowledgeSourceType;
    sourceName: string;
    chunkIndex: number;
    totalChunks: number;
    startOffset?: number;
    endOffset?: number;
    title?: string;
    sourceUrl?: string;
    mimeType?: string;
}
/**
 * Source document metadata
 */
export interface KnowledgeSource {
    id: string;
    name: string;
    type: KnowledgeSourceType;
    url?: string;
    mimeType?: string;
    size?: number;
    chunkCount: number;
    namespace: string;
    metadata: Record<string, unknown>;
    createdAt: number;
    updatedAt: number;
}
/**
 * Options for remembering
 */
export interface RememberOptions {
    type?: MemoryType;
    importance?: number;
    metadata?: Record<string, unknown>;
    sessionId?: string;
    ttl?: number;
    sourceId?: string;
    sourceType?: KnowledgeSourceType;
    sourceName?: string;
    chunkIndex?: number;
    totalChunks?: number;
}
/**
 * Options for recalling
 */
export interface RecallOptions {
    type?: MemoryType;
    limit?: number;
    minImportance?: number;
    minSimilarity?: number;
    sessionId?: string;
    filter?: Record<string, unknown>;
    mode?: 'vector' | 'keyword' | 'hybrid';
    alpha?: number;
    sourceId?: string;
    includeSource?: boolean;
}
/**
 * Recall result
 */
export interface RecallResult {
    memory: Memory;
    score: number;
    vectorSimilarity?: number;
    keywordScore?: number;
    source?: {
        id: string;
        name: string;
        type: KnowledgeSourceType;
        url?: string;
        chunkIndex: number;
        totalChunks: number;
    };
}
/**
 * Memory statistics
 */
export interface MemoryStats {
    total: number;
    byType: {
        episodic: number;
        semantic: number;
        working: number;
        knowledge: number;
    };
    averageImportance: number;
    oldestMemory?: number;
    newestMemory?: number;
    knowledgeSources?: number;
    knowledgeChunks?: number;
}
/**
 * Chunking options for document ingestion
 */
export interface ChunkingOptions {
    chunkSize?: number;
    chunkOverlap?: number;
    separators?: string[];
    preserveParagraphs?: boolean;
}
/**
 * Document ingestion request
 */
export interface IngestRequest {
    content: string;
    name: string;
    type?: KnowledgeSourceType;
    url?: string;
    mimeType?: string;
    metadata?: Record<string, unknown>;
    chunking?: ChunkingOptions;
    generateEmbeddings?: boolean;
}
/**
 * Document ingestion result
 */
export interface IngestResult {
    sourceId: string;
    sourceName: string;
    chunksCreated: number;
    embeddingsGenerated: boolean;
    totalCharacters: number;
    averageChunkSize: number;
}
//# sourceMappingURL=types.d.ts.map