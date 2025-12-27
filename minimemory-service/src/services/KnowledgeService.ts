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
import type {
	KnowledgeSource,
	KnowledgeSourceType,
	ChunkingOptions,
	IngestRequest,
	IngestResult,
} from '../memory/types.js';

// Default chunking configuration
const DEFAULT_CHUNK_SIZE = 1000;
const DEFAULT_CHUNK_OVERLAP = 200;
const DEFAULT_SEPARATORS = ['\n\n', '\n', '. ', '! ', '? ', '; ', ', ', ' '];

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
export class KnowledgeService {
	private config: Required<KnowledgeServiceConfig>;

	constructor(
		private db: D1Database | null,
		config: KnowledgeServiceConfig = {}
	) {
		this.config = {
			enabled: config.enabled ?? true,
			defaultChunkSize: config.defaultChunkSize ?? DEFAULT_CHUNK_SIZE,
			defaultChunkOverlap: config.defaultChunkOverlap ?? DEFAULT_CHUNK_OVERLAP,
			maxChunksPerDocument: config.maxChunksPerDocument ?? 1000,
		};
	}

	/**
	 * Check if the service is available
	 */
	isAvailable(): boolean {
		return this.config.enabled && this.db !== null;
	}

	/**
	 * Chunk text into overlapping segments
	 */
	chunkText(text: string, options: ChunkingOptions = {}): TextChunk[] {
		const chunkSize = options.chunkSize ?? this.config.defaultChunkSize;
		const overlap = options.chunkOverlap ?? this.config.defaultChunkOverlap;
		const separators = options.separators ?? DEFAULT_SEPARATORS;
		const preserveParagraphs = options.preserveParagraphs ?? true;

		const chunks: TextChunk[] = [];
		let currentPosition = 0;
		let chunkIndex = 0;

		while (currentPosition < text.length) {
			// Calculate end position for this chunk
			let endPosition = Math.min(currentPosition + chunkSize, text.length);

			// If not at the end, try to find a natural break point
			if (endPosition < text.length) {
				const searchStart = Math.max(currentPosition + chunkSize - 100, currentPosition);
				let bestBreak = -1;

				// Try each separator in order of preference
				for (const separator of separators) {
					const idx = text.lastIndexOf(separator, endPosition);
					if (idx > searchStart) {
						bestBreak = idx + separator.length;
						if (preserveParagraphs && separator === '\n\n') {
							break; // Prefer paragraph breaks
						}
					}
				}

				if (bestBreak > currentPosition) {
					endPosition = bestBreak;
				}
			}

			// Extract chunk text
			const chunkText = text.slice(currentPosition, endPosition).trim();

			if (chunkText.length > 0) {
				chunks.push({
					text: chunkText,
					index: chunkIndex,
					startOffset: currentPosition,
					endOffset: endPosition,
				});
				chunkIndex++;
			}

			// Move position, accounting for overlap
			currentPosition = endPosition - overlap;

			// Prevent infinite loops
			if (currentPosition <= chunks[chunks.length - 1]?.startOffset) {
				currentPosition = endPosition;
			}

			// Safety check for max chunks
			if (chunks.length >= this.config.maxChunksPerDocument) {
				break;
			}
		}

		return chunks;
	}

	/**
	 * Create a new knowledge source
	 */
	async createSource(
		namespace: string,
		source: Omit<KnowledgeSource, 'id' | 'createdAt' | 'updatedAt'>
	): Promise<KnowledgeSource> {
		if (!this.db) {
			throw new Error('D1 database not configured');
		}

		const now = Date.now();
		const id = `src_${generateId()}`;

		const knowledgeSource: KnowledgeSource = {
			id,
			...source,
			createdAt: now,
			updatedAt: now,
		};

		await this.db
			.prepare(`
				INSERT INTO knowledge_sources
				(id, namespace, name, type, url, mime_type, size, chunk_count, metadata, created_at, updated_at)
				VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
			`)
			.bind(
				knowledgeSource.id,
				namespace,
				knowledgeSource.name,
				knowledgeSource.type,
				knowledgeSource.url || null,
				knowledgeSource.mimeType || null,
				knowledgeSource.size || null,
				knowledgeSource.chunkCount,
				JSON.stringify(knowledgeSource.metadata),
				knowledgeSource.createdAt,
				knowledgeSource.updatedAt
			)
			.run();

		return knowledgeSource;
	}

	/**
	 * Get a knowledge source by ID
	 */
	async getSource(id: string): Promise<KnowledgeSource | null> {
		if (!this.db) return null;

		const result = await this.db
			.prepare('SELECT * FROM knowledge_sources WHERE id = ?')
			.bind(id)
			.first<SourceRow>();

		if (!result) return null;

		return this.rowToSource(result);
	}

	/**
	 * List all knowledge sources in a namespace
	 */
	async listSources(namespace: string, options?: {
		type?: KnowledgeSourceType;
		limit?: number;
		offset?: number;
	}): Promise<{ sources: KnowledgeSource[]; total: number }> {
		if (!this.db) {
			return { sources: [], total: 0 };
		}

		const limit = options?.limit ?? 100;
		const offset = options?.offset ?? 0;

		let query = 'SELECT * FROM knowledge_sources WHERE namespace = ?';
		const params: unknown[] = [namespace];

		if (options?.type) {
			query += ' AND type = ?';
			params.push(options.type);
		}

		query += ' ORDER BY created_at DESC LIMIT ? OFFSET ?';
		params.push(limit, offset);

		const [results, countResult] = await Promise.all([
			this.db.prepare(query).bind(...params).all<SourceRow>(),
			this.db
				.prepare('SELECT COUNT(*) as total FROM knowledge_sources WHERE namespace = ?')
				.bind(namespace)
				.first<{ total: number }>(),
		]);

		return {
			sources: (results.results || []).map(r => this.rowToSource(r)),
			total: countResult?.total ?? 0,
		};
	}

	/**
	 * Delete a knowledge source and all its chunks
	 */
	async deleteSource(id: string): Promise<boolean> {
		if (!this.db) return false;

		// Delete chunks first (will cascade, but explicit for memory cleanup)
		await this.db
			.prepare('DELETE FROM memories WHERE metadata LIKE ?')
			.bind(`%"sourceId":"${id}"%`)
			.run();

		const result = await this.db
			.prepare('DELETE FROM knowledge_sources WHERE id = ?')
			.bind(id)
			.run();

		return (result.meta?.changes ?? 0) > 0;
	}

	/**
	 * Update chunk count for a source
	 */
	async updateSourceChunkCount(id: string, chunkCount: number): Promise<void> {
		if (!this.db) return;

		await this.db
			.prepare('UPDATE knowledge_sources SET chunk_count = ?, updated_at = ? WHERE id = ?')
			.bind(chunkCount, Date.now(), id)
			.run();
	}

	/**
	 * Get knowledge stats for a namespace
	 */
	async getStats(namespace: string): Promise<{
		totalSources: number;
		totalChunks: number;
		byType: Record<KnowledgeSourceType, number>;
		totalSize: number;
	}> {
		if (!this.db) {
			return {
				totalSources: 0,
				totalChunks: 0,
				byType: { document: 0, url: 0, api: 0, manual: 0 },
				totalSize: 0,
			};
		}

		const stats = await this.db
			.prepare(`
				SELECT
					COUNT(*) as total_sources,
					SUM(chunk_count) as total_chunks,
					SUM(CASE WHEN type = 'document' THEN 1 ELSE 0 END) as documents,
					SUM(CASE WHEN type = 'url' THEN 1 ELSE 0 END) as urls,
					SUM(CASE WHEN type = 'api' THEN 1 ELSE 0 END) as apis,
					SUM(CASE WHEN type = 'manual' THEN 1 ELSE 0 END) as manuals,
					SUM(COALESCE(size, 0)) as total_size
				FROM knowledge_sources WHERE namespace = ?
			`)
			.bind(namespace)
			.first<{
				total_sources: number;
				total_chunks: number;
				documents: number;
				urls: number;
				apis: number;
				manuals: number;
				total_size: number;
			}>();

		return {
			totalSources: stats?.total_sources ?? 0,
			totalChunks: stats?.total_chunks ?? 0,
			byType: {
				document: stats?.documents ?? 0,
				url: stats?.urls ?? 0,
				api: stats?.apis ?? 0,
				manual: stats?.manuals ?? 0,
			},
			totalSize: stats?.total_size ?? 0,
		};
	}

	// ============ Helper Methods ============

	private rowToSource(row: SourceRow): KnowledgeSource {
		return {
			id: row.id,
			name: row.name,
			type: row.type as KnowledgeSourceType,
			url: row.url || undefined,
			mimeType: row.mime_type || undefined,
			size: row.size || undefined,
			chunkCount: row.chunk_count,
			namespace: row.namespace,
			metadata: row.metadata ? JSON.parse(row.metadata) : {},
			createdAt: row.created_at,
			updatedAt: row.updated_at,
		};
	}
}

/**
 * Generate a short unique ID
 */
function generateId(): string {
	const timestamp = Date.now().toString(36);
	const random = Math.random().toString(36).substring(2, 8);
	return `${timestamp}_${random}`;
}

// Row type for D1 queries
interface SourceRow {
	id: string;
	namespace: string;
	name: string;
	type: string;
	url: string | null;
	mime_type: string | null;
	size: number | null;
	chunk_count: number;
	metadata: string | null;
	created_at: number;
	updated_at: number;
}

// Export for convenience
export { KnowledgeSourceType, ChunkingOptions, IngestRequest, IngestResult };
