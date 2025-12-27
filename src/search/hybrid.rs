//! Motor de búsqueda híbrida.
//!
//! Combina búsqueda vectorial, BM25 y filtros de metadata.

use crate::distance::Distance;
use crate::error::{Error, Result};
use crate::index::{BM25Index, Index};
use crate::query::{Filter, FilterEvaluator};
use crate::storage::Storage;
use crate::types::{HybridSearchResult, Metadata, VectorId};

use super::rrf::{weighted_reciprocal_rank_fusion, RankedResult, DEFAULT_RRF_K};

/// Modo de búsqueda.
#[derive(Debug, Clone)]
pub enum SearchMode {
    /// Solo búsqueda vectorial (similitud)
    Vector,
    /// Solo búsqueda keyword (BM25)
    Keyword,
    /// Híbrida: vector + keyword con RRF
    Hybrid {
        /// Peso para resultados vectoriales (0.0-1.0)
        vector_weight: f32,
        /// Peso para resultados keyword (0.0-1.0)
        keyword_weight: f32,
    },
    /// Solo filtro de metadata (sin ranking por similitud)
    FilterOnly,
}

impl Default for SearchMode {
    fn default() -> Self {
        SearchMode::Vector
    }
}

/// Parámetros de búsqueda híbrida.
#[derive(Debug, Clone)]
pub struct HybridSearchParams {
    /// Vector de consulta (requerido para Vector/Hybrid)
    pub vector: Option<Vec<f32>>,
    /// Query de texto (requerido para Keyword/Hybrid)
    pub text_query: Option<String>,
    /// Filtro de metadata (opcional)
    pub filter: Option<Filter>,
    /// Modo de búsqueda
    pub mode: SearchMode,
    /// Número de resultados
    pub k: usize,
}

impl HybridSearchParams {
    /// Crea parámetros para búsqueda vectorial.
    pub fn vector(query: Vec<f32>, k: usize) -> Self {
        Self {
            vector: Some(query),
            text_query: None,
            filter: None,
            mode: SearchMode::Vector,
            k,
        }
    }

    /// Crea parámetros para búsqueda por keyword.
    pub fn keyword(query: impl Into<String>, k: usize) -> Self {
        Self {
            vector: None,
            text_query: Some(query.into()),
            filter: None,
            mode: SearchMode::Keyword,
            k,
        }
    }

    /// Crea parámetros para búsqueda híbrida.
    pub fn hybrid(
        vector: Vec<f32>,
        text: impl Into<String>,
        k: usize,
    ) -> Self {
        Self {
            vector: Some(vector),
            text_query: Some(text.into()),
            filter: None,
            mode: SearchMode::Hybrid {
                vector_weight: 0.5,
                keyword_weight: 0.5,
            },
            k,
        }
    }

    /// Crea parámetros para búsqueda solo por filtro.
    pub fn filter_only(filter: Filter, limit: usize) -> Self {
        Self {
            vector: None,
            text_query: None,
            filter: Some(filter),
            mode: SearchMode::FilterOnly,
            k: limit,
        }
    }

    /// Añade un filtro de metadata.
    pub fn with_filter(mut self, filter: Filter) -> Self {
        self.filter = Some(filter);
        self
    }

    /// Configura los pesos para búsqueda híbrida.
    pub fn with_weights(mut self, vector_weight: f32, keyword_weight: f32) -> Self {
        self.mode = SearchMode::Hybrid {
            vector_weight,
            keyword_weight,
        };
        self
    }
}

/// Motor de búsqueda híbrida.
pub struct HybridSearch;

impl HybridSearch {
    /// Ejecuta búsqueda híbrida.
    ///
    /// # Arguments
    /// * `params` - Parámetros de búsqueda
    /// * `vector_index` - Índice vectorial
    /// * `bm25_index` - Índice BM25 (opcional)
    /// * `storage` - Storage de documentos
    /// * `distance` - Métrica de distancia
    pub fn search(
        params: &HybridSearchParams,
        vector_index: &dyn Index,
        bm25_index: Option<&BM25Index>,
        storage: &dyn Storage,
        distance: Distance,
    ) -> Result<Vec<HybridSearchResult>> {
        match &params.mode {
            SearchMode::Vector => {
                Self::vector_search(params, vector_index, storage, distance)
            }
            SearchMode::Keyword => {
                Self::keyword_search(params, bm25_index, storage)
            }
            SearchMode::Hybrid { vector_weight, keyword_weight } => {
                Self::hybrid_search(
                    params,
                    vector_index,
                    bm25_index,
                    storage,
                    distance,
                    *vector_weight,
                    *keyword_weight,
                )
            }
            SearchMode::FilterOnly => {
                Self::filter_only_search(params, storage)
            }
        }
    }

    fn vector_search(
        params: &HybridSearchParams,
        index: &dyn Index,
        storage: &dyn Storage,
        distance: Distance,
    ) -> Result<Vec<HybridSearchResult>> {
        let query = params.vector.as_ref()
            .ok_or_else(|| Error::InvalidConfig(
                "Vector query required for vector search".into()
            ))?;

        // Buscar más resultados si hay filtro (pre-filter approach)
        let search_k = if params.filter.is_some() {
            params.k * 10  // Buscar 10x más para compensar filtrado
        } else {
            params.k
        };

        let results = index.search(query, search_k, storage, distance)?;

        // Aplicar filtro
        let filtered: Vec<_> = results.into_iter()
            .filter(|r| {
                if let Some(filter) = &params.filter {
                    FilterEvaluator::evaluate(filter, r.metadata.as_ref())
                } else {
                    true
                }
            })
            .take(params.k)
            .enumerate()
            .map(|(rank, r)| HybridSearchResult {
                id: r.id,
                score: r.distance,  // Menor = mejor
                vector_distance: Some(r.distance),
                bm25_score: None,
                vector_rank: Some(rank),
                keyword_rank: None,
                metadata: r.metadata,
            })
            .collect();

        Ok(filtered)
    }

    fn keyword_search(
        params: &HybridSearchParams,
        bm25_index: Option<&BM25Index>,
        storage: &dyn Storage,
    ) -> Result<Vec<HybridSearchResult>> {
        let query = params.text_query.as_ref()
            .ok_or_else(|| Error::InvalidConfig(
                "Text query required for keyword search".into()
            ))?;

        let index = bm25_index
            .ok_or_else(|| Error::InvalidConfig(
                "BM25 index required for keyword search".into()
            ))?;

        let search_k = if params.filter.is_some() {
            params.k * 10
        } else {
            params.k
        };

        let results = index.search(query, search_k);

        let mut hybrid_results = Vec::new();
        for (rank, result) in results.into_iter().enumerate() {
            if let Ok(Some(doc)) = storage.get(&result.id) {
                // Aplicar filtro
                if let Some(filter) = &params.filter {
                    if !FilterEvaluator::evaluate(filter, doc.metadata.as_ref()) {
                        continue;
                    }
                }

                hybrid_results.push(HybridSearchResult {
                    id: result.id,
                    score: -result.score,  // Negativo para que menor = mejor (consistente con distance)
                    vector_distance: None,
                    bm25_score: Some(result.score),
                    vector_rank: None,
                    keyword_rank: Some(rank),
                    metadata: doc.metadata,
                });

                if hybrid_results.len() >= params.k {
                    break;
                }
            }
        }

        Ok(hybrid_results)
    }

    fn hybrid_search(
        params: &HybridSearchParams,
        vector_index: &dyn Index,
        bm25_index: Option<&BM25Index>,
        storage: &dyn Storage,
        distance: Distance,
        vector_weight: f32,
        keyword_weight: f32,
    ) -> Result<Vec<HybridSearchResult>> {
        // Obtener resultados de ambas búsquedas
        let fetch_k = params.k * 3;  // Fetch más para RRF

        // Vector search
        let vector_results = if let Some(query) = &params.vector {
            let results = vector_index.search(query, fetch_k, storage, distance)?;
            results.into_iter()
                .enumerate()
                .map(|(rank, r)| RankedResult {
                    id: r.id,
                    rank,
                    original_score: r.distance,
                })
                .collect()
        } else {
            Vec::new()
        };

        // Keyword search
        let keyword_results = if let (Some(query), Some(index)) = (&params.text_query, bm25_index) {
            index.search(query, fetch_k)
                .into_iter()
                .enumerate()
                .map(|(rank, result)| RankedResult {
                    id: result.id,
                    rank,
                    original_score: result.score,
                })
                .collect()
        } else {
            Vec::new()
        };

        // Guardar info original para lookups
        let vector_info: std::collections::HashMap<_, _> = vector_results
            .iter()
            .map(|r| (r.id.clone(), (r.rank, r.original_score)))
            .collect();

        let keyword_info: std::collections::HashMap<_, _> = keyword_results
            .iter()
            .map(|r| (r.id.clone(), (r.rank, r.original_score)))
            .collect();

        // Aplicar RRF con pesos
        let rrf_results = weighted_reciprocal_rank_fusion(
            vec![
                (vector_results, vector_weight),
                (keyword_results, keyword_weight),
            ],
            DEFAULT_RRF_K,
        );

        // Construir resultados finales
        let mut final_results = Vec::new();
        for (id, rrf_score) in rrf_results {
            if let Ok(Some(doc)) = storage.get(&id) {
                // Aplicar filtro
                if let Some(filter) = &params.filter {
                    if !FilterEvaluator::evaluate(filter, doc.metadata.as_ref()) {
                        continue;
                    }
                }

                let (vec_rank, vec_dist) = vector_info.get(&id)
                    .map(|(r, d)| (Some(*r), Some(*d)))
                    .unwrap_or((None, None));

                let (kw_rank, kw_score) = keyword_info.get(&id)
                    .map(|(r, s)| (Some(*r), Some(*s)))
                    .unwrap_or((None, None));

                final_results.push(HybridSearchResult {
                    id,
                    score: -rrf_score,  // Negativo para que menor = mejor
                    vector_distance: vec_dist,
                    bm25_score: kw_score,
                    vector_rank: vec_rank,
                    keyword_rank: kw_rank,
                    metadata: doc.metadata,
                });

                if final_results.len() >= params.k {
                    break;
                }
            }
        }

        Ok(final_results)
    }

    fn filter_only_search(
        params: &HybridSearchParams,
        storage: &dyn Storage,
    ) -> Result<Vec<HybridSearchResult>> {
        let filter = params.filter.as_ref()
            .ok_or_else(|| Error::InvalidConfig(
                "Filter required for filter-only search".into()
            ))?;

        let results: Vec<_> = storage.iter()
            .filter(|doc| FilterEvaluator::evaluate(filter, doc.metadata.as_ref()))
            .take(params.k)
            .map(|doc| HybridSearchResult {
                id: doc.id,
                score: 0.0,  // Sin ranking
                vector_distance: None,
                bm25_score: None,
                vector_rank: None,
                keyword_rank: None,
                metadata: doc.metadata,
            })
            .collect();

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::FlatIndex;
    use crate::storage::MemoryStorage;
    use crate::types::Metadata;
    use std::sync::Arc;

    fn setup_test_data() -> (Arc<MemoryStorage>, Arc<FlatIndex>, Arc<BM25Index>) {
        let storage = Arc::new(MemoryStorage::new());
        let vector_index = Arc::new(FlatIndex::new());
        let bm25_index = Arc::new(BM25Index::new(vec!["title".into(), "content".into()]));

        // Doc 1: About Rust
        let mut meta1 = Metadata::new();
        meta1.insert("title", "Rust Programming");
        meta1.insert("content", "Learn Rust systems programming");
        meta1.insert("category", "tech");
        storage.insert("doc-1".into(), Some(vec![1.0, 0.0, 0.0]), Some(meta1.clone())).unwrap();
        vector_index.add("doc-1", &[1.0, 0.0, 0.0]).unwrap();
        bm25_index.add("doc-1", Some(&meta1)).unwrap();

        // Doc 2: About Python
        let mut meta2 = Metadata::new();
        meta2.insert("title", "Python Guide");
        meta2.insert("content", "Python for beginners programming");
        meta2.insert("category", "tech");
        storage.insert("doc-2".into(), Some(vec![0.0, 1.0, 0.0]), Some(meta2.clone())).unwrap();
        vector_index.add("doc-2", &[0.0, 1.0, 0.0]).unwrap();
        bm25_index.add("doc-2", Some(&meta2)).unwrap();

        // Doc 3: About Cooking (different category)
        let mut meta3 = Metadata::new();
        meta3.insert("title", "Cooking Recipes");
        meta3.insert("content", "Delicious food recipes");
        meta3.insert("category", "food");
        storage.insert("doc-3".into(), Some(vec![0.0, 0.0, 1.0]), Some(meta3.clone())).unwrap();
        vector_index.add("doc-3", &[0.0, 0.0, 1.0]).unwrap();
        bm25_index.add("doc-3", Some(&meta3)).unwrap();

        (storage, vector_index, bm25_index)
    }

    #[test]
    fn test_vector_search() {
        let (storage, vector_index, _) = setup_test_data();

        let params = HybridSearchParams::vector(vec![1.0, 0.0, 0.0], 2);
        let results = HybridSearch::search(
            &params,
            vector_index.as_ref(),
            None,
            storage.as_ref(),
            Distance::Euclidean,
        ).unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "doc-1"); // Closest to query
    }

    #[test]
    fn test_keyword_search() {
        let (storage, _, bm25_index) = setup_test_data();
        let vector_index = FlatIndex::new();

        let params = HybridSearchParams::keyword("rust programming", 2);
        let results = HybridSearch::search(
            &params,
            &vector_index,
            Some(bm25_index.as_ref()),
            storage.as_ref(),
            Distance::Euclidean,
        ).unwrap();

        assert!(!results.is_empty());
        assert_eq!(results[0].id, "doc-1"); // Has "rust" and "programming"
    }

    #[test]
    fn test_hybrid_search() {
        let (storage, vector_index, bm25_index) = setup_test_data();

        let params = HybridSearchParams::hybrid(
            vec![0.0, 1.0, 0.0], // Closest to doc-2
            "rust",              // Matches doc-1
            3,
        );

        let results = HybridSearch::search(
            &params,
            vector_index.as_ref(),
            Some(bm25_index.as_ref()),
            storage.as_ref(),
            Distance::Euclidean,
        ).unwrap();

        assert!(!results.is_empty());
        // Both doc-1 (rust keyword) and doc-2 (vector) should be in results
    }

    #[test]
    fn test_filter_search() {
        let (storage, vector_index, _) = setup_test_data();

        let filter = Filter::eq("category", "tech");
        let params = HybridSearchParams::vector(vec![0.5, 0.5, 0.0], 10)
            .with_filter(filter);

        let results = HybridSearch::search(
            &params,
            vector_index.as_ref(),
            None,
            storage.as_ref(),
            Distance::Euclidean,
        ).unwrap();

        // Should only return tech category (doc-1 and doc-2)
        assert_eq!(results.len(), 2);
        for r in &results {
            assert!(r.id == "doc-1" || r.id == "doc-2");
        }
    }

    #[test]
    fn test_filter_only_search() {
        let (storage, vector_index, _) = setup_test_data();

        let filter = Filter::eq("category", "food");
        let params = HybridSearchParams::filter_only(filter, 10);

        let results = HybridSearch::search(
            &params,
            vector_index.as_ref(),
            None,
            storage.as_ref(),
            Distance::Euclidean,
        ).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "doc-3");
    }
}
