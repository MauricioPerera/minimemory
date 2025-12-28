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
//! use minimemory::search::{HybridSearchParams, SearchMode};
//!
//! // Búsqueda híbrida
//! let params = HybridSearchParams::hybrid(query_vec, "rust programming", 10);
//!
//! // Búsqueda con filtro
//! let params = HybridSearchParams::vector(query_vec, 10)
//!     .with_filter(Filter::eq("category", "tech"));
//! ```

mod hybrid;
mod rrf;

pub use hybrid::{HybridSearch, HybridSearchParams, SearchMode};
pub use rrf::{reciprocal_rank_fusion, weighted_reciprocal_rank_fusion, RankedResult};
