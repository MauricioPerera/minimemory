/**
 * @minimemory/sdk
 *
 * JavaScript/TypeScript SDK for minimemory vector database service.
 *
 * @example
 * ```ts
 * import { createClient } from '@minimemory/sdk';
 *
 * const client = createClient({
 *   baseUrl: 'https://your-worker.workers.dev',
 *   apiKey: 'mm_your_api_key'
 * });
 *
 * // Store memories
 * await client.remember('User prefers dark mode', {
 *   type: 'semantic',
 *   importance: 0.8
 * });
 *
 * // Search with hybrid (vector + keyword) search
 * const results = await client.recall('user preferences', {
 *   mode: 'hybrid',
 *   limit: 5
 * });
 *
 * // Ingest documents for RAG
 * await client.knowledge.ingest({
 *   content: documentText,
 *   name: 'guide.md',
 *   chunking: { chunkSize: 1000 }
 * });
 * ```
 *
 * @packageDocumentation
 */

// Client
export { MiniMemoryClient, MiniMemoryError, createClient } from './client.js';

// Types
export type {
	// Memory types
	MemoryType,
	KnowledgeSourceType,
	Memory,
	RecallResult,
	MemoryStats,

	// Options
	RememberOptions,
	RecallOptions,
	ForgetFilter,
	UpdateOptions,

	// Knowledge types
	KnowledgeSource,
	KnowledgeChunk,
	KnowledgeStats,
	ChunkingOptions,
	IngestOptions,
	IngestResult,

	// Embedding types
	EmbeddingDimensions,
	EmbedOptions,
	EmbedResult,
	EmbedBatchResult,
	EmbedInfo,

	// Response types
	ApiResponse,
	RememberResponse,
	RecallResponse,
	ForgetResponse,
	StatsResponse,
	ExportResponse,
	ImportResponse,

	// Config
	MiniMemoryConfig,
	RequestOptions,

	// Agent Token types
	AgentPermission,
	AgentToken,
	CreateAgentTokenOptions,
	UpdateAgentTokenOptions,
	AgentTokenListOptions,
	AgentTokenListResponse,
	AgentTokenStats,
	AgentValidationResult,
} from './types.js';
