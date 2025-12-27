/**
 * minimemory SDK Types
 */
type MemoryType = 'episodic' | 'semantic' | 'working' | 'knowledge';
type KnowledgeSourceType = 'document' | 'url' | 'api' | 'manual';
interface Memory {
    id: string;
    type: MemoryType;
    content: string;
    importance: number;
    metadata?: Record<string, unknown>;
    createdAt: number;
    lastAccessed?: number;
    accessCount?: number;
}
interface RecallResult {
    id: string;
    type: MemoryType;
    content: string;
    score: number;
    vectorSimilarity?: number;
    keywordScore?: number;
    importance: number;
    metadata?: Record<string, unknown>;
    createdAt: number;
    source?: {
        id: string;
        name: string;
        type: KnowledgeSourceType;
        url?: string;
        chunkIndex: number;
        totalChunks: number;
    };
}
interface MemoryStats {
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
}
interface RememberOptions {
    type?: MemoryType;
    importance?: number;
    metadata?: Record<string, unknown>;
    sessionId?: string;
    ttl?: number;
    embedding?: number[];
    generateEmbedding?: boolean;
}
interface RecallOptions {
    type?: MemoryType;
    limit?: number;
    minImportance?: number;
    minSimilarity?: number;
    sessionId?: string;
    mode?: 'vector' | 'keyword' | 'hybrid';
    alpha?: number;
    query?: string;
    keywords?: string;
    embedding?: number[];
}
interface ForgetFilter {
    type?: MemoryType;
    [key: string]: unknown;
}
interface UpdateOptions {
    content?: string;
    importance?: number;
    metadata?: Record<string, unknown>;
    embedding?: number[];
}
interface KnowledgeSource {
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
interface ChunkingOptions {
    chunkSize?: number;
    chunkOverlap?: number;
    separators?: string[];
    preserveParagraphs?: boolean;
}
interface IngestOptions {
    content: string;
    name: string;
    type?: KnowledgeSourceType;
    url?: string;
    mimeType?: string;
    metadata?: Record<string, unknown>;
    chunking?: ChunkingOptions;
    generateEmbeddings?: boolean;
}
interface IngestResult {
    sourceId: string;
    sourceName: string;
    chunksCreated: number;
    embeddingsGenerated: boolean;
    totalCharacters: number;
    averageChunkSize: number;
    durationMs: number;
}
interface KnowledgeChunk {
    id: string;
    content: string;
    chunkIndex: number;
    startOffset?: number;
    endOffset?: number;
    createdAt: number;
}
interface KnowledgeStats {
    totalSources: number;
    totalChunks: number;
    byType: Record<string, number>;
    averageChunksPerSource: number;
}
type EmbeddingDimensions = 768 | 512 | 256 | 128;
interface EmbedOptions {
    dimensions?: EmbeddingDimensions;
}
interface EmbedResult {
    embedding: number[];
    dimensions: number;
    model: string;
    truncated?: boolean;
}
interface EmbedBatchResult {
    embeddings: number[][];
    dimensions: number;
    model: string;
    count: number;
}
interface EmbedInfo {
    available: boolean;
    model: string;
    dimensions: {
        default: number;
        available: number[];
    };
    matryoshka: boolean;
    pricing: {
        perThousandNeurons: number;
        freeDaily: number;
    };
    estimatedCosts: Record<string, string>;
}
interface ApiResponse<T = unknown> {
    success: boolean;
    error?: string;
    data?: T;
}
interface RememberResponse {
    success: boolean;
    memory: Memory;
    embeddingGenerated: boolean;
    persisted: boolean;
}
interface RecallResponse {
    success: boolean;
    count: number;
    embeddingGenerated: boolean;
    results: RecallResult[];
}
interface ForgetResponse {
    success: boolean;
    message: string;
    count?: number;
}
interface StatsResponse {
    success: boolean;
    namespace: string;
    stats: MemoryStats;
    source: 'd1' | 'memory';
}
interface ExportResponse {
    success: boolean;
    namespace: string;
    data: {
        memories: Memory[];
    };
    source: 'd1' | 'memory';
}
interface ImportResponse {
    success: boolean;
    count: number;
    message: string;
}
interface MiniMemoryConfig {
    baseUrl: string;
    namespace?: string;
    apiKey?: string;
    accessToken?: string;
    timeout?: number;
    headers?: Record<string, string>;
}
interface RequestOptions {
    namespace?: string;
    headers?: Record<string, string>;
    timeout?: number;
}
type AgentPermission = 'read' | 'write';
interface AgentToken {
    id: string;
    userId: string;
    tenantId?: string;
    name: string;
    description?: string;
    allowedMemories: string[];
    permissions: AgentPermission[];
    isActive: boolean;
    lastUsedAt?: number;
    useCount: number;
    expiresAt?: number;
    createdAt: number;
    updatedAt: number;
}
interface CreateAgentTokenOptions {
    name: string;
    description?: string;
    allowedMemories?: string[];
    permissions?: AgentPermission[];
    expiresAt?: number;
}
interface UpdateAgentTokenOptions {
    name?: string;
    description?: string;
    allowedMemories?: string[];
    permissions?: AgentPermission[];
    isActive?: boolean;
    expiresAt?: number | null;
}
interface AgentTokenListOptions {
    active?: boolean;
    limit?: number;
    offset?: number;
}
interface AgentTokenListResponse {
    tokens: AgentToken[];
    total: number;
    hasMore: boolean;
}
interface AgentTokenStats {
    total: number;
    active: number;
    inactive: number;
    expired: number;
    totalUseCount: number;
}
interface AgentValidationResult {
    valid: boolean;
    userId?: string;
    tenantId?: string;
    agentTokenId?: string;
    agentName?: string;
    allowedMemories?: string[];
    permissions?: AgentPermission[];
    expiresAt?: number | null;
    error?: string;
}

/**
 * minimemory SDK Client
 *
 * A type-safe client for interacting with the minimemory vector database service.
 */

declare class MiniMemoryError extends Error {
    status?: number | undefined;
    code?: string | undefined;
    constructor(message: string, status?: number | undefined, code?: string | undefined);
}
declare class MiniMemoryClient {
    private baseUrl;
    private namespace;
    private apiKey?;
    private accessToken?;
    private timeout;
    private defaultHeaders;
    constructor(config: MiniMemoryConfig);
    /**
     * Set the active namespace for subsequent operations
     */
    setNamespace(namespace: string): this;
    /**
     * Set API key for authentication
     */
    setApiKey(apiKey: string): this;
    /**
     * Set JWT access token for authentication
     */
    setAccessToken(token: string): this;
    private request;
    /**
     * Store a new memory
     *
     * @param content - The text content to remember
     * @param options - Optional settings for the memory
     * @returns The created memory with metadata
     *
     * @example
     * ```ts
     * const result = await client.remember('User prefers dark mode', {
     *   type: 'semantic',
     *   importance: 0.8,
     *   metadata: { category: 'preferences' }
     * });
     * ```
     */
    remember(content: string, options?: RememberOptions): Promise<RememberResponse>;
    /**
     * Search for relevant memories
     *
     * @param queryOrOptions - Search query string or full options object
     * @param options - Additional options (if first param is query string)
     * @returns Array of matching memories with scores
     *
     * @example
     * ```ts
     * // Simple query
     * const results = await client.recall('user preferences');
     *
     * // With options
     * const results = await client.recall('dark mode', {
     *   type: 'semantic',
     *   limit: 5,
     *   mode: 'hybrid'
     * });
     * ```
     */
    recall(queryOrOptions: string | RecallOptions, options?: Omit<RecallOptions, 'query'>): Promise<RecallResponse>;
    /**
     * Delete a specific memory by ID
     *
     * @param id - Memory ID to delete
     * @returns Success status and message
     */
    forget(id: string): Promise<ForgetResponse>;
    /**
     * Delete memories matching a filter
     *
     * @param filter - Filter criteria (e.g., { type: 'working' })
     * @returns Count of deleted memories
     */
    forgetByFilter(filter: ForgetFilter): Promise<ForgetResponse>;
    /**
     * Get a specific memory by ID
     *
     * @param id - Memory ID to retrieve
     * @returns The memory object or null if not found
     */
    get(id: string): Promise<{
        success: boolean;
        memory: Memory;
        source: string;
    }>;
    /**
     * Update a memory's content or metadata
     *
     * @param id - Memory ID to update
     * @param updates - Fields to update
     * @returns The updated memory
     */
    update(id: string, updates: UpdateOptions): Promise<{
        success: boolean;
        memory: Memory;
    }>;
    /**
     * Get memory statistics for the namespace
     *
     * @returns Statistics including counts by type, importance averages, etc.
     */
    stats(): Promise<StatsResponse>;
    /**
     * Clean up expired working memories
     *
     * @returns Count of memories removed
     */
    cleanup(): Promise<{
        success: boolean;
        count: number;
        message: string;
    }>;
    /**
     * Apply importance decay to all memories
     *
     * @returns Success status
     */
    decay(): Promise<{
        success: boolean;
        message: string;
    }>;
    /**
     * Export all memories from the namespace
     *
     * @returns All memories in exportable format
     */
    export(): Promise<ExportResponse>;
    /**
     * Import memories into the namespace
     *
     * @param memories - Array of memories to import
     * @returns Count of imported memories
     */
    import(memories: Memory[]): Promise<ImportResponse>;
    /**
     * Clear all memories in the namespace
     *
     * @returns Success status
     */
    clear(): Promise<{
        success: boolean;
        message: string;
    }>;
    /**
     * Ingest a document into the knowledge bank
     *
     * @param options - Document content and ingestion options
     * @returns Ingestion result with source ID and chunk count
     *
     * @example
     * ```ts
     * const result = await client.knowledge.ingest({
     *   content: documentText,
     *   name: 'product-manual.md',
     *   type: 'document',
     *   chunking: { chunkSize: 1000, chunkOverlap: 200 }
     * });
     * ```
     */
    get knowledge(): {
        ingest: (options: IngestOptions) => Promise<IngestResult & {
            success: boolean;
        }>;
        /**
         * List all knowledge sources
         */
        listSources: (options?: {
            type?: string;
            limit?: number;
            offset?: number;
        }) => Promise<{
            success: boolean;
            sources: KnowledgeSource[];
            total: number;
            hasMore: boolean;
        }>;
        /**
         * Get a specific knowledge source
         */
        getSource: (id: string) => Promise<{
            success: boolean;
            source: KnowledgeSource;
        }>;
        /**
         * Delete a knowledge source and its chunks
         */
        deleteSource: (id: string) => Promise<{
            success: boolean;
            message: string;
        }>;
        /**
         * Get chunks for a specific source
         */
        getChunks: (sourceId: string, options?: {
            limit?: number;
            offset?: number;
        }) => Promise<{
            success: boolean;
            source: {
                id: string;
                name: string;
                type: string;
            };
            chunks: KnowledgeChunk[];
            total: number;
            hasMore: boolean;
        }>;
        /**
         * Get knowledge bank statistics
         */
        stats: () => Promise<{
            success: boolean;
            namespace: string;
            stats: KnowledgeStats;
        }>;
        /**
         * Preview how content will be chunked
         */
        previewChunking: (content: string, options?: ChunkingOptions) => Promise<{
            success: boolean;
            totalChunks: number;
            totalCharacters: number;
            averageChunkSize: number;
            chunks: Array<{
                index: number;
                length: number;
                preview: string;
                startOffset: number;
                endOffset: number;
            }>;
        }>;
    };
    /**
     * Generate embeddings using Workers AI
     */
    get embed(): {
        /**
         * Generate embedding for a single text
         */
        single: (text: string, options?: EmbedOptions) => Promise<EmbedResult & {
            success: boolean;
        }>;
        /**
         * Generate embeddings for multiple texts
         */
        batch: (texts: string[], options?: EmbedOptions) => Promise<EmbedBatchResult & {
            success: boolean;
        }>;
        /**
         * Get embedding service info
         */
        info: () => Promise<EmbedInfo>;
    };
    /**
     * Agent token management for MCP access control.
     * Requires JWT authentication (use setAccessToken).
     */
    get agentTokens(): {
        /**
         * List all agent tokens for the authenticated user
         *
         * @param options - Optional filter and pagination
         * @returns List of agent tokens with pagination info
         */
        list: (options?: AgentTokenListOptions) => Promise<AgentTokenListResponse>;
        /**
         * Create a new agent token
         *
         * @param options - Token configuration
         * @returns The created token with its ID
         *
         * @example
         * ```ts
         * const result = await client.agentTokens.create({
         *   name: 'Work Assistant',
         *   permissions: ['read', 'write'],
         *   allowedMemories: ['*']
         * });
         * console.log(result.token.id); // at_xxx
         * ```
         */
        create: (options: CreateAgentTokenOptions) => Promise<{
            token: AgentToken;
            message: string;
        }>;
        /**
         * Get a specific agent token by ID
         *
         * @param id - Agent token ID
         * @returns The agent token
         */
        get: (id: string) => Promise<{
            token: AgentToken;
        }>;
        /**
         * Update an agent token
         *
         * @param id - Agent token ID
         * @param updates - Fields to update
         * @returns The updated token
         */
        update: (id: string, updates: UpdateAgentTokenOptions) => Promise<{
            token: AgentToken;
        }>;
        /**
         * Delete an agent token
         *
         * @param id - Agent token ID
         * @returns Success status
         */
        delete: (id: string) => Promise<{
            success: boolean;
        }>;
        /**
         * Toggle agent token active status
         *
         * @param id - Agent token ID
         * @returns The toggled token with new status
         */
        toggle: (id: string) => Promise<{
            token: AgentToken;
            message: string;
        }>;
        /**
         * Add a memory ID to the token's allowed list
         *
         * @param id - Agent token ID
         * @param memoryId - Memory ID to add
         * @returns The updated token
         */
        addMemory: (id: string, memoryId: string) => Promise<{
            token: AgentToken;
        }>;
        /**
         * Remove a memory ID from the token's allowed list
         *
         * @param id - Agent token ID
         * @param memoryId - Memory ID to remove
         * @returns The updated token
         */
        removeMemory: (id: string, memoryId: string) => Promise<{
            token: AgentToken;
        }>;
        /**
         * Get usage statistics for the user's tokens
         *
         * @returns Aggregated statistics
         */
        stats: () => Promise<{
            stats: AgentTokenStats;
        }>;
        /**
         * Validate an API key + agent token combination.
         * Used by MCP servers to verify agent credentials.
         *
         * @param apiKey - The API key
         * @param agentToken - The agent token ID
         * @returns Validation result with permissions
         */
        validate: (apiKey: string, agentToken: string) => Promise<AgentValidationResult>;
    };
}
/**
 * Create a new MiniMemory client
 *
 * @param config - Client configuration
 * @returns Configured client instance
 *
 * @example
 * ```ts
 * import { createClient } from '@minimemory/sdk';
 *
 * const client = createClient({
 *   baseUrl: 'https://your-worker.workers.dev',
 *   apiKey: 'mm_your_api_key',
 *   namespace: 'my-app'
 * });
 *
 * // Store a memory
 * await client.remember('Important fact', { type: 'semantic' });
 *
 * // Search memories
 * const results = await client.recall('important');
 * ```
 */
declare function createClient(config: MiniMemoryConfig): MiniMemoryClient;

export { type AgentPermission, type AgentToken, type AgentTokenListOptions, type AgentTokenListResponse, type AgentTokenStats, type AgentValidationResult, type ApiResponse, type ChunkingOptions, type CreateAgentTokenOptions, type EmbedBatchResult, type EmbedInfo, type EmbedOptions, type EmbedResult, type EmbeddingDimensions, type ExportResponse, type ForgetFilter, type ForgetResponse, type ImportResponse, type IngestOptions, type IngestResult, type KnowledgeChunk, type KnowledgeSource, type KnowledgeSourceType, type KnowledgeStats, type Memory, type MemoryStats, type MemoryType, MiniMemoryClient, type MiniMemoryConfig, MiniMemoryError, type RecallOptions, type RecallResponse, type RecallResult, type RememberOptions, type RememberResponse, type RequestOptions, type StatsResponse, type UpdateAgentTokenOptions, type UpdateOptions, createClient };
