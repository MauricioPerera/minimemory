//! Motor de búsqueda híbrida.
//!
//! Combina búsqueda vectorial, búsqueda por keywords (BM25), y filtros de metadata
//! usando Reciprocal Rank Fusion (RRF) para obtener resultados óptimos.
//!
//! # Modos de Búsqueda
//!
//! - **Vector**: Solo similitud vectorial
//! - **Keyword**: Solo BM25 (full-text)
//! - **Hybrid**: Combina vector + keyword con RRF
//! - **FilterOnly**: Solo filtros de metadata, sin ranking
//!
//! # Ejemplo
//!
//! ```rust
//! use minimemory::{HybridSearchParams, SearchMode, Filter};
//!
//! // Vector de consulta (embedding)
//! let query_vec = vec![0.1, 0.2, 0.3];
//!
//! // Búsqueda híbrida (vector + keywords)
//! let params = HybridSearchParams::hybrid(query_vec.clone(), "rust programming", 10);
//!
//! // Búsqueda vectorial con filtro de metadata
//! let params = HybridSearchParams::vector(query_vec, 10)
//!     .with_filter(Filter::eq("category", "tech"));
//! ```

mod hybrid;
mod rrf;

pub use hybrid::{HybridSearch, HybridSearchParams, SearchMode};
pub use rrf::{reciprocal_rank_fusion, weighted_reciprocal_rank_fusion, RankedResult};
