/**
 * Hybrid Search - Combines vector similarity and keyword (BM25) search
 */
import type { BM25SearchResult } from './BM25Index.js';
export type SearchMode = 'vector' | 'keyword' | 'hybrid';
export type FusionMethod = 'rrf' | 'weighted';
export interface VectorSearchResult {
    id: string;
    distance: number;
    similarity: number;
    metadata?: Record<string, unknown>;
}
export interface HybridSearchResult {
    id: string;
    score: number;
    vectorRank?: number;
    keywordRank?: number;
    vectorSimilarity?: number;
    keywordScore?: number;
    metadata?: Record<string, unknown>;
}
/**
 * Performs Reciprocal Rank Fusion (RRF) on two result sets
 *
 * RRF is preferred for hybrid search because:
 * 1. It does not require score normalization
 * 2. It is robust to score distribution differences
 * 3. It handles missing documents gracefully
 *
 * Formula: RRF_score(d) = 1/(k + rank_vector(d)) + 1/(k + rank_keyword(d))
 */
export declare function reciprocalRankFusion(vectorResults: VectorSearchResult[], keywordResults: BM25SearchResult[], k: number, rrfConstant?: number): HybridSearchResult[];
/**
 * Performs weighted score combination
 *
 * Formula: combined_score = alpha * vector_score + (1 - alpha) * keyword_score
 */
export declare function weightedCombination(vectorResults: VectorSearchResult[], keywordResults: BM25SearchResult[], k: number, alpha?: number): HybridSearchResult[];
/**
 * Performs hybrid search using the specified fusion method
 */
export declare function hybridFusion(vectorResults: VectorSearchResult[], keywordResults: BM25SearchResult[], k: number, method?: FusionMethod, options?: {
    alpha?: number;
    rrfConstant?: number;
}): HybridSearchResult[];
//# sourceMappingURL=HybridSearch.d.ts.map