/**
 * MinimemoryClient - HTTP wrapper for minimemory API
 */

export interface RememberParams {
	content: string;
	type?: 'episodic' | 'semantic' | 'working';
	importance?: number;
	metadata?: Record<string, unknown>;
}

export interface RememberResult {
	success: boolean;
	id: string;
	type: string;
	importance: number;
}

export interface RecallParams {
	query: string;
	type?: 'episodic' | 'semantic' | 'working' | 'knowledge';
	limit?: number;
	threshold?: number;
	mode?: 'vector' | 'keyword' | 'hybrid';
}

export interface Memory {
	id: string;
	type: string;
	content: string;
	importance: number;
	metadata?: Record<string, unknown>;
	createdAt: number;
	updatedAt: number;
	score?: number;
}

export interface RecallResult {
	success: boolean;
	memories: Memory[];
	total: number;
}

export interface GetResult {
	success: boolean;
	memory: Memory | null;
}

export interface ForgetResult {
	success: boolean;
	deleted: boolean;
}

export interface StatsResult {
	success: boolean;
	namespace: string;
	total: number;
	byType: Record<string, number>;
}

export interface IngestParams {
	content: string;
	name: string;
	type?: 'document' | 'webpage' | 'code' | 'note';
	chunking?: {
		strategy?: 'fixed' | 'semantic' | 'paragraph';
		maxChunkSize?: number;
		overlap?: number;
	};
}

export interface IngestResult {
	success: boolean;
	sourceId: string;
	sourceName: string;
	chunksCreated: number;
	embeddingsGenerated: boolean;
	totalCharacters: number;
}

export class MinimemoryClient {
	private baseUrl: string;
	private apiKey: string;
	private namespace: string;

	constructor(baseUrl: string, apiKey: string, namespace: string = 'default') {
		// Remove trailing slash
		this.baseUrl = baseUrl.replace(/\/$/, '');
		this.apiKey = apiKey;
		this.namespace = namespace;
	}

	private async request<T>(
		method: string,
		path: string,
		body?: unknown
	): Promise<T> {
		const url = `${this.baseUrl}/api/v1${path}`;

		const res = await fetch(url, {
			method,
			headers: {
				'Content-Type': 'application/json',
				'X-API-Key': this.apiKey,
				'X-Namespace': this.namespace,
			},
			body: body ? JSON.stringify(body) : undefined,
		});

		if (!res.ok) {
			const error = await res.text();
			throw new Error(`API error ${res.status}: ${error}`);
		}

		return res.json() as Promise<T>;
	}

	/**
	 * Store a new memory
	 */
	async remember(params: RememberParams): Promise<RememberResult> {
		return this.request<RememberResult>('POST', '/remember', params);
	}

	/**
	 * Search for memories
	 */
	async recall(params: RecallParams): Promise<RecallResult> {
		return this.request<RecallResult>('POST', '/recall', params);
	}

	/**
	 * Get a memory by ID
	 */
	async get(id: string): Promise<GetResult> {
		return this.request<GetResult>('GET', `/memory/${id}`);
	}

	/**
	 * Delete a memory by ID
	 */
	async forget(id: string): Promise<ForgetResult> {
		return this.request<ForgetResult>('DELETE', `/forget/${id}`);
	}

	/**
	 * Get namespace statistics
	 */
	async stats(): Promise<StatsResult> {
		return this.request<StatsResult>('GET', '/stats');
	}

	/**
	 * Ingest a document into the knowledge bank
	 */
	async ingest(params: IngestParams): Promise<IngestResult> {
		return this.request<IngestResult>('POST', '/knowledge/ingest', params);
	}
}
