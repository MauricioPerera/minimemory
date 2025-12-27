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

// Workers AI binding type
interface Ai {
  run(model: string, inputs: Record<string, unknown>): Promise<unknown>;
}

/**
 * Normalize a vector to unit length
 */
function normalizeVector(vector: number[]): number[] {
  let norm = 0;
  for (const v of vector) {
    norm += v * v;
  }
  norm = Math.sqrt(norm);
  if (norm === 0) return vector;
  return vector.map(v => v / norm);
}

/**
 * Truncate vector to target dimensions (Matryoshka)
 */
function truncateVector(vector: number[], targetDims: number): number[] {
  if (vector.length <= targetDims) return vector;
  return vector.slice(0, targetDims);
}

export class EmbeddingService {
  private ai: Ai;
  private model: string;
  private defaultDimensions: EmbeddingDimensions;

  constructor(ai: Ai, options?: {
    model?: string;
    defaultDimensions?: EmbeddingDimensions;
  }) {
    this.ai = ai;
    this.model = options?.model || '@cf/google/gemma-embedding-300m';
    this.defaultDimensions = options?.defaultDimensions || 768;
  }

  /**
   * Generate embedding for a single text
   */
  async embed(text: string, options?: EmbeddingOptions): Promise<EmbeddingResult> {
    const dims = options?.dimensions || this.defaultDimensions;
    const shouldNormalize = options?.normalize ?? true;

    const response = await this.ai.run(this.model, {
      text: [text],
    }) as { data: number[][] };

    if (!response?.data?.[0]) {
      throw new Error('Failed to generate embedding: empty response');
    }

    let embedding = response.data[0];
    const originalDims = embedding.length;
    const truncated = dims < originalDims;

    // Matryoshka truncation
    if (truncated) {
      embedding = truncateVector(embedding, dims);
    }

    // Normalize after truncation (important for Matryoshka)
    if (shouldNormalize) {
      embedding = normalizeVector(embedding);
    }

    return {
      embedding,
      dimensions: embedding.length,
      model: this.model,
      truncated,
    };
  }

  /**
   * Generate embeddings for multiple texts (batch)
   */
  async embedBatch(texts: string[], options?: EmbeddingOptions): Promise<BatchEmbeddingResult> {
    const dims = options?.dimensions || this.defaultDimensions;
    const shouldNormalize = options?.normalize ?? true;

    // Workers AI supports batch input
    const response = await this.ai.run(this.model, {
      text: texts,
    }) as { data: number[][] };

    if (!response?.data || response.data.length === 0) {
      throw new Error('Failed to generate embeddings: empty response');
    }

    let embeddings = response.data;

    // Matryoshka truncation + normalize for each
    embeddings = embeddings.map(emb => {
      let result = dims < emb.length ? truncateVector(emb, dims) : emb;
      if (shouldNormalize) {
        result = normalizeVector(result);
      }
      return result;
    });

    return {
      embeddings,
      dimensions: dims,
      model: this.model,
      count: embeddings.length,
    };
  }

  /**
   * Get embedding for text, with caching support
   * Returns null if text is empty or only whitespace
   */
  async getEmbedding(text: string, options?: EmbeddingOptions): Promise<number[] | null> {
    const trimmed = text?.trim();
    if (!trimmed) {
      return null;
    }

    const result = await this.embed(trimmed, options);
    return result.embedding;
  }

  /**
   * Get the configured default dimensions
   */
  get dimensions(): EmbeddingDimensions {
    return this.defaultDimensions;
  }

  /**
   * Get the model name
   */
  get modelName(): string {
    return this.model;
  }

  /**
   * Estimate cost for N embeddings
   * $0.011 per 1000 neurons
   */
  static estimateCost(count: number, dimensions: EmbeddingDimensions = 768): number {
    const neurons = count * dimensions;
    return (neurons / 1000) * 0.011;
  }

  /**
   * Get daily free tier limit (in number of embeddings)
   * Free tier: 10,000 neurons/day
   */
  static getDailyFreeLimit(dimensions: EmbeddingDimensions = 768): number {
    return Math.floor(10000 / dimensions);
  }
}

/**
 * Create a mock embedding service for testing without Workers AI
 * Generates deterministic embeddings based on text hash
 */
export function createMockEmbeddingService(dimensions: EmbeddingDimensions = 768): EmbeddingService {
  const mockAi: Ai = {
    async run(_model: string, inputs: Record<string, unknown>): Promise<unknown> {
      const texts = inputs.text as string[];
      const embeddings = texts.map(text => {
        // Generate deterministic embedding based on text
        const embedding = new Array(768).fill(0);
        for (let i = 0; i < text.length && i < 768; i++) {
          embedding[i] = (text.charCodeAt(i) - 64) / 100;
        }
        // Normalize
        let norm = 0;
        for (const v of embedding) norm += v * v;
        norm = Math.sqrt(norm) || 1;
        return embedding.map(v => v / norm);
      });
      return { data: embeddings };
    },
  };

  return new EmbeddingService(mockAi, { defaultDimensions: dimensions });
}

export default EmbeddingService;
