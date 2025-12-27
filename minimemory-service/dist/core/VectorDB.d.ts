/**
 * VectorDB - In-memory vector database with hybrid search support
 * Supports vector quantization for memory efficiency (int8, binary)
 */
import { BM25SearchResult, SerializedBM25Index } from './BM25Index.js';
import { SearchMode, FusionMethod, HybridSearchResult } from './HybridSearch.js';
import { QuantizationType } from './Quantization.js';
export type DistanceMetric = 'cosine' | 'euclidean' | 'dot';
export type IndexType = 'flat' | 'hnsw';
export type { SearchMode, FusionMethod, HybridSearchResult, BM25SearchResult, QuantizationType };
export interface VectorDBOptions {
    dimensions: number;
    distance?: DistanceMetric;
    indexType?: IndexType;
    quantization?: QuantizationType;
    rescoreOversample?: number;
}
export interface SearchResult {
    id: string;
    distance: number;
    similarity: number;
    metadata?: Record<string, unknown>;
}
export type FilterOperator = '$eq' | '$ne' | '$gt' | '$gte' | '$lt' | '$lte' | '$in' | '$nin' | '$exists' | '$contains' | '$startsWith' | '$endsWith';
export type FilterCondition = {
    $eq: unknown;
} | {
    $ne: unknown;
} | {
    $gt: number | string | Date;
} | {
    $gte: number | string | Date;
} | {
    $lt: number | string | Date;
} | {
    $lte: number | string | Date;
} | {
    $in: unknown[];
} | {
    $nin: unknown[];
} | {
    $exists: boolean;
} | {
    $contains: string;
} | {
    $startsWith: string;
} | {
    $endsWith: string;
};
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
interface SerializedVector {
    id: string;
    vector: number[];
    metadata: Record<string, unknown> | null;
    norm?: number;
    createdAt: number;
    updatedAt: number;
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
export declare class VectorDB {
    private vectors;
    private readonly _dimensions;
    private readonly _distance;
    private readonly _indexType;
    private readonly _quantization;
    private readonly _rescoreOversample;
    private bm25Index;
    private bm25TextFields;
    constructor(options: VectorDBOptions);
    get dimensions(): number;
    get distance(): DistanceMetric;
    get indexType(): IndexType;
    get quantization(): QuantizationType;
    get length(): number;
    private computeNorm;
    private calculateDistance;
    /**
     * Create quantized representations for a vector
     */
    private createQuantizedRepresentations;
    insert(id: string, vector: number[], metadata?: Record<string, unknown>): void;
    upsert(id: string, vector: number[], metadata?: Record<string, unknown>): void;
    /**
     * Search using quantized vectors (fast approximate search)
     */
    private searchQuantized;
    /**
     * Rescore candidates using full-precision vectors
     */
    private rescoreCandidates;
    search(query: number[], k: number, options?: Partial<SearchOptions>): SearchResult[];
    get(id: string): {
        id: string;
        vector: number[];
        metadata: Record<string, unknown> | null;
        createdAt: number;
        updatedAt: number;
    } | null;
    delete(id: string): boolean;
    contains(id: string): boolean;
    clear(): void;
    getIds(): string[];
    configureBM25(options: {
        textFields: string[];
        k1?: number;
        b?: number;
    }): void;
    private ensureBM25Index;
    keywordSearch(query: string, k: number, options?: {
        textFields?: string[];
        filter?: MetadataFilter;
        k1?: number;
        b?: number;
    }): BM25SearchResult[];
    hybridSearch(options: HybridSearchOptions): HybridSearchResult[];
    export(): SerializedDB;
    static import(data: SerializedDB): VectorDB;
    stats(): Record<string, unknown>;
}
//# sourceMappingURL=VectorDB.d.ts.map