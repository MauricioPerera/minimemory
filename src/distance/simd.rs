//! Implementaciones SIMD optimizadas para cálculos de distancia.
//!
//! Proporciona versiones aceleradas de las métricas de distancia
//! usando instrucciones SIMD cuando están disponibles:
//! - x86_64: AVX-512, AVX2+FMA, SSE
//! - aarch64: NEON (Apple Silicon, ARM servers)
//!
//! El código selecciona automáticamente la mejor implementación disponible
//! en tiempo de ejecución.

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

#[cfg(target_arch = "aarch64")]
use std::arch::aarch64::*;

// ============================================================================
// Public API - Selección automática de implementación
// ============================================================================

/// Calcula la distancia euclidiana usando SIMD si está disponible.
#[inline]
pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    #[cfg(target_arch = "x86_64")]
    {
        // AVX-512 (16 floats por operación)
        #[cfg(target_feature = "avx512f")]
        {
            if is_x86_feature_detected!("avx512f") && a.len() >= 16 {
                return unsafe { euclidean_avx512(a, b) };
            }
        }

        // AVX2 (8 floats por operación)
        #[cfg(target_feature = "avx2")]
        {
            if is_x86_feature_detected!("avx2") && a.len() >= 8 {
                return unsafe { euclidean_avx2(a, b) };
            }
        }

        // SSE (4 floats por operación)
        #[cfg(target_feature = "sse")]
        {
            if is_x86_feature_detected!("sse") && a.len() >= 4 {
                return unsafe { euclidean_sse(a, b) };
            }
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        if a.len() >= 4 {
            return unsafe { euclidean_neon(a, b) };
        }
    }

    euclidean_scalar(a, b)
}

/// Calcula la distancia coseno usando SIMD si está disponible.
#[inline]
pub fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
    #[cfg(target_arch = "x86_64")]
    {
        #[cfg(target_feature = "avx512f")]
        {
            if is_x86_feature_detected!("avx512f") && a.len() >= 16 {
                return unsafe { cosine_avx512(a, b) };
            }
        }

        #[cfg(target_feature = "avx2")]
        {
            if is_x86_feature_detected!("avx2") && a.len() >= 8 {
                return unsafe { cosine_avx2(a, b) };
            }
        }

        #[cfg(target_feature = "sse")]
        {
            if is_x86_feature_detected!("sse") && a.len() >= 4 {
                return unsafe { cosine_sse(a, b) };
            }
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        if a.len() >= 4 {
            return unsafe { cosine_neon(a, b) };
        }
    }

    cosine_scalar(a, b)
}

/// Calcula el producto punto (negativo) usando SIMD si está disponible.
#[inline]
pub fn dot_product_distance(a: &[f32], b: &[f32]) -> f32 {
    #[cfg(target_arch = "x86_64")]
    {
        #[cfg(target_feature = "avx512f")]
        {
            if is_x86_feature_detected!("avx512f") && a.len() >= 16 {
                return unsafe { -dot_avx512(a, b) };
            }
        }

        #[cfg(target_feature = "avx2")]
        {
            if is_x86_feature_detected!("avx2") && a.len() >= 8 {
                return unsafe { -dot_avx2(a, b) };
            }
        }

        #[cfg(target_feature = "sse")]
        {
            if is_x86_feature_detected!("sse") && a.len() >= 4 {
                return unsafe { -dot_sse(a, b) };
            }
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        if a.len() >= 4 {
            return unsafe { -dot_neon(a, b) };
        }
    }

    dot_scalar(a, b)
}

/// Calcula la distancia Manhattan (L1) usando SIMD si está disponible.
#[inline]
pub fn manhattan_distance(a: &[f32], b: &[f32]) -> f32 {
    // Scalar fallback for all architectures (Manhattan is simple enough
    // that the compiler auto-vectorizes well with -C target-cpu=native)
    manhattan_scalar(a, b)
}

// ============================================================================
// Implementaciones escalares (fallback universal)
// ============================================================================

#[inline]
fn euclidean_scalar(a: &[f32], b: &[f32]) -> f32 {
    let mut sum = 0.0f32;
    let chunks = a.len() / 4;
    let remainder = a.len() % 4;

    // Loop unrolling para mejor pipeline
    for i in 0..chunks {
        let base = i * 4;
        let d0 = a[base] - b[base];
        let d1 = a[base + 1] - b[base + 1];
        let d2 = a[base + 2] - b[base + 2];
        let d3 = a[base + 3] - b[base + 3];
        sum += d0 * d0 + d1 * d1 + d2 * d2 + d3 * d3;
    }

    let base = chunks * 4;
    for i in 0..remainder {
        let d = a[base + i] - b[base + i];
        sum += d * d;
    }

    sum.sqrt()
}

#[inline]
fn cosine_scalar(a: &[f32], b: &[f32]) -> f32 {
    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;

    for i in 0..a.len() {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }

    let denom = (norm_a * norm_b).sqrt();
    if denom == 0.0 {
        return 1.0;
    }

    1.0 - (dot / denom)
}

#[inline]
fn dot_scalar(a: &[f32], b: &[f32]) -> f32 {
    let mut dot = 0.0f32;
    for i in 0..a.len() {
        dot += a[i] * b[i];
    }
    -dot
}

#[inline]
fn manhattan_scalar(a: &[f32], b: &[f32]) -> f32 {
    let mut sum = 0.0f32;
    let chunks = a.len() / 4;
    let remainder = a.len() % 4;

    for i in 0..chunks {
        let base = i * 4;
        sum += (a[base] - b[base]).abs()
            + (a[base + 1] - b[base + 1]).abs()
            + (a[base + 2] - b[base + 2]).abs()
            + (a[base + 3] - b[base + 3]).abs();
    }

    let base = chunks * 4;
    for i in 0..remainder {
        sum += (a[base + i] - b[base + i]).abs();
    }

    sum
}

// ============================================================================
// ARM NEON Implementations (aarch64 - Apple Silicon, ARM servers)
// ============================================================================

#[cfg(target_arch = "aarch64")]
#[inline]
unsafe fn euclidean_neon(a: &[f32], b: &[f32]) -> f32 {
    let mut sum = vdupq_n_f32(0.0);
    let chunks = a.len() / 4;

    for i in 0..chunks {
        let offset = i * 4;
        let va = vld1q_f32(a.as_ptr().add(offset));
        let vb = vld1q_f32(b.as_ptr().add(offset));
        let diff = vsubq_f32(va, vb);
        sum = vfmaq_f32(sum, diff, diff); // FMA: sum += diff * diff
    }

    // Reducción horizontal NEON
    let mut result = vaddvq_f32(sum);

    // Procesar elementos restantes
    let remainder_start = chunks * 4;
    for i in remainder_start..a.len() {
        let d = a[i] - b[i];
        result += d * d;
    }

    result.sqrt()
}

#[cfg(target_arch = "aarch64")]
#[inline]
unsafe fn cosine_neon(a: &[f32], b: &[f32]) -> f32 {
    let mut dot_sum = vdupq_n_f32(0.0);
    let mut norm_a_sum = vdupq_n_f32(0.0);
    let mut norm_b_sum = vdupq_n_f32(0.0);

    let chunks = a.len() / 4;

    for i in 0..chunks {
        let offset = i * 4;
        let va = vld1q_f32(a.as_ptr().add(offset));
        let vb = vld1q_f32(b.as_ptr().add(offset));

        dot_sum = vfmaq_f32(dot_sum, va, vb);
        norm_a_sum = vfmaq_f32(norm_a_sum, va, va);
        norm_b_sum = vfmaq_f32(norm_b_sum, vb, vb);
    }

    let mut dot = vaddvq_f32(dot_sum);
    let mut norm_a = vaddvq_f32(norm_a_sum);
    let mut norm_b = vaddvq_f32(norm_b_sum);

    // Procesar elementos restantes
    let remainder_start = chunks * 4;
    for i in remainder_start..a.len() {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }

    let denom = (norm_a * norm_b).sqrt();
    if denom == 0.0 {
        return 1.0;
    }

    1.0 - (dot / denom)
}

#[cfg(target_arch = "aarch64")]
#[inline]
unsafe fn dot_neon(a: &[f32], b: &[f32]) -> f32 {
    let mut sum = vdupq_n_f32(0.0);
    let chunks = a.len() / 4;

    for i in 0..chunks {
        let offset = i * 4;
        let va = vld1q_f32(a.as_ptr().add(offset));
        let vb = vld1q_f32(b.as_ptr().add(offset));
        sum = vfmaq_f32(sum, va, vb);
    }

    let mut result = vaddvq_f32(sum);

    // Procesar elementos restantes
    let remainder_start = chunks * 4;
    for i in remainder_start..a.len() {
        result += a[i] * b[i];
    }

    result
}

// ============================================================================
// AVX-512 Implementations (x86_64 modern servers)
// ============================================================================

#[allow(dead_code)]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx512f")]
unsafe fn euclidean_avx512(a: &[f32], b: &[f32]) -> f32 {
    let mut sum = _mm512_setzero_ps();
    let chunks = a.len() / 16;

    for i in 0..chunks {
        let offset = i * 16;
        let va = _mm512_loadu_ps(a.as_ptr().add(offset));
        let vb = _mm512_loadu_ps(b.as_ptr().add(offset));
        let diff = _mm512_sub_ps(va, vb);
        sum = _mm512_fmadd_ps(diff, diff, sum);
    }

    let mut result = _mm512_reduce_add_ps(sum);

    // Procesar elementos restantes
    let remainder_start = chunks * 16;
    for i in remainder_start..a.len() {
        let d = a[i] - b[i];
        result += d * d;
    }

    result.sqrt()
}

#[allow(dead_code)]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx512f")]
unsafe fn cosine_avx512(a: &[f32], b: &[f32]) -> f32 {
    let mut dot_sum = _mm512_setzero_ps();
    let mut norm_a_sum = _mm512_setzero_ps();
    let mut norm_b_sum = _mm512_setzero_ps();

    let chunks = a.len() / 16;

    for i in 0..chunks {
        let offset = i * 16;
        let va = _mm512_loadu_ps(a.as_ptr().add(offset));
        let vb = _mm512_loadu_ps(b.as_ptr().add(offset));

        dot_sum = _mm512_fmadd_ps(va, vb, dot_sum);
        norm_a_sum = _mm512_fmadd_ps(va, va, norm_a_sum);
        norm_b_sum = _mm512_fmadd_ps(vb, vb, norm_b_sum);
    }

    let mut dot = _mm512_reduce_add_ps(dot_sum);
    let mut norm_a = _mm512_reduce_add_ps(norm_a_sum);
    let mut norm_b = _mm512_reduce_add_ps(norm_b_sum);

    // Procesar elementos restantes
    let remainder_start = chunks * 16;
    for i in remainder_start..a.len() {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }

    let denom = (norm_a * norm_b).sqrt();
    if denom == 0.0 {
        return 1.0;
    }

    1.0 - (dot / denom)
}

#[allow(dead_code)]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx512f")]
unsafe fn dot_avx512(a: &[f32], b: &[f32]) -> f32 {
    let mut sum = _mm512_setzero_ps();
    let chunks = a.len() / 16;

    for i in 0..chunks {
        let offset = i * 16;
        let va = _mm512_loadu_ps(a.as_ptr().add(offset));
        let vb = _mm512_loadu_ps(b.as_ptr().add(offset));
        sum = _mm512_fmadd_ps(va, vb, sum);
    }

    let mut result = _mm512_reduce_add_ps(sum);

    // Procesar elementos restantes
    let remainder_start = chunks * 16;
    for i in remainder_start..a.len() {
        result += a[i] * b[i];
    }

    result
}

// ============================================================================
// AVX2 Implementations (x86_64 - most modern x86 CPUs)
// ============================================================================

#[allow(dead_code)]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
#[target_feature(enable = "fma")]
unsafe fn euclidean_avx2(a: &[f32], b: &[f32]) -> f32 {
    let mut sum = _mm256_setzero_ps();
    let chunks = a.len() / 8;

    for i in 0..chunks {
        let offset = i * 8;
        let va = _mm256_loadu_ps(a.as_ptr().add(offset));
        let vb = _mm256_loadu_ps(b.as_ptr().add(offset));
        let diff = _mm256_sub_ps(va, vb);
        sum = _mm256_fmadd_ps(diff, diff, sum);
    }

    let mut result = horizontal_sum_avx2(sum);

    // Procesar elementos restantes
    let remainder_start = chunks * 8;
    for i in remainder_start..a.len() {
        let d = a[i] - b[i];
        result += d * d;
    }

    result.sqrt()
}

#[allow(dead_code)]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
#[target_feature(enable = "fma")]
unsafe fn cosine_avx2(a: &[f32], b: &[f32]) -> f32 {
    let mut dot_sum = _mm256_setzero_ps();
    let mut norm_a_sum = _mm256_setzero_ps();
    let mut norm_b_sum = _mm256_setzero_ps();

    let chunks = a.len() / 8;

    for i in 0..chunks {
        let offset = i * 8;
        let va = _mm256_loadu_ps(a.as_ptr().add(offset));
        let vb = _mm256_loadu_ps(b.as_ptr().add(offset));

        dot_sum = _mm256_fmadd_ps(va, vb, dot_sum);
        norm_a_sum = _mm256_fmadd_ps(va, va, norm_a_sum);
        norm_b_sum = _mm256_fmadd_ps(vb, vb, norm_b_sum);
    }

    let mut dot = horizontal_sum_avx2(dot_sum);
    let mut norm_a = horizontal_sum_avx2(norm_a_sum);
    let mut norm_b = horizontal_sum_avx2(norm_b_sum);

    // Procesar elementos restantes
    let remainder_start = chunks * 8;
    for i in remainder_start..a.len() {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }

    let denom = (norm_a * norm_b).sqrt();
    if denom == 0.0 {
        return 1.0;
    }

    1.0 - (dot / denom)
}

#[allow(dead_code)]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
#[target_feature(enable = "fma")]
unsafe fn dot_avx2(a: &[f32], b: &[f32]) -> f32 {
    let mut sum = _mm256_setzero_ps();
    let chunks = a.len() / 8;

    for i in 0..chunks {
        let offset = i * 8;
        let va = _mm256_loadu_ps(a.as_ptr().add(offset));
        let vb = _mm256_loadu_ps(b.as_ptr().add(offset));
        sum = _mm256_fmadd_ps(va, vb, sum);
    }

    let mut result = horizontal_sum_avx2(sum);

    // Procesar elementos restantes
    let remainder_start = chunks * 8;
    for i in remainder_start..a.len() {
        result += a[i] * b[i];
    }

    result
}

/// Suma horizontal de un registro AVX2 de 8 floats
#[allow(dead_code)]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
#[inline]
unsafe fn horizontal_sum_avx2(v: __m256) -> f32 {
    let v128_low = _mm256_castps256_ps128(v);
    let v128_high = _mm256_extractf128_ps(v, 1);
    let v128 = _mm_add_ps(v128_low, v128_high);
    let v64 = _mm_add_ps(v128, _mm_movehl_ps(v128, v128));
    let v32 = _mm_add_ss(v64, _mm_shuffle_ps(v64, v64, 1));
    _mm_cvtss_f32(v32)
}

// ============================================================================
// SSE Implementations (x86_64 - fallback for older CPUs)
// ============================================================================

#[allow(dead_code)]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse")]
unsafe fn euclidean_sse(a: &[f32], b: &[f32]) -> f32 {
    let mut sum = _mm_setzero_ps();
    let chunks = a.len() / 4;

    for i in 0..chunks {
        let offset = i * 4;
        let va = _mm_loadu_ps(a.as_ptr().add(offset));
        let vb = _mm_loadu_ps(b.as_ptr().add(offset));
        let diff = _mm_sub_ps(va, vb);
        let sq = _mm_mul_ps(diff, diff);
        sum = _mm_add_ps(sum, sq);
    }

    let mut result = horizontal_sum_sse(sum);

    // Procesar elementos restantes
    let remainder_start = chunks * 4;
    for i in remainder_start..a.len() {
        let d = a[i] - b[i];
        result += d * d;
    }

    result.sqrt()
}

#[allow(dead_code)]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse")]
unsafe fn cosine_sse(a: &[f32], b: &[f32]) -> f32 {
    let mut dot_sum = _mm_setzero_ps();
    let mut norm_a_sum = _mm_setzero_ps();
    let mut norm_b_sum = _mm_setzero_ps();

    let chunks = a.len() / 4;

    for i in 0..chunks {
        let offset = i * 4;
        let va = _mm_loadu_ps(a.as_ptr().add(offset));
        let vb = _mm_loadu_ps(b.as_ptr().add(offset));

        let dot_prod = _mm_mul_ps(va, vb);
        dot_sum = _mm_add_ps(dot_sum, dot_prod);

        let norm_a_sq = _mm_mul_ps(va, va);
        norm_a_sum = _mm_add_ps(norm_a_sum, norm_a_sq);

        let norm_b_sq = _mm_mul_ps(vb, vb);
        norm_b_sum = _mm_add_ps(norm_b_sum, norm_b_sq);
    }

    let mut dot = horizontal_sum_sse(dot_sum);
    let mut norm_a = horizontal_sum_sse(norm_a_sum);
    let mut norm_b = horizontal_sum_sse(norm_b_sum);

    // Procesar elementos restantes
    let remainder_start = chunks * 4;
    for i in remainder_start..a.len() {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }

    let denom = (norm_a * norm_b).sqrt();
    if denom == 0.0 {
        return 1.0;
    }

    1.0 - (dot / denom)
}

#[allow(dead_code)]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse")]
unsafe fn dot_sse(a: &[f32], b: &[f32]) -> f32 {
    let mut sum = _mm_setzero_ps();
    let chunks = a.len() / 4;

    for i in 0..chunks {
        let offset = i * 4;
        let va = _mm_loadu_ps(a.as_ptr().add(offset));
        let vb = _mm_loadu_ps(b.as_ptr().add(offset));
        let prod = _mm_mul_ps(va, vb);
        sum = _mm_add_ps(sum, prod);
    }

    let mut result = horizontal_sum_sse(sum);

    // Procesar elementos restantes
    let remainder_start = chunks * 4;
    for i in remainder_start..a.len() {
        result += a[i] * b[i];
    }

    result
}

/// Suma horizontal de un registro SSE de 4 floats
#[allow(dead_code)]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse")]
#[inline]
unsafe fn horizontal_sum_sse(v: __m128) -> f32 {
    let shuf = _mm_shuffle_ps(v, v, 0b10_11_00_01);
    let sums = _mm_add_ps(v, shuf);
    let shuf = _mm_movehl_ps(sums, sums);
    let sums = _mm_add_ss(sums, shuf);
    _mm_cvtss_f32(sums)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_euclidean_scalar() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let dist = euclidean_scalar(&a, &b);
        assert!((dist - std::f32::consts::SQRT_2).abs() < 1e-5);
    }

    #[test]
    fn test_euclidean_simd() {
        let a: Vec<f32> = (0..128).map(|i| i as f32 / 128.0).collect();
        let b: Vec<f32> = (0..128).map(|i| (i + 1) as f32 / 128.0).collect();

        let scalar = euclidean_scalar(&a, &b);
        let simd = euclidean_distance(&a, &b);

        assert!(
            (scalar - simd).abs() < 1e-4,
            "scalar={}, simd={}",
            scalar,
            simd
        );
    }

    #[test]
    fn test_cosine_identical() {
        let a = vec![1.0, 2.0, 3.0];
        let dist = cosine_distance(&a, &a);
        assert!(dist.abs() < 1e-5);
    }

    #[test]
    fn test_cosine_orthogonal() {
        let a = vec![1.0, 0.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0, 0.0];
        let dist = cosine_distance(&a, &b);
        assert!((dist - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_cosine_simd() {
        let a: Vec<f32> = (0..128).map(|i| (i as f32 + 1.0) / 128.0).collect();
        let b: Vec<f32> = (0..128).map(|i| (i as f32 + 2.0) / 128.0).collect();

        let scalar = cosine_scalar(&a, &b);
        let simd = cosine_distance(&a, &b);

        assert!(
            (scalar - simd).abs() < 1e-4,
            "scalar={}, simd={}",
            scalar,
            simd
        );
    }

    #[test]
    fn test_dot_product() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        let dist = dot_product_distance(&a, &b);
        // dot = 1*4 + 2*5 + 3*6 = 32, distance = -32
        assert!((dist - (-32.0)).abs() < 1e-5);
    }

    #[test]
    fn test_dot_product_simd() {
        let a: Vec<f32> = (0..128).map(|i| i as f32 / 128.0).collect();
        let b: Vec<f32> = (0..128).map(|i| (i + 1) as f32 / 128.0).collect();

        let scalar = dot_scalar(&a, &b);
        let simd = dot_product_distance(&a, &b);

        assert!(
            (scalar - simd).abs() < 1e-4,
            "scalar={}, simd={}",
            scalar,
            simd
        );
    }

    #[test]
    fn test_small_vectors() {
        // Vectores pequeños que no usan SIMD
        let a = vec![1.0, 2.0];
        let b = vec![3.0, 4.0];

        let euc = euclidean_distance(&a, &b);
        let cos = cosine_distance(&a, &b);
        let dot = dot_product_distance(&a, &b);

        // Euclidean: sqrt((3-1)^2 + (4-2)^2) = sqrt(8) ≈ 2.828
        assert!((euc - 2.828).abs() < 0.01);

        // Cosine should be small (vectors in similar direction)
        assert!(cos < 0.1);

        // Dot product: -(1*3 + 2*4) = -11
        assert!((dot - (-11.0)).abs() < 1e-5);
    }

    #[test]
    fn test_large_vectors() {
        // Vectores grandes para forzar uso de SIMD
        let a: Vec<f32> = (0..1024).map(|i| (i as f32).sin()).collect();
        let b: Vec<f32> = (0..1024).map(|i| (i as f32).cos()).collect();

        let euc = euclidean_distance(&a, &b);
        let cos = cosine_distance(&a, &b);
        let dot = dot_product_distance(&a, &b);

        // Just verify they return reasonable values
        assert!(euc >= 0.0);
        assert!(cos >= 0.0 && cos <= 2.0);
        assert!(dot.is_finite());
    }
}
