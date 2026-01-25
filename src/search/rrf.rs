//! Reciprocal Rank Fusion (RRF) para combinar listas rankeadas.
//!
//! RRF es un método para combinar múltiples listas rankeadas en una sola,
//! sin necesidad de normalizar scores entre diferentes sistemas de ranking.
//!
//! # Fórmula
//!
//! RRF Score = Σ(1 / (k + rank_i)) para cada lista
//!
//! donde k es una constante (típicamente 60) que suaviza el ranking.

use crate::types::VectorId;
use std::collections::HashMap;

/// Constante k por defecto para RRF (valor estándar de la literatura)
pub const DEFAULT_RRF_K: f32 = 60.0;

/// Resultado de una lista rankeada.
#[derive(Debug, Clone)]
pub struct RankedResult {
    /// ID del documento
    pub id: VectorId,
    /// Rank en esta lista (0-indexed)
    pub rank: usize,
    /// Score original (para referencia)
    pub original_score: f32,
}

/// Aplica Reciprocal Rank Fusion a múltiples listas rankeadas.
///
/// # Arguments
/// * `ranked_lists` - Vector de listas rankeadas
/// * `k` - Constante de suavizado (típicamente 60)
///
/// # Returns
/// Vector de (id, rrf_score) ordenado por score descendente.
///
/// # Ejemplo
///
/// ```rust
/// use minimemory::search::{reciprocal_rank_fusion, RankedResult};
///
/// let vector_results = vec![
///     RankedResult { id: "doc-1".into(), rank: 0, original_score: 0.95 },
///     RankedResult { id: "doc-2".into(), rank: 1, original_score: 0.80 },
/// ];
///
/// let keyword_results = vec![
///     RankedResult { id: "doc-2".into(), rank: 0, original_score: 5.2 },
///     RankedResult { id: "doc-3".into(), rank: 1, original_score: 3.1 },
/// ];
///
/// let fused = reciprocal_rank_fusion(vec![vector_results, keyword_results], 60.0);
/// // doc-2 aparece en ambas listas, tendrá el score más alto
/// ```
pub fn reciprocal_rank_fusion(
    ranked_lists: Vec<Vec<RankedResult>>,
    k: f32,
) -> Vec<(VectorId, f32)> {
    let mut rrf_scores: HashMap<VectorId, f32> = HashMap::new();

    for list in ranked_lists {
        for result in list {
            let rrf_contribution = 1.0 / (k + result.rank as f32 + 1.0); // +1 porque rank es 0-indexed
            *rrf_scores.entry(result.id).or_insert(0.0) += rrf_contribution;
        }
    }

    let mut results: Vec<_> = rrf_scores.into_iter().collect();
    // Mayor RRF score = mejor
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    results
}

/// RRF con pesos personalizados para cada lista.
///
/// # Arguments
/// * `ranked_lists` - Vector de tuplas (lista, peso)
/// * `k` - Constante de suavizado
///
/// # Example
///
/// ```rust
/// use minimemory::search::{weighted_reciprocal_rank_fusion, RankedResult};
///
/// let vector_results = vec![
///     RankedResult { id: "doc-1".into(), rank: 0, original_score: 0.95 },
/// ];
///
/// let keyword_results = vec![
///     RankedResult { id: "doc-2".into(), rank: 0, original_score: 5.2 },
/// ];
///
/// // Dar más peso a la búsqueda vectorial
/// let fused = weighted_reciprocal_rank_fusion(
///     vec![(vector_results, 0.7), (keyword_results, 0.3)],
///     60.0
/// );
/// ```
pub fn weighted_reciprocal_rank_fusion(
    ranked_lists: Vec<(Vec<RankedResult>, f32)>,
    k: f32,
) -> Vec<(VectorId, f32)> {
    let mut rrf_scores: HashMap<VectorId, f32> = HashMap::new();

    for (list, weight) in ranked_lists {
        for result in list {
            let rrf_contribution = weight * (1.0 / (k + result.rank as f32 + 1.0));
            *rrf_scores.entry(result.id).or_insert(0.0) += rrf_contribution;
        }
    }

    let mut results: Vec<_> = rrf_scores.into_iter().collect();
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rrf_basic() {
        let list1 = vec![
            RankedResult {
                id: "a".into(),
                rank: 0,
                original_score: 1.0,
            },
            RankedResult {
                id: "b".into(),
                rank: 1,
                original_score: 0.9,
            },
            RankedResult {
                id: "c".into(),
                rank: 2,
                original_score: 0.8,
            },
        ];

        let list2 = vec![
            RankedResult {
                id: "b".into(),
                rank: 0,
                original_score: 5.0,
            },
            RankedResult {
                id: "a".into(),
                rank: 1,
                original_score: 4.0,
            },
            RankedResult {
                id: "d".into(),
                rank: 2,
                original_score: 3.0,
            },
        ];

        let results = reciprocal_rank_fusion(vec![list1, list2], 60.0);

        // Both 'a' and 'b' appear in both lists
        // 'b' should rank higher because: rank 0 in list2, rank 1 in list1
        // vs 'a' with rank 0 in list1, rank 1 in list2 - same total but b has better position in list2
        assert!(results.len() >= 3);

        // Find positions
        let a_pos = results.iter().position(|(id, _)| id == "a").unwrap();
        let b_pos = results.iter().position(|(id, _)| id == "b").unwrap();
        let d_pos = results.iter().position(|(id, _)| id == "d").unwrap();

        // a and b should be top 2, d should be lower
        assert!(a_pos < d_pos);
        assert!(b_pos < d_pos);
    }

    #[test]
    fn test_rrf_single_list() {
        let list = vec![
            RankedResult {
                id: "a".into(),
                rank: 0,
                original_score: 1.0,
            },
            RankedResult {
                id: "b".into(),
                rank: 1,
                original_score: 0.5,
            },
        ];

        let results = reciprocal_rank_fusion(vec![list], 60.0);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "a");
        assert_eq!(results[1].0, "b");
        assert!(results[0].1 > results[1].1);
    }

    #[test]
    fn test_weighted_rrf() {
        let list1 = vec![RankedResult {
            id: "a".into(),
            rank: 0,
            original_score: 1.0,
        }];

        let list2 = vec![RankedResult {
            id: "b".into(),
            rank: 0,
            original_score: 1.0,
        }];

        // Give much more weight to list2
        let results = weighted_reciprocal_rank_fusion(vec![(list1, 0.1), (list2, 0.9)], 60.0);

        assert_eq!(results.len(), 2);
        // 'b' should rank higher due to higher weight
        assert_eq!(results[0].0, "b");
        assert!(results[0].1 > results[1].1);
    }

    #[test]
    fn test_empty_lists() {
        let results = reciprocal_rank_fusion(vec![], 60.0);
        assert!(results.is_empty());

        let results = reciprocal_rank_fusion(vec![vec![]], 60.0);
        assert!(results.is_empty());
    }
}
