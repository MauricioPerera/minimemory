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
	importance: number;        // 0-1 scale for decay/consolidation
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
	event: string;             // What happened
	context?: string;          // Where/when context
	participants?: string[];   // Who was involved
	outcome?: string;          // Result of the event
}

/**
 * Semantic memory - facts and knowledge
 */
export interface SemanticMemory extends Memory {
	type: 'semantic';
	fact: string;              // The knowledge
	category?: string;         // Topic category
	confidence: number;        // 0-1 confidence in the fact
	sources: string[];         // IDs of episodic memories that support this
}

/**
 * Working memory - temporary context for current task
 */
export interface WorkingMemory extends Memory {
	type: 'working';
	sessionId: string;         // Current session
	ttl: number;               // Time to live in ms
	expiresAt: number;         // When it expires
}

/**
 * Knowledge memory - RAG knowledge bank chunks
 */
export interface KnowledgeMemory extends Memory {
	type: 'knowledge';
	sourceId: string;          // ID of the source document/URL
	sourceType: KnowledgeSourceType;
	sourceName: string;        // Original filename or URL
	chunkIndex: number;        // Position in source (0-based)
	totalChunks: number;       // Total chunks in source
	startOffset?: number;      // Character offset in source
	endOffset?: number;        // End character offset
	title?: string;            // Section/chunk title
	sourceUrl?: string;        // URL for retrieval
	mimeType?: string;         // Content type (text/plain, text/markdown, etc.)
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
	size?: number;             // Original content size in bytes
	chunkCount: number;        // Number of chunks created
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
	sessionId?: string;        // For working memory
	ttl?: number;              // For working memory (default: 1 hour)
	// For knowledge memory
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
	sessionId?: string;        // Filter by session
	filter?: Record<string, unknown>;
	mode?: 'vector' | 'keyword' | 'hybrid';
	alpha?: number;            // For hybrid search
	sourceId?: string;         // Filter by knowledge source
	includeSource?: boolean;   // Include source metadata in results
}

/**
 * Recall result
 */
export interface RecallResult {
	memory: Memory;
	score: number;
	vectorSimilarity?: number;
	keywordScore?: number;
	// Source citation for knowledge memory
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
	// Knowledge-specific stats
	knowledgeSources?: number;
	knowledgeChunks?: number;
}

/**
 * Chunking options for document ingestion
 */
export interface ChunkingOptions {
	chunkSize?: number;        // Target chunk size in characters (default: 1000)
	chunkOverlap?: number;     // Overlap between chunks (default: 200)
	separators?: string[];     // Custom separators for splitting
	preserveParagraphs?: boolean; // Try to split at paragraph boundaries
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
