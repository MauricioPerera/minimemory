/**
 * minimemory SDK Types
 */

// ============ Memory Types ============

export type MemoryType = 'episodic' | 'semantic' | 'working' | 'knowledge';
export type KnowledgeSourceType = 'document' | 'url' | 'api' | 'manual';

export interface Memory {
	id: string;
	type: MemoryType;
	content: string;
	importance: number;
	metadata?: Record<string, unknown>;
	createdAt: number;
	lastAccessed?: number;
	accessCount?: number;
}

export interface RecallResult {
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
}

// ============ Request Options ============

export interface RememberOptions {
	type?: MemoryType;
	importance?: number;
	metadata?: Record<string, unknown>;
	sessionId?: string;
	ttl?: number;
	embedding?: number[];
	generateEmbedding?: boolean;
}

export interface RecallOptions {
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

export interface ForgetFilter {
	type?: MemoryType;
	[key: string]: unknown;
}

export interface UpdateOptions {
	content?: string;
	importance?: number;
	metadata?: Record<string, unknown>;
	embedding?: number[];
}

// ============ Knowledge Types ============

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

export interface ChunkingOptions {
	chunkSize?: number;
	chunkOverlap?: number;
	separators?: string[];
	preserveParagraphs?: boolean;
}

export interface IngestOptions {
	content: string;
	name: string;
	type?: KnowledgeSourceType;
	url?: string;
	mimeType?: string;
	metadata?: Record<string, unknown>;
	chunking?: ChunkingOptions;
	generateEmbeddings?: boolean;
}

export interface IngestResult {
	sourceId: string;
	sourceName: string;
	chunksCreated: number;
	embeddingsGenerated: boolean;
	totalCharacters: number;
	averageChunkSize: number;
	durationMs: number;
}

export interface KnowledgeChunk {
	id: string;
	content: string;
	chunkIndex: number;
	startOffset?: number;
	endOffset?: number;
	createdAt: number;
}

export interface KnowledgeStats {
	totalSources: number;
	totalChunks: number;
	byType: Record<string, number>;
	averageChunksPerSource: number;
}

// ============ Embedding Types ============

export type EmbeddingDimensions = 768 | 512 | 256 | 128;

export interface EmbedOptions {
	dimensions?: EmbeddingDimensions;
}

export interface EmbedResult {
	embedding: number[];
	dimensions: number;
	model: string;
	truncated?: boolean;
}

export interface EmbedBatchResult {
	embeddings: number[][];
	dimensions: number;
	model: string;
	count: number;
}

export interface EmbedInfo {
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

// ============ API Response Types ============

export interface ApiResponse<T = unknown> {
	success: boolean;
	error?: string;
	data?: T;
}

export interface RememberResponse {
	success: boolean;
	memory: Memory;
	embeddingGenerated: boolean;
	persisted: boolean;
}

export interface RecallResponse {
	success: boolean;
	count: number;
	embeddingGenerated: boolean;
	results: RecallResult[];
}

export interface ForgetResponse {
	success: boolean;
	message: string;
	count?: number;
}

export interface StatsResponse {
	success: boolean;
	namespace: string;
	stats: MemoryStats;
	source: 'd1' | 'memory';
}

export interface ExportResponse {
	success: boolean;
	namespace: string;
	data: {
		memories: Memory[];
	};
	source: 'd1' | 'memory';
}

export interface ImportResponse {
	success: boolean;
	count: number;
	message: string;
}

// ============ Client Configuration ============

export interface MiniMemoryConfig {
	baseUrl: string;
	namespace?: string;
	apiKey?: string;
	accessToken?: string;
	timeout?: number;
	headers?: Record<string, string>;
}

export interface RequestOptions {
	namespace?: string;
	headers?: Record<string, string>;
	timeout?: number;
}

// ============ Agent Token Types ============

export type AgentPermission = 'read' | 'write';

export interface AgentToken {
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

export interface CreateAgentTokenOptions {
	name: string;
	description?: string;
	allowedMemories?: string[];
	permissions?: AgentPermission[];
	expiresAt?: number;
}

export interface UpdateAgentTokenOptions {
	name?: string;
	description?: string;
	allowedMemories?: string[];
	permissions?: AgentPermission[];
	isActive?: boolean;
	expiresAt?: number | null;
}

export interface AgentTokenListOptions {
	active?: boolean;
	limit?: number;
	offset?: number;
}

export interface AgentTokenListResponse {
	tokens: AgentToken[];
	total: number;
	hasMore: boolean;
}

export interface AgentTokenStats {
	total: number;
	active: number;
	inactive: number;
	expired: number;
	totalUseCount: number;
}

export interface AgentValidationResult {
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
