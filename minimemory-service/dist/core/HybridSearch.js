/**
 * Hybrid Search - Combines vector similarity and keyword (BM25) search
 */
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
export function reciprocalRankFusion(vectorResults, keywordResults, k, rrfConstant = 60) {
    // Build rank maps
    const vectorRanks = new Map();
    const keywordRanks = new Map();
    vectorResults.forEach((result, index) => {
        vectorRanks.set(result.id, index + 1); // 1-based ranking
    });
    keywordResults.forEach((result, index) => {
        keywordRanks.set(result.id, index + 1); // 1-based ranking
    });
    // Collect all unique document IDs
    const allIds = new Set([
        ...vectorRanks.keys(),
        ...keywordRanks.keys(),
    ]);
    // Calculate RRF scores
    const scores = [];
    for (const id of allIds) {
        const vectorRank = vectorRanks.get(id);
        const keywordRank = keywordRanks.get(id);
        // RRF score contribution from each result set
        let score = 0;
        if (vectorRank !== undefined) {
            score += 1 / (rrfConstant + vectorRank);
        }
        if (keywordRank !== undefined) {
            score += 1 / (rrfConstant + keywordRank);
        }
        // Find original result data
        const vectorResult = vectorResults.find(r => r.id === id);
        const keywordResult = keywordResults.find(r => r.id === id);
        scores.push({
            id,
            score,
            vectorRank,
            keywordRank,
            vectorSimilarity: vectorResult?.similarity,
            keywordScore: keywordResult?.score,
            metadata: vectorResult?.metadata || keywordResult?.metadata,
        });
    }
    // Sort by RRF score (descending)
    scores.sort((a, b) => b.score - a.score);
    // Return top k
    return scores.slice(0, k);
}
/**
 * Normalizes scores to [0, 1] range using min-max normalization
 */
function normalizeScores(results) {
    if (results.length === 0) {
        return new Map();
    }
    const scores = results.map(r => r.score);
    const minScore = Math.min(...scores);
    const maxScore = Math.max(...scores);
    const range = maxScore - minScore;
    const normalized = new Map();
    for (const result of results) {
        const normScore = range === 0 ? 1 : (result.score - minScore) / range;
        normalized.set(result.id, normScore);
    }
    return normalized;
}
/**
 * Performs weighted score combination
 *
 * Formula: combined_score = alpha * vector_score + (1 - alpha) * keyword_score
 */
export function weightedCombination(vectorResults, keywordResults, k, alpha = 0.5) {
    // Clamp alpha to [0, 1]
    alpha = Math.max(0, Math.min(1, alpha));
    // Normalize vector scores (similarity is already [0, 1] for cosine)
    const normalizedVector = new Map();
    for (const result of vectorResults) {
        normalizedVector.set(result.id, result.similarity);
    }
    // Normalize BM25 scores (need min-max normalization)
    const normalizedKeyword = normalizeScores(keywordResults.map(r => ({ id: r.id, score: r.score })));
    // Collect all unique document IDs
    const allIds = new Set([
        ...normalizedVector.keys(),
        ...normalizedKeyword.keys(),
    ]);
    // Calculate combined scores
    const scores = [];
    for (const id of allIds) {
        const vectorScore = normalizedVector.get(id) ?? 0;
        const keywordScore = normalizedKeyword.get(id) ?? 0;
        // Weighted combination
        const combinedScore = alpha * vectorScore + (1 - alpha) * keywordScore;
        // Find original result data
        const vectorResult = vectorResults.find(r => r.id === id);
        const keywordResult = keywordResults.find(r => r.id === id);
        // Calculate ranks
        const vectorRank = vectorResults.findIndex(r => r.id === id);
        const keywordRank = keywordResults.findIndex(r => r.id === id);
        scores.push({
            id,
            score: combinedScore,
            vectorRank: vectorRank >= 0 ? vectorRank + 1 : undefined,
            keywordRank: keywordRank >= 0 ? keywordRank + 1 : undefined,
            vectorSimilarity: vectorResult?.similarity,
            keywordScore: keywordResult?.score,
            metadata: vectorResult?.metadata || keywordResult?.metadata,
        });
    }
    // Sort by combined score (descending)
    scores.sort((a, b) => b.score - a.score);
    // Return top k
    return scores.slice(0, k);
}
/**
 * Performs hybrid search using the specified fusion method
 */
export function hybridFusion(vectorResults, keywordResults, k, method = 'rrf', options) {
    if (method === 'rrf') {
        return reciprocalRankFusion(vectorResults, keywordResults, k, options?.rrfConstant ?? 60);
    }
    else {
        return weightedCombination(vectorResults, keywordResults, k, options?.alpha ?? 0.5);
    }
}
//# sourceMappingURL=HybridSearch.js.map