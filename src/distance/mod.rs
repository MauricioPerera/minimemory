mod simd;

use serde::{Deserialize, Serialize};

/// Métricas de distancia para similitud vectorial.
///
/// Todas las métricas retornan valores donde **menor = más similar**.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Distance {
    /// Distancia coseno (1 - similitud_coseno)
    /// - 0 = vectores idénticos
    /// - 1 = vectores ortogonales
    /// - 2 = vectores opuestos
    Cosine,
    /// Distancia euclidiana (L2)
    /// - 0 = vectores idénticos
    /// - Rango: [0, ∞)
    Euclidean,
    /// Producto punto negativo
    /// - Valores más negativos = más similares
    DotProduct,
}

impl Distance {
    /// Calcula la distancia entre dos vectores.
    ///
    /// Usa implementaciones SIMD (AVX2/SSE) cuando están disponibles
    /// para máximo rendimiento.
    #[inline]
    pub fn calculate(&self, a: &[f32], b: &[f32]) -> f32 {
        match self {
            Distance::Cosine => simd::cosine_distance(a, b),
            Distance::Euclidean => simd::euclidean_distance(a, b),
            Distance::DotProduct => simd::dot_product_distance(a, b),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_identical_vectors() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        let dist = Distance::Cosine.calculate(&a, &b);
        assert!((dist - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_orthogonal_vectors() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let dist = Distance::Cosine.calculate(&a, &b);
        assert!((dist - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_euclidean_identical_vectors() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        let dist = Distance::Euclidean.calculate(&a, &b);
        assert!((dist - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_euclidean_known_distance() {
        let a = vec![0.0, 0.0];
        let b = vec![3.0, 4.0];
        let dist = Distance::Euclidean.calculate(&a, &b);
        assert!((dist - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_dot_product() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        let dist = Distance::DotProduct.calculate(&a, &b);
        // dot = 1*4 + 2*5 + 3*6 = 32, distance = -32
        assert!((dist - (-32.0)).abs() < 1e-6);
    }
}
