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
    /// Distancia Manhattan (L1)
    /// - Suma de diferencias absolutas
    /// - 0 = vectores idénticos
    /// - Rango: [0, ∞)
    Manhattan,
}

impl Distance {
    /// Calcula la distancia entre dos vectores.
    ///
    /// Usa implementaciones SIMD (AVX2/SSE) cuando están disponibles
    /// para máximo rendimiento.
    ///
    /// # Panics
    ///
    /// Panica si `a` y `b` tienen longitudes distintas. Las rutas SIMD indexan
    /// `b` en los mismos offsets que `a` sin verificar límites, así que una
    /// longitud inconsistente sería lectura fuera de límites (UB); el panic
    /// en esta capa segura previene el UB antes de despachar.
    #[inline]
    pub fn calculate(&self, a: &[f32], b: &[f32]) -> f32 {
        assert_eq!(
            a.len(),
            b.len(),
            "Distance::calculate requiere vectores de igual longitud (a.len() = {}, b.len() = {})",
            a.len(),
            b.len()
        );
        match self {
            Distance::Cosine => simd::cosine_distance(a, b),
            Distance::Euclidean => simd::euclidean_distance(a, b),
            Distance::DotProduct => simd::dot_product_distance(a, b),
            Distance::Manhattan => simd::manhattan_distance(a, b),
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
    fn test_manhattan_identical_vectors() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        let dist = Distance::Manhattan.calculate(&a, &b);
        assert!((dist - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_manhattan_known_distance() {
        let a = vec![0.0, 0.0];
        let b = vec![3.0, 4.0];
        let dist = Distance::Manhattan.calculate(&a, &b);
        assert!((dist - 7.0).abs() < 1e-6); // |3| + |4| = 7
    }

    #[test]
    fn test_dot_product() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        let dist = Distance::DotProduct.calculate(&a, &b);
        // dot = 1*4 + 2*5 + 3*6 = 32, distance = -32
        assert!((dist - (-32.0)).abs() < 1e-6);
    }

    #[test]
    #[should_panic(expected = "vectores de igual longitud")]
    fn test_cosine_mismatched_lengths_panics() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0];
        Distance::Cosine.calculate(&a, &b);
    }

    #[test]
    #[should_panic(expected = "vectores de igual longitud")]
    fn test_euclidean_mismatched_lengths_panics() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0];
        Distance::Euclidean.calculate(&a, &b);
    }

    #[test]
    #[should_panic(expected = "vectores de igual longitud")]
    fn test_dot_product_mismatched_lengths_panics() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0];
        Distance::DotProduct.calculate(&a, &b);
    }

    #[test]
    #[should_panic(expected = "vectores de igual longitud")]
    fn test_manhattan_mismatched_lengths_panics() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0];
        Distance::Manhattan.calculate(&a, &b);
    }

    #[test]
    fn test_equal_lengths_unchanged() {
        // Valores tomados de los tests existentes: el guard no altera resultados.
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        assert!((Distance::Cosine.calculate(&a, &b) - 0.0).abs() < 1e-6);
        assert!((Distance::Euclidean.calculate(&a, &b) - 0.0).abs() < 1e-6);
        assert!((Distance::Manhattan.calculate(&a, &b) - 0.0).abs() < 1e-6);

        let c = vec![0.0, 0.0];
        let d = vec![3.0, 4.0];
        assert!((Distance::Euclidean.calculate(&c, &d) - 5.0).abs() < 1e-6);
        assert!((Distance::Manhattan.calculate(&c, &d) - 7.0).abs() < 1e-6);

        let e = vec![1.0, 2.0, 3.0];
        let f = vec![4.0, 5.0, 6.0];
        assert!((Distance::DotProduct.calculate(&e, &f) - (-32.0)).abs() < 1e-6);
    }
}
