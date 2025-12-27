/**
 * Vector Quantization for memory-efficient storage and fast similarity search
 *
 * Supports:
 * - Scalar Quantization (int8): 4x memory reduction, ~99% accuracy
 * - Binary Quantization (1-bit): 32x memory reduction, ~95% accuracy (needs rescoring)
 */
export type QuantizationType = 'none' | 'int8' | 'binary';
export interface QuantizationConfig {
    type: QuantizationType;
    oversample?: number;
}
/**
 * Quantize a float32 vector to int8
 * Assumes input is normalized to [-1, 1] range (e.g., cosine similarity embeddings)
 */
export declare function quantizeScalar(vector: number[]): Int8Array;
/**
 * Dequantize int8 vector back to float32
 */
export declare function dequantizeScalar(quantized: Int8Array): number[];
/**
 * Compute dot product between two int8 vectors
 * Much faster than float32 due to integer arithmetic
 */
export declare function dotProductInt8(a: Int8Array, b: Int8Array): number;
/**
 * Compute cosine similarity between two int8 vectors
 * Returns value in range [-1, 1]
 */
export declare function cosineSimilarityInt8(a: Int8Array, b: Int8Array): number;
/**
 * Quantize a float32 vector to binary (1-bit per value)
 * Each bit represents: 1 if value > 0, else 0
 */
export declare function quantizeBinary(vector: number[]): Uint8Array;
/**
 * Compute Hamming distance between two binary vectors
 * Uses XOR + population count (very fast)
 * Lower distance = more similar
 */
export declare function hammingDistance(a: Uint8Array, b: Uint8Array): number;
/**
 * Convert Hamming distance to similarity score [0, 1]
 * @param distance Hamming distance
 * @param totalBits Total number of bits in the vector
 */
export declare function hammingToSimilarity(distance: number, totalBits: number): number;
/**
 * Convert Int8Array to base64 string for JSON/D1 storage
 */
export declare function int8ToBase64(arr: Int8Array): string;
/**
 * Convert base64 string back to Int8Array
 */
export declare function base64ToInt8(base64: string): Int8Array;
/**
 * Convert Uint8Array to base64 string for JSON/D1 storage
 */
export declare function uint8ToBase64(arr: Uint8Array): string;
/**
 * Convert base64 string back to Uint8Array
 */
export declare function base64ToUint8(base64: string): Uint8Array;
/**
 * Truncate vector to target dimensions (Matryoshka Representation Learning)
 * Valid targets for EmbeddingGemma: 768, 512, 256, 128
 */
export declare function truncateDimensions(vector: number[], targetDims: number): number[];
export interface QuantizedVector {
    type: QuantizationType;
    data: string;
    originalDims: number;
}
/**
 * Quantize a vector with the specified method
 */
export declare function quantizeVector(vector: number[], type: QuantizationType, targetDims?: number): QuantizedVector;
/**
 * Compute similarity between two quantized vectors
 */
export declare function quantizedSimilarity(a: QuantizedVector, b: QuantizedVector): number;
/**
 * Get memory size in bytes for a quantized vector
 */
export declare function getQuantizedSize(dims: number, type: QuantizationType): number;
/**
 * Calculate memory savings percentage
 */
export declare function calculateSavings(dims: number, type: QuantizationType): number;
//# sourceMappingURL=Quantization.d.ts.map