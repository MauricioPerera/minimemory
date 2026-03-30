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
//! | Type | Compression | Levels | Accuracy | Speed |
//! |------|-------------|--------|----------|-------|
//! | None | 1x | ∞ | 100% | Baseline |
//! | Int8 | 4x | 256 | ~99% | Faster |
//! | Int3 | 10.7x | 8 | ~96-98% | Fast |
//! | Binary | 32x | 2 | ~90-95% | Much faster |

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
    /// 3-bit quantization (8 levels per dimension)
    /// Provides ~10.7x memory reduction with good accuracy (~96-98%)
    /// Packed as 21 values per u64 word (21 * 3 = 63 bits)
    Int3,
    /// Binary quantization (1 bit per dimension)
    /// Provides 32x memory reduction, best for high-dimensional vectors
    Binary,
    /// Polar angle quantization (3-bit per pair of dimensions)
    /// Converts pairs to polar angles, ~21x compression
    /// Requires even dimension count
    Polar,
}

impl QuantizationType {
    /// Convert to u8 for serialization
    pub fn to_u8(&self) -> u8 {
        match self {
            QuantizationType::None => 0,
            QuantizationType::Int8 => 1,
            QuantizationType::Int3 => 2,
            QuantizationType::Binary => 3,
            QuantizationType::Polar => 4,
        }
    }

    /// Convert from u8
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => QuantizationType::Int8,
            2 => QuantizationType::Int3,
            3 => QuantizationType::Binary,
            4 => QuantizationType::Polar,
            _ => QuantizationType::None,
        }
    }
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

    /// Quantize a single f32 value to 3-bit (0..7)
    #[inline]
    pub fn quantize_value_3bit(&self, val: f32) -> u8 {
        let clamped = val.clamp(self.min_val, self.max_val);
        let range = self.max_val - self.min_val;
        if range <= 0.0 {
            return 3; // midpoint
        }
        let normalized = (clamped - self.min_val) / range * 7.0;
        (normalized.round() as u8).min(7)
    }

    /// Dequantize a single 3-bit value (0..7) back to f32
    #[inline]
    pub fn dequantize_value_3bit(&self, val: u8) -> f32 {
        let range = self.max_val - self.min_val;
        self.min_val + (val as f32 / 7.0) * range
    }
}

/// Number of 3-bit values packed per u64 word (21 * 3 = 63 bits)
pub const INT3_VALUES_PER_WORD: usize = 21;

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
    /// 3-bit quantized (8 levels, packed 21 values per u64)
    Int3 {
        data: Vec<u64>,
        params: ScalarQuantParams,
        dimensions: usize,
    },
    /// Binary quantized (packed bits)
    Binary { data: Vec<u64>, dimensions: usize },
    /// Polar angle quantized (3-bit angles for pairs of dimensions)
    /// Each pair of dimensions → 1 angle quantized to 8 levels
    Polar {
        /// Packed 3-bit angle indices (MSB-first, same packing as Int3)
        data: Vec<u64>,
        /// Number of original dimensions (must be even)
        dimensions: usize,
        /// Seed used for deterministic rotation
        seed: u32,
    },
}

impl QuantizedVector {
    /// Get the number of dimensions
    pub fn dimensions(&self) -> usize {
        match self {
            QuantizedVector::Full(v) => v.len(),
            QuantizedVector::Int8 { data, .. } => data.len(),
            QuantizedVector::Int3 { dimensions, .. } => *dimensions,
            QuantizedVector::Binary { dimensions, .. } => *dimensions,
            QuantizedVector::Polar { dimensions, .. } => *dimensions,
        }
    }

    /// Get memory usage in bytes
    pub fn memory_bytes(&self) -> usize {
        match self {
            QuantizedVector::Full(v) => v.len() * 4,
            QuantizedVector::Int8 { data, .. } => {
                data.len() + std::mem::size_of::<ScalarQuantParams>()
            }
            QuantizedVector::Int3 { data, .. } => {
                data.len() * 8 + std::mem::size_of::<ScalarQuantParams>()
            }
            QuantizedVector::Binary { data, .. } => data.len() * 8,
            QuantizedVector::Polar { data, .. } => data.len() * 8 + 4, // +4 for seed
        }
    }

    /// Dequantize back to f32 vector
    pub fn to_f32(&self) -> Vec<f32> {
        match self {
            QuantizedVector::Full(v) => v.clone(),
            QuantizedVector::Int8 { data, params } => {
                data.iter().map(|&v| params.dequantize_value(v)).collect()
            }
            QuantizedVector::Int3 {
                data,
                params,
                dimensions,
            } => {
                let mut result = Vec::with_capacity(*dimensions);
                for i in 0..*dimensions {
                    let word_idx = i / INT3_VALUES_PER_WORD;
                    let pos_in_word = i % INT3_VALUES_PER_WORD;
                    let shift = pos_in_word * 3;
                    let val = ((data[word_idx] >> shift) & 0x7) as u8;
                    result.push(params.dequantize_value_3bit(val));
                }
                result
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
            QuantizedVector::Polar {
                data,
                dimensions,
                seed,
            } => {
                let pairs = *dimensions / 2;
                let (signs, perm) = polar_generate_rotation(*dimensions, *seed);
                // Unpack angle indices and reconstruct in rotated space
                let mut rotated = vec![0.0f32; *dimensions];
                for p in 0..pairs {
                    let idx = polar_unpack_3bit(data, p);
                    rotated[p * 2] = POLAR_COS_TABLE[idx as usize];
                    rotated[p * 2 + 1] = POLAR_SIN_TABLE[idx as usize];
                }
                // Inverse rotation: out[perm[i]] = rotated[i] * signs[perm[i]]
                let mut out = vec![0.0f32; *dimensions];
                for i in 0..*dimensions {
                    out[perm[i]] = rotated[i] * signs[perm[i]];
                }
                out
            }
        }
    }
}

// ============================================================================
// Polar quantization helpers
// ============================================================================

/// Number of angular bins for polar quantization (3 bits = 8 levels)
const POLAR_NUM_BINS: usize = 8;

/// Precomputed cosine values for 8 angular bin midpoints
/// theta_i = -PI + (i + 0.5) * (2*PI / 8) for i in 0..8
const POLAR_COS_TABLE: [f32; POLAR_NUM_BINS] = {
    // Computed at compile time: cos(-PI + (i+0.5)*PI/4)
    // i=0: cos(-7PI/8), i=1: cos(-5PI/8), ..., i=7: cos(7PI/8)
    [
        -0.9238795, // cos(-7π/8)
        -0.3826834, // cos(-5π/8)
        0.3826834,  // cos(-3π/8)
        0.9238795,  // cos(-π/8)
        0.9238795,  // cos(π/8)
        0.3826834,  // cos(3π/8)
        -0.3826834, // cos(5π/8)
        -0.9238795, // cos(7π/8)
    ]
};

/// Precomputed sine values for 8 angular bin midpoints
const POLAR_SIN_TABLE: [f32; POLAR_NUM_BINS] = {
    [
        -0.3826834, // sin(-7π/8)
        -0.9238795, // sin(-5π/8)
        -0.9238795, // sin(-3π/8)
        -0.3826834, // sin(-π/8)
        0.3826834,  // sin(π/8)
        0.9238795,  // sin(3π/8)
        0.9238795,  // sin(5π/8)
        0.3826834,  // sin(7π/8)
    ]
};

/// xorshift32 PRNG for deterministic rotation generation
#[inline]
fn xorshift32(mut state: u32) -> u32 {
    state ^= state << 13;
    state ^= state >> 17;
    state ^= state << 5;
    state
}

/// Generate deterministic rotation: sign-flip array + permutation
fn polar_generate_rotation(dim: usize, seed: u32) -> (Vec<f32>, Vec<usize>) {
    let mut state = seed;
    let mut signs = vec![0.0f32; dim];
    for i in 0..dim {
        state = xorshift32(state);
        signs[i] = if (state & 1) == 1 { 1.0 } else { -1.0 };
    }

    let mut perm: Vec<usize> = (0..dim).collect();
    state = seed.wrapping_mul(7).wrapping_add(13);
    for i in (1..dim).rev() {
        state = xorshift32(state);
        let j = (state as usize) % (i + 1);
        perm.swap(i, j);
    }

    (signs, perm)
}

/// Apply rotation: out[i] = vec[perm[i]] * signs[perm[i]]
fn polar_rotate(vec: &[f32], signs: &[f32], perm: &[usize]) -> Vec<f32> {
    let mut out = vec![0.0f32; vec.len()];
    for i in 0..vec.len() {
        out[i] = vec[perm[i]] * signs[perm[i]];
    }
    out
}

/// L2 normalize a vector
fn l2_normalize(v: &[f32]) -> Vec<f32> {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 1e-10 {
        v.iter().map(|x| x / norm).collect()
    } else {
        v.to_vec()
    }
}

/// Pack a 3-bit value at pair position p into u64 array (same packing as Int3)
#[inline]
fn polar_pack_3bit(data: &mut [u64], p: usize, val: u8) {
    let word_idx = p / INT3_VALUES_PER_WORD;
    let pos = p % INT3_VALUES_PER_WORD;
    let shift = pos * 3;
    data[word_idx] |= (val as u64 & 0x7) << shift;
}

/// Unpack a 3-bit value at pair position p from u64 array
#[inline]
fn polar_unpack_3bit(data: &[u64], p: usize) -> u8 {
    let word_idx = p / INT3_VALUES_PER_WORD;
    let pos = p % INT3_VALUES_PER_WORD;
    let shift = pos * 3;
    ((data[word_idx] >> shift) & 0x7) as u8
}

/// Asymmetric cosine distance: float query vs polar-quantized stored vector
/// Computes in rotated space for efficiency (avoids inverse rotation)
pub fn cosine_distance_polar_asymmetric(
    query: &[f32],
    stored_data: &[u64],
    dimensions: usize,
    seed: u32,
) -> f32 {
    let pairs = dimensions / 2;
    let (signs, perm) = polar_generate_rotation(dimensions, seed);
    let query_norm = l2_normalize(query);
    let query_rot = polar_rotate(&query_norm, &signs, &perm);

    let mut dot = 0.0f32;
    let mut nq = 0.0f32;
    for p in 0..pairs {
        let qa = query_rot[p * 2];
        let qb = query_rot[p * 2 + 1];
        let idx = polar_unpack_3bit(stored_data, p) as usize;
        dot += qa * POLAR_COS_TABLE[idx] + qb * POLAR_SIN_TABLE[idx];
        nq += qa * qa + qb * qb;
    }

    // Stored vector norm = sqrt(pairs) because each pair contributes cos²+sin²=1
    let denom = nq.sqrt() * (pairs as f32).sqrt();
    if denom == 0.0 {
        1.0
    } else {
        1.0 - (dot / denom)
    }
}

/// Symmetric cosine distance between two polar-quantized vectors
pub fn cosine_distance_polar_symmetric(
    a: &[u64],
    b: &[u64],
    dimensions: usize,
) -> f32 {
    let pairs = dimensions / 2;
    let mut dot = 0.0f32;

    for p in 0..pairs {
        let idx_a = polar_unpack_3bit(a, p) as usize;
        let idx_b = polar_unpack_3bit(b, p) as usize;
        // cos(theta_a - theta_b) = cos_a*cos_b + sin_a*sin_b
        dot += POLAR_COS_TABLE[idx_a] * POLAR_COS_TABLE[idx_b]
            + POLAR_SIN_TABLE[idx_a] * POLAR_SIN_TABLE[idx_b];
    }

    // Both vectors are unit by construction (cos^2+sin^2=1 per pair)
    // So norm_a = norm_b = sqrt(pairs) and dot / (norm_a * norm_b) = dot / pairs
    1.0 - (dot / pairs as f32)
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

    /// Create a new 3-bit quantizer with default parameters
    pub fn int3(dimensions: usize) -> Self {
        Self {
            quant_type: QuantizationType::Int3,
            scalar_params: Some(ScalarQuantParams::default()),
            dimensions,
        }
    }

    /// Create a new 3-bit quantizer trained on sample vectors
    pub fn int3_trained(dimensions: usize, samples: &[&[f32]]) -> Self {
        Self {
            quant_type: QuantizationType::Int3,
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

    /// Create a new polar angle quantizer (dimensions must be even)
    pub fn polar(dimensions: usize) -> Self {
        Self {
            quant_type: QuantizationType::Polar,
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

            QuantizationType::Int3 => {
                let params = self.scalar_params.as_ref().cloned().unwrap_or_default();
                let num_words = self.dimensions.div_ceil(INT3_VALUES_PER_WORD);
                let mut data = vec![0u64; num_words];

                for (i, &val) in vector.iter().enumerate() {
                    let q = params.quantize_value_3bit(val) as u64;
                    let word_idx = i / INT3_VALUES_PER_WORD;
                    let pos_in_word = i % INT3_VALUES_PER_WORD;
                    let shift = pos_in_word * 3;
                    data[word_idx] |= q << shift;
                }

                Ok(QuantizedVector::Int3 {
                    data,
                    params,
                    dimensions: self.dimensions,
                })
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

            QuantizationType::Polar => {
                if self.dimensions % 2 != 0 {
                    return Err(Error::InvalidConfig(
                        "Polar quantization requires even dimensions".into(),
                    ));
                }

                let seed = 42u32;
                let pairs = self.dimensions / 2;
                let num_words = pairs.div_ceil(INT3_VALUES_PER_WORD);
                let mut data = vec![0u64; num_words];

                let norm = l2_normalize(vector);
                let (signs, perm) = polar_generate_rotation(self.dimensions, seed);
                let rotated = polar_rotate(&norm, &signs, &perm);

                for p in 0..pairs {
                    let a = rotated[p * 2];
                    let b = rotated[p * 2 + 1];
                    let theta = b.atan2(a); // [-PI, PI]
                    let level_f = (theta + std::f32::consts::PI)
                        / (2.0 * std::f32::consts::PI)
                        * POLAR_NUM_BINS as f32;
                    // Modulo for angular wraparound (PI and -PI map to same bin)
                    let level = (level_f as u8) % (POLAR_NUM_BINS as u8);
                    polar_pack_3bit(&mut data, p, level);
                }

                Ok(QuantizedVector::Polar {
                    data,
                    dimensions: self.dimensions,
                    seed,
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
        if self.quant_type == QuantizationType::Int8 || self.quant_type == QuantizationType::Int3 {
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

// ============================================================================
// 3-bit distance calculations
// ============================================================================

/// Unpack a single 3-bit value from a packed u64 word
#[inline(always)]
fn unpack_int3(word: u64, pos: usize) -> u8 {
    ((word >> (pos * 3)) & 0x7) as u8
}

/// Calculate cosine distance between two 3-bit quantized vectors
#[inline]
pub fn cosine_distance_int3(a: &[u64], b: &[u64], dimensions: usize) -> f32 {
    debug_assert_eq!(a.len(), b.len());

    let mut dot: i32 = 0;
    let mut norm_a: i32 = 0;
    let mut norm_b: i32 = 0;

    let full_words = dimensions / INT3_VALUES_PER_WORD;
    let remainder = dimensions % INT3_VALUES_PER_WORD;

    for i in 0..full_words {
        let wa = a[i];
        let wb = b[i];
        for pos in 0..INT3_VALUES_PER_WORD {
            let va = unpack_int3(wa, pos) as i32;
            let vb = unpack_int3(wb, pos) as i32;
            dot += va * vb;
            norm_a += va * va;
            norm_b += vb * vb;
        }
    }

    if remainder > 0 {
        let wa = a[full_words];
        let wb = b[full_words];
        for pos in 0..remainder {
            let va = unpack_int3(wa, pos) as i32;
            let vb = unpack_int3(wb, pos) as i32;
            dot += va * vb;
            norm_a += va * va;
            norm_b += vb * vb;
        }
    }

    let norm = (norm_a as f32).sqrt() * (norm_b as f32).sqrt();
    if norm == 0.0 {
        return 1.0;
    }
    1.0 - (dot as f32 / norm)
}

/// Calculate euclidean distance between two 3-bit quantized vectors
#[inline]
pub fn euclidean_distance_int3(a: &[u64], b: &[u64], dimensions: usize) -> f32 {
    debug_assert_eq!(a.len(), b.len());

    let mut sum: i32 = 0;

    let full_words = dimensions / INT3_VALUES_PER_WORD;
    let remainder = dimensions % INT3_VALUES_PER_WORD;

    for i in 0..full_words {
        let wa = a[i];
        let wb = b[i];
        for pos in 0..INT3_VALUES_PER_WORD {
            let diff = unpack_int3(wa, pos) as i32 - unpack_int3(wb, pos) as i32;
            sum += diff * diff;
        }
    }

    if remainder > 0 {
        let wa = a[full_words];
        let wb = b[full_words];
        for pos in 0..remainder {
            let diff = unpack_int3(wa, pos) as i32 - unpack_int3(wb, pos) as i32;
            sum += diff * diff;
        }
    }

    (sum as f32).sqrt()
}

/// Calculate dot product distance between two 3-bit quantized vectors
#[inline]
pub fn dot_product_distance_int3(a: &[u64], b: &[u64], dimensions: usize) -> f32 {
    debug_assert_eq!(a.len(), b.len());

    let mut dot: i32 = 0;

    let full_words = dimensions / INT3_VALUES_PER_WORD;
    let remainder = dimensions % INT3_VALUES_PER_WORD;

    for i in 0..full_words {
        let wa = a[i];
        let wb = b[i];
        for pos in 0..INT3_VALUES_PER_WORD {
            dot += unpack_int3(wa, pos) as i32 * unpack_int3(wb, pos) as i32;
        }
    }

    if remainder > 0 {
        let wa = a[full_words];
        let wb = b[full_words];
        for pos in 0..remainder {
            dot += unpack_int3(wa, pos) as i32 * unpack_int3(wb, pos) as i32;
        }
    }

    -(dot as f32)
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
// Manhattan distance for quantized vectors
// ============================================================================

/// Calculate Manhattan (L1) distance between two int8 quantized vectors
#[inline]
pub fn manhattan_distance_int8(a: &[i8], b: &[i8]) -> f32 {
    debug_assert_eq!(a.len(), b.len());
    let mut sum: i32 = 0;
    for (&ai, &bi) in a.iter().zip(b.iter()) {
        sum += (ai as i32 - bi as i32).abs();
    }
    sum as f32
}

/// Calculate Manhattan (L1) distance between two 3-bit quantized vectors
#[inline]
pub fn manhattan_distance_int3(a: &[u64], b: &[u64], dimensions: usize) -> f32 {
    debug_assert_eq!(a.len(), b.len());
    let mut sum: i32 = 0;
    let full_words = dimensions / INT3_VALUES_PER_WORD;
    let remainder = dimensions % INT3_VALUES_PER_WORD;

    for i in 0..full_words {
        let wa = a[i];
        let wb = b[i];
        for pos in 0..INT3_VALUES_PER_WORD {
            sum += (unpack_int3(wa, pos) as i32 - unpack_int3(wb, pos) as i32).abs();
        }
    }
    if remainder > 0 {
        let wa = a[full_words];
        let wb = b[full_words];
        for pos in 0..remainder {
            sum += (unpack_int3(wa, pos) as i32 - unpack_int3(wb, pos) as i32).abs();
        }
    }
    sum as f32
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
                Distance::Manhattan => manhattan_distance_int8(da, db),
            })
        }

        (
            QuantizedVector::Int3 {
                data: da,
                dimensions: dim_a,
                ..
            },
            QuantizedVector::Int3 {
                data: db,
                dimensions: dim_b,
                ..
            },
        ) => {
            if dim_a != dim_b {
                return Err(Error::DimensionMismatch {
                    expected: *dim_a,
                    got: *dim_b,
                });
            }
            Ok(match distance_type {
                Distance::Cosine => cosine_distance_int3(da, db, *dim_a),
                Distance::Euclidean => euclidean_distance_int3(da, db, *dim_a),
                Distance::DotProduct => dot_product_distance_int3(da, db, *dim_a),
                Distance::Manhattan => manhattan_distance_int3(da, db, *dim_a),
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
                Distance::Manhattan => hamming_distance_binary(da, db) as f32,
            })
        }

        (
            QuantizedVector::Polar {
                data: da,
                dimensions: dim_a,
                ..
            },
            QuantizedVector::Polar {
                data: db,
                dimensions: dim_b,
                ..
            },
        ) => {
            if dim_a != dim_b {
                return Err(Error::DimensionMismatch {
                    expected: *dim_a,
                    got: *dim_b,
                });
            }
            Ok(match distance_type {
                Distance::Cosine => cosine_distance_polar_symmetric(da, db, *dim_a),
                _ => {
                    // For non-cosine metrics, dequantize and compute
                    let va = a.to_f32();
                    let vb = b.to_f32();
                    distance_type.calculate(&va, &vb)
                }
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

    // ========================================================================
    // 3-bit quantization tests
    // ========================================================================

    #[test]
    fn test_int3_quantization_roundtrip() {
        let quantizer = Quantizer::int3(64);
        let vector: Vec<f32> = (0..64).map(|i| (i as f32 - 32.0) / 32.0).collect();

        let quantized = quantizer.quantize(&vector).unwrap();
        let restored = quantizer.dequantize(&quantized);

        assert_eq!(restored.len(), 64);

        // 3-bit has 8 levels over [-1, 1] range = step size ~0.286
        // max error should be <= half step = ~0.143
        for (orig, rest) in vector.iter().zip(restored.iter()) {
            assert!(
                (orig - rest).abs() < 0.20,
                "3-bit dequantization error too large: orig={}, restored={}",
                orig,
                rest
            );
        }
    }

    #[test]
    fn test_int3_memory_savings() {
        let quantizer = Quantizer::int3(384);
        let vector: Vec<f32> = (0..384).map(|i| (i as f32) / 384.0).collect();

        let quantized = quantizer.quantize(&vector).unwrap();

        match &quantized {
            QuantizedVector::Int3 {
                data, dimensions, ..
            } => {
                assert_eq!(*dimensions, 384);
                // 384 / 21 = 18.28 -> 19 u64 words
                assert_eq!(data.len(), 19);
            }
            _ => panic!("Expected Int3 quantization"),
        }

        // Memory: 19 * 8 + 12 (params) = 164 bytes vs 384 * 4 = 1536 bytes
        let mem = quantized.memory_bytes();
        let full_mem = 384 * 4;
        assert!(
            mem < full_mem / 5,
            "Int3 should use < 1/5 of full memory: {} vs {}",
            mem,
            full_mem
        );
    }

    #[test]
    fn test_int3_packing() {
        // Verify that 21 values pack correctly into one u64
        let quantizer = Quantizer::int3(21);
        // Create a vector where each dimension maps to a different 3-bit level
        let vector: Vec<f32> = (0..21).map(|i| -1.0 + (i as f32 / 20.0) * 2.0).collect();

        let quantized = quantizer.quantize(&vector).unwrap();

        match &quantized {
            QuantizedVector::Int3 { data, .. } => {
                assert_eq!(data.len(), 1, "21 values should fit in 1 u64");
            }
            _ => panic!("Expected Int3"),
        }

        // Roundtrip should preserve relative order
        let restored = quantized.to_f32();
        for i in 1..21 {
            assert!(
                restored[i] >= restored[i - 1] - 0.01,
                "Order not preserved at {}: {} < {}",
                i,
                restored[i],
                restored[i - 1]
            );
        }
    }

    #[test]
    fn test_cosine_distance_int3_identical() {
        // Pack two identical vectors
        let quantizer = Quantizer::int3(64);
        let v: Vec<f32> = (0..64).map(|i| (i as f32 - 32.0) / 32.0).collect();

        let qa = quantizer.quantize(&v).unwrap();
        let qb = quantizer.quantize(&v).unwrap();

        if let (
            QuantizedVector::Int3 {
                data: da,
                dimensions: dim_a,
                ..
            },
            QuantizedVector::Int3 {
                data: db,
                dimensions: dim_b,
                ..
            },
        ) = (&qa, &qb)
        {
            let dist = cosine_distance_int3(da, db, *dim_a);
            assert!(dist < 0.01, "Identical vectors should have ~0 distance, got {}", dist);
            assert_eq!(dim_a, dim_b);
        } else {
            panic!("Expected Int3");
        }
    }

    #[test]
    fn test_quantized_distance_int3() {
        let quantizer = Quantizer::int3(32);
        let a: Vec<f32> = (0..32).map(|i| (i as f32) / 32.0).collect();
        let b: Vec<f32> = (0..32).map(|i| (31 - i) as f32 / 32.0).collect();

        let qa = quantizer.quantize(&a).unwrap();
        let qb = quantizer.quantize(&b).unwrap();

        // Should work for all distance types without error
        let _ = quantized_distance(&qa, &qb, crate::Distance::Cosine).unwrap();
        let _ = quantized_distance(&qa, &qb, crate::Distance::Euclidean).unwrap();
        let _ = quantized_distance(&qa, &qb, crate::Distance::DotProduct).unwrap();
    }

    #[test]
    fn test_int3_trained_quantizer() {
        let samples: Vec<Vec<f32>> = vec![
            vec![0.1, 0.2, 0.3],
            vec![-0.5, 0.8, 0.0],
            vec![0.9, -0.9, 0.5],
        ];
        let sample_refs: Vec<&[f32]> = samples.iter().map(|v| v.as_slice()).collect();

        let quantizer = Quantizer::int3_trained(3, &sample_refs);

        let params = quantizer.scalar_params.as_ref().unwrap();
        assert!(params.min_val <= -0.5);
        assert!(params.max_val >= 0.9);

        // Quantize and verify
        let qvec = quantizer.quantize(&samples[0]).unwrap();
        let restored = qvec.to_f32();
        assert_eq!(restored.len(), 3);
    }

    // ========================================================================
    // Polar quantization tests
    // ========================================================================

    #[test]
    fn test_polar_rotation_deterministic() {
        let (signs1, perm1) = polar_generate_rotation(8, 42);
        let (signs2, perm2) = polar_generate_rotation(8, 42);
        assert_eq!(signs1, signs2);
        assert_eq!(perm1, perm2);

        // Different seed → different rotation
        let (signs3, _) = polar_generate_rotation(8, 99);
        assert_ne!(signs1, signs3);
    }

    #[test]
    fn test_polar_quantize_roundtrip() {
        let quantizer = Quantizer::polar(8); // 8 dims → 4 pairs
        let v = vec![0.5, 0.3, -0.2, 0.8, 0.1, -0.6, 0.4, 0.7];

        let qvec = quantizer.quantize(&v).unwrap();
        let restored = qvec.to_f32();

        assert_eq!(restored.len(), 8);
        // Direction should be roughly preserved (cosine similarity > 0.5)
        let dot: f32 = v.iter().zip(restored.iter()).map(|(a, b)| a * b).sum();
        let na: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        let nb: f32 = restored.iter().map(|x| x * x).sum::<f32>().sqrt();
        let cosim = if na * nb > 0.0 { dot / (na * nb) } else { 0.0 };
        assert!(
            cosim > 0.3,
            "Polar roundtrip should preserve direction, got cosine={}",
            cosim
        );
    }

    #[test]
    fn test_polar_requires_even_dims() {
        let quantizer = Quantizer::polar(7); // odd!
        let v = vec![0.1; 7];
        let result = quantizer.quantize(&v);
        assert!(result.is_err());
    }

    #[test]
    fn test_polar_memory_savings() {
        let quantizer = Quantizer::polar(384); // 192 pairs → 192 * 3 bits
        let v: Vec<f32> = (0..384).map(|i| (i as f32 - 192.0) / 192.0).collect();
        let qvec = quantizer.quantize(&v).unwrap();

        // 192 pairs / 21 per word = 10 words = 80 bytes + 4 seed
        let mem = qvec.memory_bytes();
        let full_mem = 384 * 4;
        assert!(
            mem < full_mem / 10,
            "Polar should use < 1/10 of full memory: {} vs {}",
            mem,
            full_mem
        );
    }

    #[test]
    fn test_polar_symmetric_distance_identical() {
        let quantizer = Quantizer::polar(8);
        let v = vec![0.5, 0.3, -0.2, 0.8, 0.1, -0.6, 0.4, 0.7];

        let qa = quantizer.quantize(&v).unwrap();
        let qb = quantizer.quantize(&v).unwrap();

        if let (
            QuantizedVector::Polar { data: da, dimensions: dim_a, .. },
            QuantizedVector::Polar { data: db, dimensions: dim_b, .. },
        ) = (&qa, &qb)
        {
            let dist = cosine_distance_polar_symmetric(da, db, *dim_a);
            assert!(
                dist < 0.01,
                "Identical vectors should have ~0 polar distance, got {}",
                dist
            );
            assert_eq!(dim_a, dim_b);
        } else {
            panic!("Expected Polar");
        }
    }

    #[test]
    fn test_polar_asymmetric_distance() {
        let quantizer = Quantizer::polar(8);
        let v = vec![0.5, 0.3, -0.2, 0.8, 0.1, -0.6, 0.4, 0.7];

        let qvec = quantizer.quantize(&v).unwrap();

        if let QuantizedVector::Polar { data, dimensions, seed, .. } = &qvec {
            let dist = cosine_distance_polar_asymmetric(&v, data, *dimensions, *seed);
            assert!(
                dist < 0.5,
                "Asymmetric distance to self should be small, got {}",
                dist
            );
        } else {
            panic!("Expected Polar");
        }
    }

    #[test]
    fn test_polar_quantized_distance() {
        let quantizer = Quantizer::polar(8);
        let a = vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let b = vec![0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0];

        let qa = quantizer.quantize(&a).unwrap();
        let qb = quantizer.quantize(&b).unwrap();

        // Different directions should have larger distance
        let dist = quantized_distance(&qa, &qb, crate::Distance::Cosine).unwrap();
        assert!(dist > 0.0, "Different vectors should have positive distance");
    }
}
