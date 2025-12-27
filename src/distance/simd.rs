//! Implementaciones SIMD optimizadas para cálculos de distancia.
//!
//! Proporciona versiones aceleradas de las métricas de distancia
//! usando instrucciones SIMD (AVX2, SSE) cuando están disponibles.

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

/// Calcula la distancia euclidiana usando SIMD si está disponible.
#[inline]
pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
    {
        if is_x86_feature_detected!("avx2") && a.len() >= 8 {
            return unsafe { euclidean_avx2(a, b) };
        }
    }

    #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
    {
        if is_x86_feature_detected!("sse") && a.len() >= 4 {
            return unsafe { euclidean_sse(a, b) };
        }
    }

    euclidean_scalar(a, b)
}

/// Calcula la similitud coseno usando SIMD si está disponible.
#[inline]
pub fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
    #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
    {
        if is_x86_feature_detected!("avx2") && a.len() >= 8 {
            return unsafe { cosine_avx2(a, b) };
        }
    }

    cosine_scalar(a, b)
}

/// Calcula el producto punto usando SIMD si está disponible.
#[inline]
pub fn dot_product_distance(a: &[f32], b: &[f32]) -> f32 {
    #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
    {
        if is_x86_feature_detected!("avx2") && a.len() >= 8 {
            return unsafe { -dot_avx2(a, b) };
        }
    }

    dot_scalar(a, b)
}

// ============================================================================
// Implementaciones escalares (fallback)
// ============================================================================

#[inline]
fn euclidean_scalar(a: &[f32], b: &[f32]) -> f32 {
    let mut sum = 0.0f32;
    let chunks = a.len() / 4;
    let remainder = a.len() % 4;

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

// ============================================================================
// Implementaciones AVX2
// ============================================================================

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

    // Reducción horizontal
    let mut result = horizontal_sum_avx2(sum);

    // Procesar elementos restantes
    let remainder_start = chunks * 8;
    for i in remainder_start..a.len() {
        let d = a[i] - b[i];
        result += d * d;
    }

    result.sqrt()
}

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
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
#[inline]
unsafe fn horizontal_sum_avx2(v: __m256) -> f32 {
    // v = [a0, a1, a2, a3, a4, a5, a6, a7]
    // Sumar las dos mitades: [a0+a4, a1+a5, a2+a6, a3+a7, ...]
    let v128_low = _mm256_castps256_ps128(v);
    let v128_high = _mm256_extractf128_ps(v, 1);
    let v128 = _mm_add_ps(v128_low, v128_high);

    // Ahora tenemos 4 valores, hacer suma horizontal
    let v64 = _mm_add_ps(v128, _mm_movehl_ps(v128, v128));
    let v32 = _mm_add_ss(v64, _mm_shuffle_ps(v64, v64, 1));

    _mm_cvtss_f32(v32)
}

// ============================================================================
// Implementaciones SSE
// ============================================================================

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

    // Reducción horizontal
    let shuf = _mm_shuffle_ps(sum, sum, 0b10_11_00_01);
    let sums = _mm_add_ps(sum, shuf);
    let shuf = _mm_movehl_ps(sums, sums);
    let sums = _mm_add_ss(sums, shuf);
    let mut result = _mm_cvtss_f32(sums);

    // Procesar elementos restantes
    let remainder_start = chunks * 4;
    for i in remainder_start..a.len() {
        let d = a[i] - b[i];
        result += d * d;
    }

    result.sqrt()
}

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

        assert!((scalar - simd).abs() < 1e-4, "scalar={}, simd={}", scalar, simd);
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
    fn test_dot_product() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        let dist = dot_product_distance(&a, &b);
        // dot = 1*4 + 2*5 + 3*6 = 32, distance = -32
        assert!((dist - (-32.0)).abs() < 1e-5);
    }
}
