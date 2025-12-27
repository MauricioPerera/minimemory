/**
 * VectorDB - In-memory vector database with hybrid search support
 * Supports vector quantization for memory efficiency (int8, binary)
 */

import { BM25Index, BM25SearchResult, SerializedBM25Index } from './BM25Index.js';
import {
	SearchMode,
	FusionMethod,
	HybridSearchResult,
	VectorSearchResult,
	hybridFusion,
} from './HybridSearch.js';
import {
	QuantizationType,
	quantizeScalar,
	dequantizeScalar,
	quantizeBinary,
	cosineSimilarityInt8,
	hammingDistance,
	hammingToSimilarity,
	int8ToBase64,
	base64ToInt8,
	uint8ToBase64,
	base64ToUint8,
	getQuantizedSize,
	calculateSavings,
} from './Quantization.js';

export type DistanceMetric = 'cosine' | 'euclidean' | 'dot';
export type IndexType = 'flat' | 'hnsw';

// Re-export types
export type { SearchMode, FusionMethod, HybridSearchResult, BM25SearchResult, QuantizationType };

export interface VectorDBOptions {
	dimensions: number;
	distance?: DistanceMetric;
	indexType?: IndexType;
	quantization?: QuantizationType;
	// For binary quantization: oversample factor for rescoring
	rescoreOversample?: number;
}

export interface SearchResult {
	id: string;
	distance: number;
	similarity: number;
	metadata?: Record<string, unknown>;
}

// ============================================================================
// Metadata Filtering Types
// ============================================================================

export type FilterOperator =
	| '$eq' | '$ne' | '$gt' | '$gte' | '$lt' | '$lte'
	| '$in' | '$nin' | '$exists' | '$contains' | '$startsWith' | '$endsWith';

export type FilterCondition =
	| { $eq: unknown }
	| { $ne: unknown }
	| { $gt: number | string | Date }
	| { $gte: number | string | Date }
	| { $lt: number | string | Date }
	| { $lte: number | string | Date }
	| { $in: unknown[] }
	| { $nin: unknown[] }
	| { $exists: boolean }
	| { $contains: string }
	| { $startsWith: string }
	| { $endsWith: string };

export type MetadataFilterValue = unknown | FilterCondition;

export interface MetadataFilter {
	[field: string]: MetadataFilterValue | MetadataFilter[] | undefined;
	$and?: MetadataFilter[];
	$or?: MetadataFilter[];
}

export interface SearchOptions {
	k: number;
	filter?: MetadataFilter;
	minSimilarity?: number;
	includeVectors?: boolean;
}

export interface HybridSearchOptions {
	mode: SearchMode;
	k: number;
	queryVector?: number[];
	filter?: MetadataFilter;
	minSimilarity?: number;
	keywords?: string;
	textFields?: string[];
	bm25K1?: number;
	bm25B?: number;
	alpha?: number;
	fusionMethod?: FusionMethod;
	rrfConstant?: number;
}

interface StoredVector {
	id: string;
	vector: number[];
	metadata: Record<string, unknown> | null;
	norm?: number;
	createdAt: number;
	updatedAt: number;
	// Quantized representations (optional, for memory efficiency)
	quantizedInt8?: Int8Array;
	quantizedBinary?: Uint8Array;
}

interface SerializedVector {
	id: string;
	vector: number[];
	metadata: Record<string, unknown> | null;
	norm?: number;
	createdAt: number;
	updatedAt: number;
	// Serialized as base64
	quantizedInt8?: string;
	quantizedBinary?: string;
}

export interface SerializedDB {
	version: string;
	dimensions: number;
	distance: DistanceMetric;
	indexType: IndexType;
	quantization?: QuantizationType;
	vectors: SerializedVector[];
	bm25Index?: SerializedBM25Index;
}

// ============================================================================
// Distance Functions
// ============================================================================

function cosineDistance(a: number[], b: number[], normA?: number, normB?: number): number {
	let dot = 0;
	let nA = normA ?? 0;
	let nB = normB ?? 0;

	const needNormA = normA === undefined;
	const needNormB = normB === undefined;

	for (let i = 0; i < a.length; i++) {
		dot += a[i] * b[i];
		if (needNormA) nA += a[i] * a[i];
		if (needNormB) nB += b[i] * b[i];
	}

	if (needNormA) nA = Math.sqrt(nA);
	if (needNormB) nB = Math.sqrt(nB);

	const denom = nA * nB;
	if (denom === 0) return 1;

	const similarity = Math.max(-1, Math.min(1, dot / denom));
	return 1 - similarity;
}

function euclideanDistance(a: number[], b: number[]): number {
	let sum = 0;
	for (let i = 0; i < a.length; i++) {
		const diff = a[i] - b[i];
		sum += diff * diff;
	}
	return Math.sqrt(sum);
}

function dotProductDistance(a: number[], b: number[]): number {
	let dot = 0;
	for (let i = 0; i < a.length; i++) {
		dot += a[i] * b[i];
	}
	return -dot;
}

// ============================================================================
// Metadata Filter Evaluation
// ============================================================================

function getNestedValue(obj: Record<string, unknown> | null, path: string): unknown {
	if (!obj) return undefined;

	const parts = path.split('.');
	let current: unknown = obj;

	for (const part of parts) {
		if (current === null || current === undefined) return undefined;
		if (typeof current !== 'object') return undefined;
		current = (current as Record<string, unknown>)[part];
	}

	return current;
}

function compareValues(a: unknown, b: unknown): number {
	if (a instanceof Date && b instanceof Date) {
		return a.getTime() - b.getTime();
	}

	if (typeof a === 'string' && typeof b === 'string') {
		const dateA = Date.parse(a);
		const dateB = Date.parse(b);
		if (!isNaN(dateA) && !isNaN(dateB)) {
			return dateA - dateB;
		}
		return a.localeCompare(b);
	}

	if (typeof a === 'number' && typeof b === 'number') {
		return a - b;
	}

	return String(a).localeCompare(String(b));
}

function isFilterCondition(value: unknown): value is FilterCondition {
	if (!value || typeof value !== 'object') return false;
	const keys = Object.keys(value);
	return keys.length > 0 && keys.every(k => k.startsWith('$'));
}

function evaluateCondition(fieldValue: unknown, condition: FilterCondition): boolean {
	const entries = Object.entries(condition);

	for (const [operator, operand] of entries) {
		switch (operator) {
			case '$eq':
				if (fieldValue !== operand) return false;
				break;
			case '$ne':
				if (fieldValue === operand) return false;
				break;
			case '$gt':
				if (fieldValue === undefined || compareValues(fieldValue, operand) <= 0) return false;
				break;
			case '$gte':
				if (fieldValue === undefined || compareValues(fieldValue, operand) < 0) return false;
				break;
			case '$lt':
				if (fieldValue === undefined || compareValues(fieldValue, operand) >= 0) return false;
				break;
			case '$lte':
				if (fieldValue === undefined || compareValues(fieldValue, operand) > 0) return false;
				break;
			case '$in':
				if (!Array.isArray(operand) || !operand.includes(fieldValue)) return false;
				break;
			case '$nin':
				if (!Array.isArray(operand) || operand.includes(fieldValue)) return false;
				break;
			case '$exists':
				if (operand === true && fieldValue === undefined) return false;
				if (operand === false && fieldValue !== undefined) return false;
				break;
			case '$contains':
				if (typeof fieldValue !== 'string' || typeof operand !== 'string') return false;
				if (!fieldValue.toLowerCase().includes(operand.toLowerCase())) return false;
				break;
			case '$startsWith':
				if (typeof fieldValue !== 'string' || typeof operand !== 'string') return false;
				if (!fieldValue.toLowerCase().startsWith(operand.toLowerCase())) return false;
				break;
			case '$endsWith':
				if (typeof fieldValue !== 'string' || typeof operand !== 'string') return false;
				if (!fieldValue.toLowerCase().endsWith(operand.toLowerCase())) return false;
				break;
		}
	}

	return true;
}

function evaluateFilter(
	metadata: Record<string, unknown> | null,
	filter: MetadataFilter
): boolean {
	if (filter.$and) {
		for (const subFilter of filter.$and) {
			if (!evaluateFilter(metadata, subFilter)) {
				return false;
			}
		}
	}

	if (filter.$or) {
		let anyMatch = false;
		for (const subFilter of filter.$or) {
			if (evaluateFilter(metadata, subFilter)) {
				anyMatch = true;
				break;
			}
		}
		if (!anyMatch && filter.$or.length > 0) {
			return false;
		}
	}

	for (const [field, condition] of Object.entries(filter)) {
		if (field === '$and' || field === '$or') continue;

		const fieldValue = getNestedValue(metadata, field);

		if (isFilterCondition(condition)) {
			if (!evaluateCondition(fieldValue, condition)) {
				return false;
			}
		} else {
			if (fieldValue !== condition) {
				return false;
			}
		}
	}

	return true;
}

// ============================================================================
// VectorDB Class
// ============================================================================

export class VectorDB {
	private vectors: Map<string, StoredVector> = new Map();
	private readonly _dimensions: number;
	private readonly _distance: DistanceMetric;
	private readonly _indexType: IndexType;
	private readonly _quantization: QuantizationType;
	private readonly _rescoreOversample: number;
	private bm25Index: BM25Index | null = null;
	private bm25TextFields: string[] = [];

	constructor(options: VectorDBOptions) {
		this._dimensions = options.dimensions;
		this._distance = options.distance || 'cosine';
		this._indexType = options.indexType || 'flat';
		this._quantization = options.quantization || 'none';
		this._rescoreOversample = options.rescoreOversample || 4;
	}

	get dimensions(): number { return this._dimensions; }
	get distance(): DistanceMetric { return this._distance; }
	get indexType(): IndexType { return this._indexType; }
	get quantization(): QuantizationType { return this._quantization; }
	get length(): number { return this.vectors.size; }

	private computeNorm(vector: number[]): number {
		let sum = 0;
		for (let i = 0; i < vector.length; i++) {
			sum += vector[i] * vector[i];
		}
		return Math.sqrt(sum);
	}

	private calculateDistance(a: number[], b: number[], normA?: number, normB?: number): number {
		switch (this._distance) {
			case 'cosine':
				return cosineDistance(a, b, normA, normB);
			case 'euclidean':
				return euclideanDistance(a, b);
			case 'dot':
				return dotProductDistance(a, b);
			default:
				return cosineDistance(a, b, normA, normB);
		}
	}

	/**
	 * Create quantized representations for a vector
	 */
	private createQuantizedRepresentations(vector: number[]): {
		quantizedInt8?: Int8Array;
		quantizedBinary?: Uint8Array;
	} {
		const result: { quantizedInt8?: Int8Array; quantizedBinary?: Uint8Array } = {};

		if (this._quantization === 'int8' || this._quantization === 'binary') {
			// Always create int8 for potential rescoring
			result.quantizedInt8 = quantizeScalar(vector);
		}

		if (this._quantization === 'binary') {
			result.quantizedBinary = quantizeBinary(vector);
		}

		return result;
	}

	insert(id: string, vector: number[], metadata?: Record<string, unknown>): void {
		if (vector.length !== this._dimensions) {
			throw new Error(`Dimension mismatch: expected ${this._dimensions}, got ${vector.length}`);
		}

		if (this.vectors.has(id)) {
			throw new Error(`Vector with id "${id}" already exists. Use upsert() instead.`);
		}

		const now = Date.now();
		const norm = this._distance === 'cosine' ? this.computeNorm(vector) : undefined;
		const quantized = this.createQuantizedRepresentations(vector);

		this.vectors.set(id, {
			id,
			vector: [...vector],
			metadata: metadata || null,
			norm,
			createdAt: now,
			updatedAt: now,
			...quantized,
		});

		if (this.bm25Index) {
			this.bm25Index.addDocument(id, metadata || null);
		}
	}

	upsert(id: string, vector: number[], metadata?: Record<string, unknown>): void {
		if (vector.length !== this._dimensions) {
			throw new Error(`Dimension mismatch: expected ${this._dimensions}, got ${vector.length}`);
		}

		const existing = this.vectors.get(id);
		const now = Date.now();
		const norm = this._distance === 'cosine' ? this.computeNorm(vector) : undefined;
		const quantized = this.createQuantizedRepresentations(vector);

		this.vectors.set(id, {
			id,
			vector: [...vector],
			metadata: metadata || null,
			norm,
			createdAt: existing?.createdAt ?? now,
			updatedAt: now,
			...quantized,
		});

		if (this.bm25Index) {
			this.bm25Index.updateDocument(id, metadata || null);
		}
	}

	/**
	 * Search using quantized vectors (fast approximate search)
	 */
	private searchQuantized(
		query: number[],
		k: number,
		filter?: MetadataFilter
	): { id: string; similarity: number; metadata?: Record<string, unknown> }[] {
		if (this._quantization === 'binary') {
			// Binary quantization with Hamming distance
			const queryBinary = quantizeBinary(query);
			const results: { id: string; similarity: number; metadata?: Record<string, unknown> }[] = [];

			for (const stored of this.vectors.values()) {
				if (filter && !evaluateFilter(stored.metadata, filter)) {
					continue;
				}

				if (stored.quantizedBinary) {
					const distance = hammingDistance(queryBinary, stored.quantizedBinary);
					const similarity = hammingToSimilarity(distance, this._dimensions);
					results.push({
						id: stored.id,
						similarity,
						metadata: stored.metadata || undefined,
					});
				}
			}

			results.sort((a, b) => b.similarity - a.similarity);
			return results.slice(0, k);
		} else if (this._quantization === 'int8') {
			// Scalar quantization with int8 cosine similarity
			const queryInt8 = quantizeScalar(query);
			const results: { id: string; similarity: number; metadata?: Record<string, unknown> }[] = [];

			for (const stored of this.vectors.values()) {
				if (filter && !evaluateFilter(stored.metadata, filter)) {
					continue;
				}

				if (stored.quantizedInt8) {
					const similarity = cosineSimilarityInt8(queryInt8, stored.quantizedInt8);
					results.push({
						id: stored.id,
						similarity,
						metadata: stored.metadata || undefined,
					});
				}
			}

			results.sort((a, b) => b.similarity - a.similarity);
			return results.slice(0, k);
		}

		return [];
	}

	/**
	 * Rescore candidates using full-precision vectors
	 */
	private rescoreCandidates(
		query: number[],
		candidates: { id: string; similarity: number; metadata?: Record<string, unknown> }[]
	): SearchResult[] {
		const queryNorm = this._distance === 'cosine' ? this.computeNorm(query) : undefined;

		return candidates.map(candidate => {
			const stored = this.vectors.get(candidate.id);
			if (!stored) {
				return {
					id: candidate.id,
					distance: 1,
					similarity: candidate.similarity,
					metadata: candidate.metadata,
				};
			}

			const distance = this.calculateDistance(query, stored.vector, queryNorm, stored.norm);
			let similarity: number;
			if (this._distance === 'cosine') {
				similarity = 1 - distance;
			} else if (this._distance === 'dot') {
				similarity = -distance;
			} else {
				similarity = 1 / (1 + distance);
			}

			return {
				id: stored.id,
				distance,
				similarity,
				metadata: stored.metadata || undefined,
			};
		}).sort((a, b) => a.distance - b.distance);
	}

	search(query: number[], k: number, options?: Partial<SearchOptions>): SearchResult[] {
		if (query.length !== this._dimensions) {
			throw new Error(`Query dimension mismatch: expected ${this._dimensions}, got ${query.length}`);
		}

		if (this.vectors.size === 0) {
			return [];
		}

		const filter = options?.filter;
		const minSimilarity = options?.minSimilarity ?? 0;

		// Use quantized search if available
		if (this._quantization !== 'none') {
			// For binary: fetch more candidates for rescoring
			const fetchK = this._quantization === 'binary'
				? k * this._rescoreOversample
				: k;

			const candidates = this.searchQuantized(query, fetchK, filter);

			// Rescore with full precision for binary quantization
			let results: SearchResult[];
			if (this._quantization === 'binary') {
				results = this.rescoreCandidates(query, candidates).slice(0, k);
			} else {
				// int8 is accurate enough without rescoring
				results = candidates.map(c => ({
					id: c.id,
					distance: 1 - c.similarity,
					similarity: c.similarity,
					metadata: c.metadata,
				}));
			}

			// Apply minSimilarity filter
			return results.filter(r => r.similarity >= minSimilarity);
		}

		// Standard float32 search
		const queryNorm = this._distance === 'cosine' ? this.computeNorm(query) : undefined;

		const results: SearchResult[] = [];

		for (const stored of this.vectors.values()) {
			if (filter && !evaluateFilter(stored.metadata, filter)) {
				continue;
			}

			const distance = this.calculateDistance(query, stored.vector, queryNorm, stored.norm);

			let similarity: number;
			if (this._distance === 'cosine') {
				similarity = 1 - distance;
			} else if (this._distance === 'dot') {
				similarity = -distance;
			} else {
				similarity = 1 / (1 + distance);
			}

			if (similarity < minSimilarity) {
				continue;
			}

			results.push({
				id: stored.id,
				distance,
				similarity,
				metadata: stored.metadata || undefined,
			});
		}

		results.sort((a, b) => a.distance - b.distance);
		return results.slice(0, Math.min(k, results.length));
	}

	get(id: string): { id: string; vector: number[]; metadata: Record<string, unknown> | null; createdAt: number; updatedAt: number } | null {
		const stored = this.vectors.get(id);
		if (!stored) return null;
		return {
			id: stored.id,
			vector: [...stored.vector],
			metadata: stored.metadata,
			createdAt: stored.createdAt,
			updatedAt: stored.updatedAt,
		};
	}

	delete(id: string): boolean {
		const deleted = this.vectors.delete(id);
		if (deleted && this.bm25Index) {
			this.bm25Index.removeDocument(id);
		}
		return deleted;
	}

	contains(id: string): boolean {
		return this.vectors.has(id);
	}

	clear(): void {
		this.vectors.clear();
		if (this.bm25Index) {
			this.bm25Index.clear();
		}
	}

	getIds(): string[] {
		return Array.from(this.vectors.keys());
	}

	// ============================================================================
	// BM25 Keyword Search
	// ============================================================================

	configureBM25(options: { textFields: string[]; k1?: number; b?: number }): void {
		if (this.bm25Index && JSON.stringify(this.bm25TextFields) === JSON.stringify(options.textFields)) {
			return;
		}

		this.bm25Index = new BM25Index({
			textFields: options.textFields,
			k1: options.k1,
			b: options.b,
		});
		this.bm25TextFields = [...options.textFields];

		for (const stored of this.vectors.values()) {
			this.bm25Index.addDocument(stored.id, stored.metadata);
		}
	}

	private ensureBM25Index(textFields: string[], k1?: number, b?: number): void {
		if (!this.bm25Index || JSON.stringify(this.bm25TextFields) !== JSON.stringify(textFields)) {
			this.configureBM25({ textFields, k1, b });
		}
	}

	keywordSearch(query: string, k: number, options?: { textFields?: string[]; filter?: MetadataFilter; k1?: number; b?: number }): BM25SearchResult[] {
		const textFields = options?.textFields || this.bm25TextFields;

		if (textFields.length === 0) {
			throw new Error('No text fields specified for keyword search.');
		}

		this.ensureBM25Index(textFields, options?.k1, options?.b);

		let results = this.bm25Index!.search(query, k * 2);

		if (options?.filter) {
			results = results.filter(r => {
				const stored = this.vectors.get(r.id);
				return stored && evaluateFilter(stored.metadata, options.filter!);
			});
		}

		return results.slice(0, k);
	}

	hybridSearch(options: HybridSearchOptions): HybridSearchResult[] {
		const {
			mode,
			k,
			queryVector,
			keywords,
			textFields = this.bm25TextFields.length > 0 ? this.bm25TextFields : ['content', 'text', 'title'],
			filter,
			minSimilarity,
			alpha = 0.5,
			fusionMethod = 'rrf',
			rrfConstant = 60,
			bm25K1,
			bm25B,
		} = options;

		if (mode === 'vector') {
			if (!queryVector) throw new Error('queryVector is required for vector search mode');
			return this.search(queryVector, k, { filter, minSimilarity }).map(r => ({
				id: r.id,
				score: r.similarity,
				vectorSimilarity: r.similarity,
				metadata: r.metadata,
			}));
		}

		if (mode === 'keyword') {
			if (!keywords) throw new Error('keywords is required for keyword search mode');
			return this.keywordSearch(keywords, k, { textFields, filter, k1: bm25K1, b: bm25B }).map(r => ({
				id: r.id,
				score: r.score,
				keywordScore: r.score,
				metadata: r.metadata,
			}));
		}

		// Hybrid search
		if (!queryVector) throw new Error('queryVector is required for hybrid search mode');
		if (!keywords) throw new Error('keywords is required for hybrid search mode');

		const fetchK = Math.max(k * 3, 50);
		const vectorResults = this.search(queryVector, fetchK, { filter, minSimilarity });
		const keywordResults = this.keywordSearch(keywords, fetchK, { textFields, filter, k1: bm25K1, b: bm25B });

		const vectorForFusion: VectorSearchResult[] = vectorResults.map(r => ({
			id: r.id,
			distance: r.distance,
			similarity: r.similarity,
			metadata: r.metadata,
		}));

		return hybridFusion(vectorForFusion, keywordResults, k, fusionMethod, { alpha, rrfConstant });
	}

	// ============================================================================
	// Serialization
	// ============================================================================

	export(): SerializedDB {
		const serializedVectors: SerializedVector[] = [];

		for (const stored of this.vectors.values()) {
			const serialized: SerializedVector = {
				id: stored.id,
				vector: stored.vector,
				metadata: stored.metadata,
				norm: stored.norm,
				createdAt: stored.createdAt,
				updatedAt: stored.updatedAt,
			};

			// Serialize quantized vectors as base64
			if (stored.quantizedInt8) {
				serialized.quantizedInt8 = int8ToBase64(stored.quantizedInt8);
			}
			if (stored.quantizedBinary) {
				serialized.quantizedBinary = uint8ToBase64(stored.quantizedBinary);
			}

			serializedVectors.push(serialized);
		}

		const data: SerializedDB = {
			version: '3.0.0',  // Version bump for quantization support
			dimensions: this._dimensions,
			distance: this._distance,
			indexType: this._indexType,
			quantization: this._quantization,
			vectors: serializedVectors,
		};

		if (this.bm25Index) {
			data.bm25Index = this.bm25Index.serialize();
		}

		return data;
	}

	static import(data: SerializedDB): VectorDB {
		const db = new VectorDB({
			dimensions: data.dimensions,
			distance: data.distance,
			indexType: data.indexType,
			quantization: data.quantization,
		});

		for (const stored of data.vectors) {
			const entry: StoredVector = {
				id: stored.id,
				vector: stored.vector,
				metadata: stored.metadata,
				norm: stored.norm,
				createdAt: stored.createdAt ?? Date.now(),
				updatedAt: stored.updatedAt ?? Date.now(),
			};

			// Deserialize quantized vectors from base64
			if (stored.quantizedInt8) {
				entry.quantizedInt8 = base64ToInt8(stored.quantizedInt8);
			}
			if (stored.quantizedBinary) {
				entry.quantizedBinary = base64ToUint8(stored.quantizedBinary);
			}

			db.vectors.set(stored.id, entry);
		}

		if (data.bm25Index) {
			db.bm25Index = BM25Index.deserialize(data.bm25Index);
			db.bm25TextFields = data.bm25Index.textFields;
		}

		return db;
	}

	stats(): Record<string, unknown> {
		const vectorCount = this.vectors.size;
		const float32Size = vectorCount * this._dimensions * 4;  // bytes
		const quantizedSize = vectorCount * getQuantizedSize(this._dimensions, this._quantization);

		const stats: Record<string, unknown> = {
			dimensions: this._dimensions,
			distance: this._distance,
			indexType: this._indexType,
			quantization: this._quantization,
			vectorCount,
			memoryEstimate: {
				float32MB: float32Size / (1024 * 1024),
				quantizedMB: quantizedSize / (1024 * 1024),
				savingsPercent: calculateSavings(this._dimensions, this._quantization),
			},
		};

		if (this.bm25Index) {
			stats.bm25 = this.bm25Index.getStats();
		}

		return stats;
	}
}
