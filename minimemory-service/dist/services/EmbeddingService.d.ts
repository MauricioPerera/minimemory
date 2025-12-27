/**
 * EmbeddingService - Generate vector embeddings using Workers AI
 *
 * Uses Google's EmbeddingGemma-300m model:
 * - 768 dimensions (Matryoshka: truncatable to 256, 128)
 * - 100+ languages supported
 * - ~15ms inference latency
 * - $0.011 per 1000 neurons (768 dims = 768 neurons)
 *
 * Free tier: 10,000 neurons/day (~13 embeddings/day)
 */
export type EmbeddingDimensions = 768 | 512 | 256 | 128;
export interface EmbeddingOptions {
    /** Target dimensions (Matryoshka truncation) */
    dimensions?: EmbeddingDimensions;
    /** Whether to normalize the output vector */
    normalize?: boolean;
}
export interface EmbeddingResult {
    embedding: number[];
    dimensions: number;
    model: string;
    truncated: boolean;
}
export interface BatchEmbeddingResult {
    embeddings: number[][];
    dimensions: number;
    model: string;
    count: number;
}
interface Ai {
    run(model: string, inputs: Record<string, unknown>): Promise<unknown>;
}
export declare class EmbeddingService {
    private ai;
    private model;
    private defaultDimensions;
    constructor(ai: Ai, options?: {
        model?: string;
        defaultDimensions?: EmbeddingDimensions;
    });
    /**
     * Generate embedding for a single text
     */
    embed(text: string, options?: EmbeddingOptions): Promise<EmbeddingResult>;
    /**
     * Generate embeddings for multiple texts (batch)
     */
    embedBatch(texts: string[], options?: EmbeddingOptions): Promise<BatchEmbeddingResult>;
    /**
     * Get embedding for text, with caching support
     * Returns null if text is empty or only whitespace
     */
    getEmbedding(text: string, options?: EmbeddingOptions): Promise<number[] | null>;
    /**
     * Get the configured default dimensions
     */
    get dimensions(): EmbeddingDimensions;
    /**
     * Get the model name
     */
    get modelName(): string;
    /**
     * Estimate cost for N embeddings
     * $0.011 per 1000 neurons
     */
    static estimateCost(count: number, dimensions?: EmbeddingDimensions): number;
    /**
     * Get daily free tier limit (in number of embeddings)
     * Free tier: 10,000 neurons/day
     */
    static getDailyFreeLimit(dimensions?: EmbeddingDimensions): number;
}
/**
 * Create a mock embedding service for testing without Workers AI
 * Generates deterministic embeddings based on text hash
 */
export declare function createMockEmbeddingService(dimensions?: EmbeddingDimensions): EmbeddingService;
export default EmbeddingService;
//# sourceMappingURL=EmbeddingService.d.ts.map