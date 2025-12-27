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
  // For rescoring with binary quantization
  oversample?: number;  // Default: 4 (fetch 4x candidates for rescoring)
}

// ============================================================================
// Scalar Quantization (int8)
// Converts float32 [-1, 1] → int8 [-127, 127]
// Memory: 4x reduction (4 bytes → 1 byte per value)
// Accuracy: ~99% with proper normalization
// ============================================================================

/**
 * Quantize a float32 vector to int8
 * Assumes input is normalized to [-1, 1] range (e.g., cosine similarity embeddings)
 */
export function quantizeScalar(vector: number[]): Int8Array {
  const quantized = new Int8Array(vector.length);
  for (let i = 0; i < vector.length; i++) {
    // Clamp to [-1, 1] and scale to [-127, 127]
    const clamped = Math.max(-1, Math.min(1, vector[i]));
    quantized[i] = Math.round(clamped * 127);
  }
  return quantized;
}

/**
 * Dequantize int8 vector back to float32
 */
export function dequantizeScalar(quantized: Int8Array): number[] {
  const vector = new Array(quantized.length);
  for (let i = 0; i < quantized.length; i++) {
    vector[i] = quantized[i] / 127;
  }
  return vector;
}

/**
 * Compute dot product between two int8 vectors
 * Much faster than float32 due to integer arithmetic
 */
export function dotProductInt8(a: Int8Array, b: Int8Array): number {
  let sum = 0;
  const len = Math.min(a.length, b.length);
  for (let i = 0; i < len; i++) {
    sum += a[i] * b[i];
  }
  return sum;
}

/**
 * Compute cosine similarity between two int8 vectors
 * Returns value in range [-1, 1]
 */
export function cosineSimilarityInt8(a: Int8Array, b: Int8Array): number {
  let dot = 0;
  let normA = 0;
  let normB = 0;

  const len = Math.min(a.length, b.length);
  for (let i = 0; i < len; i++) {
    dot += a[i] * b[i];
    normA += a[i] * a[i];
    normB += b[i] * b[i];
  }

  const denom = Math.sqrt(normA) * Math.sqrt(normB);
  if (denom === 0) return 0;

  // Scale back: dot is in range [-127*127*dims, 127*127*dims]
  // After division by norms, result is already in [-1, 1]
  return dot / denom;
}

// ============================================================================
// Binary Quantization (1-bit)
// Converts float32 → 1 bit per value (positive = 1, negative = 0)
// Memory: 32x reduction (4 bytes → 0.125 bytes per value)
// Accuracy: ~95% (best with rescoring using original vectors)
// ============================================================================

/**
 * Quantize a float32 vector to binary (1-bit per value)
 * Each bit represents: 1 if value > 0, else 0
 */
export function quantizeBinary(vector: number[]): Uint8Array {
  const bytes = Math.ceil(vector.length / 8);
  const quantized = new Uint8Array(bytes);

  for (let i = 0; i < vector.length; i++) {
    if (vector[i] > 0) {
      quantized[Math.floor(i / 8)] |= (1 << (i % 8));
    }
  }

  return quantized;
}

/**
 * Compute Hamming distance between two binary vectors
 * Uses XOR + population count (very fast)
 * Lower distance = more similar
 */
export function hammingDistance(a: Uint8Array, b: Uint8Array): number {
  let distance = 0;
  const len = Math.min(a.length, b.length);

  for (let i = 0; i < len; i++) {
    let xor = a[i] ^ b[i];
    // Population count (count set bits)
    while (xor) {
      distance += xor & 1;
      xor >>>= 1;
    }
  }

  return distance;
}

/**
 * Convert Hamming distance to similarity score [0, 1]
 * @param distance Hamming distance
 * @param totalBits Total number of bits in the vector
 */
export function hammingToSimilarity(distance: number, totalBits: number): number {
  // Hamming similarity = 1 - (distance / totalBits)
  return 1 - (distance / totalBits);
}

// ============================================================================
// Serialization helpers for storage
// ============================================================================

/**
 * Convert Int8Array to base64 string for JSON/D1 storage
 */
export function int8ToBase64(arr: Int8Array): string {
  const bytes = new Uint8Array(arr.buffer, arr.byteOffset, arr.byteLength);
  let binary = '';
  for (let i = 0; i < bytes.length; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary);
}

/**
 * Convert base64 string back to Int8Array
 */
export function base64ToInt8(base64: string): Int8Array {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return new Int8Array(bytes.buffer);
}

/**
 * Convert Uint8Array to base64 string for JSON/D1 storage
 */
export function uint8ToBase64(arr: Uint8Array): string {
  let binary = '';
  for (let i = 0; i < arr.length; i++) {
    binary += String.fromCharCode(arr[i]);
  }
  return btoa(binary);
}

/**
 * Convert base64 string back to Uint8Array
 */
export function base64ToUint8(base64: string): Uint8Array {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

// ============================================================================
// Matryoshka dimension truncation
// ============================================================================

/**
 * Truncate vector to target dimensions (Matryoshka Representation Learning)
 * Valid targets for EmbeddingGemma: 768, 512, 256, 128
 */
export function truncateDimensions(vector: number[], targetDims: number): number[] {
  if (vector.length <= targetDims) {
    return vector;
  }
  return vector.slice(0, targetDims);
}

// ============================================================================
// Combined utilities
// ============================================================================

export interface QuantizedVector {
  type: QuantizationType;
  data: string;  // Base64 encoded
  originalDims: number;
}

/**
 * Quantize a vector with the specified method
 */
export function quantizeVector(
  vector: number[],
  type: QuantizationType,
  targetDims?: number
): QuantizedVector {
  // Optional dimension truncation (Matryoshka)
  const truncated = targetDims ? truncateDimensions(vector, targetDims) : vector;

  switch (type) {
    case 'int8': {
      const quantized = quantizeScalar(truncated);
      return {
        type: 'int8',
        data: int8ToBase64(quantized),
        originalDims: truncated.length,
      };
    }
    case 'binary': {
      const quantized = quantizeBinary(truncated);
      return {
        type: 'binary',
        data: uint8ToBase64(quantized),
        originalDims: truncated.length,
      };
    }
    default:
      throw new Error(`Unknown quantization type: ${type}`);
  }
}

/**
 * Compute similarity between two quantized vectors
 */
export function quantizedSimilarity(
  a: QuantizedVector,
  b: QuantizedVector
): number {
  if (a.type !== b.type) {
    throw new Error('Cannot compare vectors with different quantization types');
  }

  switch (a.type) {
    case 'int8': {
      const vecA = base64ToInt8(a.data);
      const vecB = base64ToInt8(b.data);
      return cosineSimilarityInt8(vecA, vecB);
    }
    case 'binary': {
      const vecA = base64ToUint8(a.data);
      const vecB = base64ToUint8(b.data);
      const distance = hammingDistance(vecA, vecB);
      return hammingToSimilarity(distance, a.originalDims);
    }
    default:
      throw new Error(`Unknown quantization type: ${a.type}`);
  }
}

/**
 * Get memory size in bytes for a quantized vector
 */
export function getQuantizedSize(dims: number, type: QuantizationType): number {
  switch (type) {
    case 'none':
      return dims * 4;  // float32 = 4 bytes
    case 'int8':
      return dims;      // int8 = 1 byte
    case 'binary':
      return Math.ceil(dims / 8);  // 1 bit per value
    default:
      return dims * 4;
  }
}

/**
 * Calculate memory savings percentage
 */
export function calculateSavings(dims: number, type: QuantizationType): number {
  const original = dims * 4;  // float32
  const quantized = getQuantizedSize(dims, type);
  return ((original - quantized) / original) * 100;
}
