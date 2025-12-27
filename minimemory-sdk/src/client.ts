/**
 * minimemory SDK Client
 *
 * A type-safe client for interacting with the minimemory vector database service.
 */

import type {
	MiniMemoryConfig,
	RequestOptions,
	RememberOptions,
	RecallOptions,
	ForgetFilter,
	UpdateOptions,
	IngestOptions,
	ChunkingOptions,
	EmbedOptions,
	EmbeddingDimensions,
	Memory,
	RecallResult,
	MemoryStats,
	KnowledgeSource,
	KnowledgeChunk,
	KnowledgeStats,
	IngestResult,
	EmbedResult,
	EmbedBatchResult,
	EmbedInfo,
	RememberResponse,
	RecallResponse,
	ForgetResponse,
	StatsResponse,
	ExportResponse,
	ImportResponse,
	AgentToken,
	CreateAgentTokenOptions,
	UpdateAgentTokenOptions,
	AgentTokenListOptions,
	AgentTokenListResponse,
	AgentTokenStats,
	AgentValidationResult,
} from './types.js';

export class MiniMemoryError extends Error {
	constructor(
		message: string,
		public status?: number,
		public code?: string
	) {
		super(message);
		this.name = 'MiniMemoryError';
	}
}

export class MiniMemoryClient {
	private baseUrl: string;
	private namespace: string;
	private apiKey?: string;
	private accessToken?: string;
	private timeout: number;
	private defaultHeaders: Record<string, string>;

	constructor(config: MiniMemoryConfig) {
		this.baseUrl = config.baseUrl.replace(/\/$/, '');
		this.namespace = config.namespace || 'default';
		this.apiKey = config.apiKey;
		this.accessToken = config.accessToken;
		this.timeout = config.timeout || 30000;
		this.defaultHeaders = config.headers || {};
	}

	// ============ Configuration ============

	/**
	 * Set the active namespace for subsequent operations
	 */
	setNamespace(namespace: string): this {
		this.namespace = namespace;
		return this;
	}

	/**
	 * Set API key for authentication
	 */
	setApiKey(apiKey: string): this {
		this.apiKey = apiKey;
		return this;
	}

	/**
	 * Set JWT access token for authentication
	 */
	setAccessToken(token: string): this {
		this.accessToken = token;
		return this;
	}

	// ============ HTTP Helpers ============

	private async request<T>(
		method: string,
		path: string,
		body?: unknown,
		options?: RequestOptions
	): Promise<T> {
		const url = `${this.baseUrl}${path}`;
		const namespace = options?.namespace || this.namespace;

		const headers: Record<string, string> = {
			'Content-Type': 'application/json',
			'X-Namespace': namespace,
			...this.defaultHeaders,
			...options?.headers,
		};

		if (this.apiKey) {
			headers['X-API-Key'] = this.apiKey;
		}
		if (this.accessToken) {
			headers['Authorization'] = `Bearer ${this.accessToken}`;
		}

		const controller = new AbortController();
		const timeoutId = setTimeout(
			() => controller.abort(),
			options?.timeout || this.timeout
		);

		try {
			const response = await fetch(url, {
				method,
				headers,
				body: body ? JSON.stringify(body) : undefined,
				signal: controller.signal,
			});

			clearTimeout(timeoutId);

			const data = await response.json() as T & { error?: string };

			if (!response.ok) {
				throw new MiniMemoryError(
					data.error || `HTTP ${response.status}`,
					response.status
				);
			}

			return data;
		} catch (error) {
			clearTimeout(timeoutId);

			if (error instanceof MiniMemoryError) {
				throw error;
			}

			if (error instanceof Error) {
				if (error.name === 'AbortError') {
					throw new MiniMemoryError('Request timeout', 408, 'TIMEOUT');
				}
				throw new MiniMemoryError(error.message);
			}

			throw new MiniMemoryError('Unknown error');
		}
	}

	// ============ Memory Operations ============

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
	async remember(content: string, options?: RememberOptions): Promise<RememberResponse> {
		return this.request<RememberResponse>('POST', '/remember', {
			content,
			...options,
		});
	}

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
	async recall(
		queryOrOptions: string | RecallOptions,
		options?: Omit<RecallOptions, 'query'>
	): Promise<RecallResponse> {
		const body: RecallOptions =
			typeof queryOrOptions === 'string'
				? { query: queryOrOptions, ...options }
				: queryOrOptions;

		return this.request<RecallResponse>('POST', '/recall', body);
	}

	/**
	 * Delete a specific memory by ID
	 *
	 * @param id - Memory ID to delete
	 * @returns Success status and message
	 */
	async forget(id: string): Promise<ForgetResponse> {
		return this.request<ForgetResponse>('DELETE', `/forget/${id}`);
	}

	/**
	 * Delete memories matching a filter
	 *
	 * @param filter - Filter criteria (e.g., { type: 'working' })
	 * @returns Count of deleted memories
	 */
	async forgetByFilter(filter: ForgetFilter): Promise<ForgetResponse> {
		return this.request<ForgetResponse>('POST', '/forget', { filter });
	}

	/**
	 * Get a specific memory by ID
	 *
	 * @param id - Memory ID to retrieve
	 * @returns The memory object or null if not found
	 */
	async get(id: string): Promise<{ success: boolean; memory: Memory; source: string }> {
		return this.request('GET', `/memory/${id}`);
	}

	/**
	 * Update a memory's content or metadata
	 *
	 * @param id - Memory ID to update
	 * @param updates - Fields to update
	 * @returns The updated memory
	 */
	async update(id: string, updates: UpdateOptions): Promise<{ success: boolean; memory: Memory }> {
		return this.request('PATCH', `/memory/${id}`, updates);
	}

	/**
	 * Get memory statistics for the namespace
	 *
	 * @returns Statistics including counts by type, importance averages, etc.
	 */
	async stats(): Promise<StatsResponse> {
		return this.request<StatsResponse>('GET', '/stats');
	}

	/**
	 * Clean up expired working memories
	 *
	 * @returns Count of memories removed
	 */
	async cleanup(): Promise<{ success: boolean; count: number; message: string }> {
		return this.request('POST', '/cleanup');
	}

	/**
	 * Apply importance decay to all memories
	 *
	 * @returns Success status
	 */
	async decay(): Promise<{ success: boolean; message: string }> {
		return this.request('POST', '/decay');
	}

	/**
	 * Export all memories from the namespace
	 *
	 * @returns All memories in exportable format
	 */
	async export(): Promise<ExportResponse> {
		return this.request<ExportResponse>('POST', '/export');
	}

	/**
	 * Import memories into the namespace
	 *
	 * @param memories - Array of memories to import
	 * @returns Count of imported memories
	 */
	async import(memories: Memory[]): Promise<ImportResponse> {
		return this.request<ImportResponse>('POST', '/import', { memories });
	}

	/**
	 * Clear all memories in the namespace
	 *
	 * @returns Success status
	 */
	async clear(): Promise<{ success: boolean; message: string }> {
		return this.request('DELETE', '/clear');
	}

	// ============ Knowledge Bank Operations ============

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
	get knowledge() {
		return {
			ingest: async (options: IngestOptions): Promise<IngestResult & { success: boolean }> => {
				return this.request('POST', '/knowledge/ingest', options);
			},

			/**
			 * List all knowledge sources
			 */
			listSources: async (options?: {
				type?: string;
				limit?: number;
				offset?: number;
			}): Promise<{
				success: boolean;
				sources: KnowledgeSource[];
				total: number;
				hasMore: boolean;
			}> => {
				const params = new URLSearchParams();
				if (options?.type) params.set('type', options.type);
				if (options?.limit) params.set('limit', String(options.limit));
				if (options?.offset) params.set('offset', String(options.offset));

				const query = params.toString();
				return this.request('GET', `/knowledge/sources${query ? `?${query}` : ''}`);
			},

			/**
			 * Get a specific knowledge source
			 */
			getSource: async (
				id: string
			): Promise<{ success: boolean; source: KnowledgeSource }> => {
				return this.request('GET', `/knowledge/sources/${id}`);
			},

			/**
			 * Delete a knowledge source and its chunks
			 */
			deleteSource: async (
				id: string
			): Promise<{ success: boolean; message: string }> => {
				return this.request('DELETE', `/knowledge/sources/${id}`);
			},

			/**
			 * Get chunks for a specific source
			 */
			getChunks: async (
				sourceId: string,
				options?: { limit?: number; offset?: number }
			): Promise<{
				success: boolean;
				source: { id: string; name: string; type: string };
				chunks: KnowledgeChunk[];
				total: number;
				hasMore: boolean;
			}> => {
				const params = new URLSearchParams();
				if (options?.limit) params.set('limit', String(options.limit));
				if (options?.offset) params.set('offset', String(options.offset));

				const query = params.toString();
				return this.request(
					'GET',
					`/knowledge/sources/${sourceId}/chunks${query ? `?${query}` : ''}`
				);
			},

			/**
			 * Get knowledge bank statistics
			 */
			stats: async (): Promise<{
				success: boolean;
				namespace: string;
				stats: KnowledgeStats;
			}> => {
				return this.request('GET', '/knowledge/stats');
			},

			/**
			 * Preview how content will be chunked
			 */
			previewChunking: async (
				content: string,
				options?: ChunkingOptions
			): Promise<{
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
			}> => {
				return this.request('POST', '/knowledge/chunk-preview', {
					content,
					chunking: options,
				});
			},
		};
	}

	// ============ Embedding Operations ============

	/**
	 * Generate embeddings using Workers AI
	 */
	get embed() {
		return {
			/**
			 * Generate embedding for a single text
			 */
			single: async (
				text: string,
				options?: EmbedOptions
			): Promise<EmbedResult & { success: boolean }> => {
				return this.request('POST', '/embed', { text, ...options });
			},

			/**
			 * Generate embeddings for multiple texts
			 */
			batch: async (
				texts: string[],
				options?: EmbedOptions
			): Promise<EmbedBatchResult & { success: boolean }> => {
				return this.request('POST', '/embed', { texts, ...options });
			},

			/**
			 * Get embedding service info
			 */
			info: async (): Promise<EmbedInfo> => {
				return this.request('GET', '/embed/info');
			},
		};
	}

	// ============ Agent Token Operations ============

	/**
	 * Agent token management for MCP access control.
	 * Requires JWT authentication (use setAccessToken).
	 */
	get agentTokens() {
		return {
			/**
			 * List all agent tokens for the authenticated user
			 *
			 * @param options - Optional filter and pagination
			 * @returns List of agent tokens with pagination info
			 */
			list: async (options?: AgentTokenListOptions): Promise<AgentTokenListResponse> => {
				const params = new URLSearchParams();
				if (options?.active !== undefined) params.set('active', String(options.active));
				if (options?.limit) params.set('limit', String(options.limit));
				if (options?.offset) params.set('offset', String(options.offset));

				const query = params.toString();
				return this.request('GET', `/agent-tokens${query ? `?${query}` : ''}`);
			},

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
			create: async (
				options: CreateAgentTokenOptions
			): Promise<{ token: AgentToken; message: string }> => {
				return this.request('POST', '/agent-tokens', options);
			},

			/**
			 * Get a specific agent token by ID
			 *
			 * @param id - Agent token ID
			 * @returns The agent token
			 */
			get: async (id: string): Promise<{ token: AgentToken }> => {
				return this.request('GET', `/agent-tokens/${id}`);
			},

			/**
			 * Update an agent token
			 *
			 * @param id - Agent token ID
			 * @param updates - Fields to update
			 * @returns The updated token
			 */
			update: async (
				id: string,
				updates: UpdateAgentTokenOptions
			): Promise<{ token: AgentToken }> => {
				return this.request('PATCH', `/agent-tokens/${id}`, updates);
			},

			/**
			 * Delete an agent token
			 *
			 * @param id - Agent token ID
			 * @returns Success status
			 */
			delete: async (id: string): Promise<{ success: boolean }> => {
				return this.request('DELETE', `/agent-tokens/${id}`);
			},

			/**
			 * Toggle agent token active status
			 *
			 * @param id - Agent token ID
			 * @returns The toggled token with new status
			 */
			toggle: async (id: string): Promise<{ token: AgentToken; message: string }> => {
				return this.request('POST', `/agent-tokens/${id}/toggle`);
			},

			/**
			 * Add a memory ID to the token's allowed list
			 *
			 * @param id - Agent token ID
			 * @param memoryId - Memory ID to add
			 * @returns The updated token
			 */
			addMemory: async (id: string, memoryId: string): Promise<{ token: AgentToken }> => {
				return this.request('POST', `/agent-tokens/${id}/add-memory`, { memoryId });
			},

			/**
			 * Remove a memory ID from the token's allowed list
			 *
			 * @param id - Agent token ID
			 * @param memoryId - Memory ID to remove
			 * @returns The updated token
			 */
			removeMemory: async (id: string, memoryId: string): Promise<{ token: AgentToken }> => {
				return this.request('POST', `/agent-tokens/${id}/remove-memory`, { memoryId });
			},

			/**
			 * Get usage statistics for the user's tokens
			 *
			 * @returns Aggregated statistics
			 */
			stats: async (): Promise<{ stats: AgentTokenStats }> => {
				return this.request('GET', '/agent-tokens/stats');
			},

			/**
			 * Validate an API key + agent token combination.
			 * Used by MCP servers to verify agent credentials.
			 *
			 * @param apiKey - The API key
			 * @param agentToken - The agent token ID
			 * @returns Validation result with permissions
			 */
			validate: async (apiKey: string, agentToken: string): Promise<AgentValidationResult> => {
				return this.request('POST', '/auth/validate-agent', { apiKey, agentToken });
			},
		};
	}
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
export function createClient(config: MiniMemoryConfig): MiniMemoryClient {
	return new MiniMemoryClient(config);
}
