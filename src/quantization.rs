//! # Vector Quantization Module
//!
//! Provides vector compression techniques to reduce memory usage while maintaining search quality.
//!
//! ## Quantization Types
//!
//! - **None**: Full f32 precision (4 bytes per dimension)
//! - **Scalar (Int8)**: 8-bit integers (1 byte per dimension) - 4x compression
//! - **Binary**: 1-bit per dimension (1/32 bytes per dimension) - 32x compression
//!
//! ## Trade-offs
//!
//! | Type | Compression | Accuracy | Speed |
//! |------|-------------|----------|-------|
//! | None | 1x | 100% | Baseline |
//! | Int8 | 4x | ~99% | Faster |
//! | Binary | 32x | ~90-95% | Much faster |

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};

/// Type of quantization to apply to vectors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum QuantizationType {
    /// No quantization - full f32 precision (4 bytes per dimension)
    #[default]
    None,
    /// Scalar quantization to int8 (-128 to 127)
    /// Provides 4x memory reduction with minimal accuracy loss
    Int8,
    /// Binary quantization (1 bit per dimension)
    /// Provides 32x memory reduction, best for high-dimensional vectors
    Binary,
}

/// Parameters for scalar quantization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalarQuantParams {
    /// Minimum value in the training set
    pub min_val: f32,
    /// Maximum value in the training set
    pub max_val: f32,
    /// Scale factor for quantization
    pub scale: f32,
}

impl Default for ScalarQuantParams {
    fn default() -> Self {
        Self {
            min_val: -1.0,
            max_val: 1.0,
            scale: 127.0,
        }
    }
}

impl ScalarQuantParams {
    /// Create new parameters from a range
    pub fn new(min_val: f32, max_val: f32) -> Self {
        let range = max_val - min_val;
        let scale = if range > 0.0 { 255.0 / range } else { 1.0 };
        Self {
            min_val,
            max_val,
            scale,
        }
    }

    /// Learn parameters from a set of vectors
    pub fn from_vectors(vectors: &[&[f32]]) -> Self {
        if vectors.is_empty() {
            return Self::default();
        }

        let mut min_val = f32::MAX;
        let mut max_val = f32::MIN;

        for vec in vectors {
            for &val in *vec {
                if val < min_val {
                    min_val = val;
                }
                if val > max_val {
                    max_val = val;
                }
            }
        }

        Self::new(min_val, max_val)
    }

    /// Quantize a single f32 value to i8
    #[inline]
    pub fn quantize_value(&self, val: f32) -> i8 {
        let clamped = val.clamp(self.min_val, self.max_val);
        let normalized = (clamped - self.min_val) * self.scale;
        (normalized - 128.0).round() as i8
    }

    /// Dequantize a single i8 value back to f32
    #[inline]
    pub fn dequantize_value(&self, val: i8) -> f32 {
        let normalized = (val as f32 + 128.0) / self.scale;
        normalized + self.min_val
    }
}

/// A quantized vector representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuantizedVector {
    /// Full precision f32 vector
    Full(Vec<f32>),
    /// Scalar quantized to int8
    Int8 {
        data: Vec<i8>,
        params: ScalarQuantParams,
    },
    /// Binary quantized (packed bits)
    Binary { data: Vec<u64>, dimensions: usize },
}

impl QuantizedVector {
    /// Get the number of dimensions
    pub fn dimensions(&self) -> usize {
        match self {
            QuantizedVector::Full(v) => v.len(),
            QuantizedVector::Int8 { data, .. } => data.len(),
            QuantizedVector::Binary { dimensions, .. } => *dimensions,
        }
    }

    /// Get memory usage in bytes
    pub fn memory_bytes(&self) -> usize {
        match self {
            QuantizedVector::Full(v) => v.len() * 4,
            QuantizedVector::Int8 { data, .. } => {
                data.len() + std::mem::size_of::<ScalarQuantParams>()
            }
            QuantizedVector::Binary { data, .. } => data.len() * 8,
        }
    }

    /// Dequantize back to f32 vector
    pub fn to_f32(&self) -> Vec<f32> {
        match self {
            QuantizedVector::Full(v) => v.clone(),
            QuantizedVector::Int8 { data, params } => {
                data.iter().map(|&v| params.dequantize_value(v)).collect()
            }
            QuantizedVector::Binary { data, dimensions } => {
                let mut result = Vec::with_capacity(*dimensions);
                for i in 0..*dimensions {
                    let word_idx = i / 64;
                    let bit_idx = i % 64;
                    let bit = (data[word_idx] >> bit_idx) & 1;
                    result.push(if bit == 1 { 1.0 } else { -1.0 });
                }
                result
            }
        }
    }
}

/// Quantizer for converting vectors to compressed representations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quantizer {
    /// Type of quantization
    pub quant_type: QuantizationType,
    /// Parameters for scalar quantization (if applicable)
    pub scalar_params: Option<ScalarQuantParams>,
    /// Number of dimensions
    pub dimensions: usize,
}

impl Quantizer {
    /// Create a new quantizer with no quantization
    pub fn none(dimensions: usize) -> Self {
        Self {
            quant_type: QuantizationType::None,
            scalar_params: None,
            dimensions,
        }
    }

    /// Create a new int8 quantizer with default parameters
    pub fn int8(dimensions: usize) -> Self {
        Self {
            quant_type: QuantizationType::Int8,
            scalar_params: Some(ScalarQuantParams::default()),
            dimensions,
        }
    }

    /// Create a new int8 quantizer trained on sample vectors
    pub fn int8_trained(dimensions: usize, samples: &[&[f32]]) -> Self {
        Self {
            quant_type: QuantizationType::Int8,
            scalar_params: Some(ScalarQuantParams::from_vectors(samples)),
            dimensions,
        }
    }

    /// Create a new binary quantizer
    pub fn binary(dimensions: usize) -> Self {
        Self {
            quant_type: QuantizationType::Binary,
            scalar_params: None,
            dimensions,
        }
    }

    /// Quantize a vector
    pub fn quantize(&self, vector: &[f32]) -> Result<QuantizedVector> {
        if vector.len() != self.dimensions {
            return Err(Error::DimensionMismatch {
                expected: self.dimensions,
                got: vector.len(),
            });
        }

        match self.quant_type {
            QuantizationType::None => Ok(QuantizedVector::Full(vector.to_vec())),

            QuantizationType::Int8 => {
                let params = self.scalar_params.as_ref().cloned().unwrap_or_default();
                let data: Vec<i8> = vector.iter().map(|&v| params.quantize_value(v)).collect();
                Ok(QuantizedVector::Int8 { data, params })
            }

            QuantizationType::Binary => {
                let num_words = self.dimensions.div_ceil(64);
                let mut data = vec![0u64; num_words];

                for (i, &val) in vector.iter().enumerate() {
                    if val > 0.0 {
                        let word_idx = i / 64;
                        let bit_idx = i % 64;
                        data[word_idx] |= 1u64 << bit_idx;
                    }
                }

                Ok(QuantizedVector::Binary {
                    data,
                    dimensions: self.dimensions,
                })
            }
        }
    }

    /// Dequantize a vector back to f32
    pub fn dequantize(&self, quantized: &QuantizedVector) -> Vec<f32> {
        quantized.to_f32()
    }

    /// Update scalar parameters from new samples
    pub fn train(&mut self, samples: &[&[f32]]) {
        if self.quant_type == QuantizationType::Int8 {
            self.scalar_params = Some(ScalarQuantParams::from_vectors(samples));
        }
    }
}

// ============================================================================
// Distance calculations for quantized vectors
// ============================================================================

/// Calculate cosine distance between two int8 quantized vectors
/// Returns approximate cosine distance (1 - cosine_similarity)
#[inline]
pub fn cosine_distance_int8(a: &[i8], b: &[i8]) -> f32 {
    debug_assert_eq!(a.len(), b.len());

    let mut dot: i32 = 0;
    let mut norm_a: i32 = 0;
    let mut norm_b: i32 = 0;

    for (&ai, &bi) in a.iter().zip(b.iter()) {
        let ai = ai as i32;
        let bi = bi as i32;
        dot += ai * bi;
        norm_a += ai * ai;
        norm_b += bi * bi;
    }

    let norm = (norm_a as f32).sqrt() * (norm_b as f32).sqrt();
    if norm == 0.0 {
        return 1.0;
    }

    1.0 - (dot as f32 / norm)
}

/// Calculate euclidean distance between two int8 quantized vectors
#[inline]
pub fn euclidean_distance_int8(a: &[i8], b: &[i8]) -> f32 {
    debug_assert_eq!(a.len(), b.len());

    let mut sum: i32 = 0;
    for (&ai, &bi) in a.iter().zip(b.iter()) {
        let diff = ai as i32 - bi as i32;
        sum += diff * diff;
    }

    (sum as f32).sqrt()
}

/// Calculate dot product distance between two int8 quantized vectors
#[inline]
pub fn dot_product_distance_int8(a: &[i8], b: &[i8]) -> f32 {
    debug_assert_eq!(a.len(), b.len());

    let mut dot: i32 = 0;
    for (&ai, &bi) in a.iter().zip(b.iter()) {
        dot += ai as i32 * bi as i32;
    }

    // Negative because we want to minimize distance
    -(dot as f32)
}

/// Calculate Hamming distance between two binary quantized vectors
/// Returns the number of differing bits
#[inline]
pub fn hamming_distance_binary(a: &[u64], b: &[u64]) -> u32 {
    debug_assert_eq!(a.len(), b.len());

    a.iter()
        .zip(b.iter())
        .map(|(&ai, &bi)| (ai ^ bi).count_ones())
        .sum()
}

/// Calculate approximate cosine distance using Hamming distance on binary vectors
/// This is a rough approximation where Hamming distance correlates with angular distance
#[inline]
pub fn cosine_distance_binary(a: &[u64], b: &[u64], dimensions: usize) -> f32 {
    let hamming = hamming_distance_binary(a, b);
    // Convert Hamming to approximate cosine distance
    // Hamming/dimensions gives fraction of differing bits
    // This correlates roughly with angular distance
    hamming as f32 / dimensions as f32
}

// ============================================================================
// Helper functions for quantized distance calculations
// ============================================================================

/// Calculate distance between two quantized vectors
pub fn quantized_distance(
    a: &QuantizedVector,
    b: &QuantizedVector,
    distance_type: crate::Distance,
) -> Result<f32> {
    use crate::Distance;

    match (a, b) {
        (QuantizedVector::Full(va), QuantizedVector::Full(vb)) => {
            Ok(distance_type.calculate(va, vb))
        }

        (QuantizedVector::Int8 { data: da, .. }, QuantizedVector::Int8 { data: db, .. }) => {
            Ok(match distance_type {
                Distance::Cosine => cosine_distance_int8(da, db),
                Distance::Euclidean => euclidean_distance_int8(da, db),
                Distance::DotProduct => dot_product_distance_int8(da, db),
            })
        }

        (
            QuantizedVector::Binary {
                data: da,
                dimensions: dim_a,
            },
            QuantizedVector::Binary {
                data: db,
                dimensions: dim_b,
            },
        ) => {
            if dim_a != dim_b {
                return Err(Error::DimensionMismatch {
                    expected: *dim_a,
                    got: *dim_b,
                });
            }
            Ok(match distance_type {
                Distance::Cosine => cosine_distance_binary(da, db, *dim_a),
                Distance::Euclidean => hamming_distance_binary(da, db) as f32,
                Distance::DotProduct => -((*dim_a as u32 - hamming_distance_binary(da, db)) as f32),
            })
        }

        _ => {
            // Mixed types - dequantize and compute
            let va = a.to_f32();
            let vb = b.to_f32();
            Ok(distance_type.calculate(&va, &vb))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scalar_quant_roundtrip() {
        let params = ScalarQuantParams::new(-1.0, 1.0);

        let original = 0.5f32;
        let quantized = params.quantize_value(original);
        let restored = params.dequantize_value(quantized);

        // Should be close (within quantization error)
        assert!((original - restored).abs() < 0.02);
    }

    #[test]
    fn test_int8_quantization() {
        // Use 32 dimensions - small vectors don't save space due to 12-byte params overhead
        let quantizer = Quantizer::int8(32);
        let vector: Vec<f32> = (0..32).map(|i| (i as f32 - 16.0) / 16.0).collect();

        let quantized = quantizer.quantize(&vector).unwrap();
        let restored = quantizer.dequantize(&quantized);

        // Check dimensions match
        assert_eq!(restored.len(), 32);

        // Check memory is smaller (32 bytes data + 12 bytes params = 44 < 128)
        assert!(quantized.memory_bytes() < 32 * 4);

        // Check values are approximately correct
        for (orig, rest) in vector.iter().zip(restored.iter()) {
            assert!((orig - rest).abs() < 0.02, "Dequantization error too large");
        }
    }

    #[test]
    fn test_binary_quantization() {
        let quantizer = Quantizer::binary(128);
        let vector: Vec<f32> = (0..128)
            .map(|i| if i % 2 == 0 { 0.5 } else { -0.5 })
            .collect();

        let quantized = quantizer.quantize(&vector).unwrap();

        match &quantized {
            QuantizedVector::Binary { data, dimensions } => {
                assert_eq!(*dimensions, 128);
                assert_eq!(data.len(), 2); // 128 bits = 2 u64s
            }
            _ => panic!("Expected binary quantization"),
        }

        // Memory should be much smaller (2 * 8 = 16 bytes vs 128 * 4 = 512 bytes)
        assert!(quantized.memory_bytes() < 128);
    }

    #[test]
    fn test_cosine_distance_int8() {
        let a = vec![100i8, 50, 25, 0];
        let b = vec![100i8, 50, 25, 0]; // Same vector

        let dist = cosine_distance_int8(&a, &b);
        assert!(dist < 0.01); // Should be very close to 0
    }

    #[test]
    fn test_hamming_distance() {
        let a = vec![0b1010101010u64];
        let b = vec![0b0101010101u64];

        let dist = hamming_distance_binary(&a, &b);
        assert_eq!(dist, 10); // All 10 bits differ
    }

    #[test]
    fn test_trained_quantizer() {
        let samples: Vec<Vec<f32>> = vec![
            vec![0.1, 0.2, 0.3],
            vec![-0.5, 0.8, 0.0],
            vec![0.9, -0.9, 0.5],
        ];
        let sample_refs: Vec<&[f32]> = samples.iter().map(|v| v.as_slice()).collect();

        let quantizer = Quantizer::int8_trained(3, &sample_refs);

        // Should have learned min/max from samples
        let params = quantizer.scalar_params.as_ref().unwrap();
        assert!(params.min_val <= -0.5);
        assert!(params.max_val >= 0.9);
    }
}
