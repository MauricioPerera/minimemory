mod bm25;
mod flat;
mod hnsw;

pub use bm25::{BM25Index, BM25SearchResult, BM25Stats};
pub use flat::FlatIndex;
pub use hnsw::HNSWIndex;

use serde::{Deserialize, Serialize};

use crate::distance::Distance;
use crate::error::Result;
use crate::storage::Storage;
use crate::types::SearchResult;

/// Type of index to use for similarity search
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum IndexType {
    /// Flat index with brute-force search (exact results)
    #[default]
    Flat,
    /// HNSW index for approximate nearest neighbor search
    HNSW {
        /// Number of connections per node (default: 16)
        m: usize,
        /// Size of dynamic candidate list during construction (default: 200)
        ef_construction: usize,
    },
}

impl IndexType {
    /// Create a new HNSW index with default parameters
    pub fn hnsw() -> Self {
        IndexType::HNSW {
            m: 16,
            ef_construction: 200,
        }
    }

    /// Create a new HNSW index with custom parameters
    pub fn hnsw_with_params(m: usize, ef_construction: usize) -> Self {
        IndexType::HNSW { m, ef_construction }
    }
}

/// Trait for vector indices
pub trait Index: Send + Sync {
    /// Add a vector to the index
    ///
    /// For indices that need to compute distances during insertion (like HNSW),
    /// storage and distance parameters are provided to access other vectors.
    fn add(
        &self,
        id: &str,
        vector: &[f32],
        storage: &dyn Storage,
        distance: Distance,
    ) -> Result<()>;

    /// Remove a vector from the index
    fn remove(&self, id: &str) -> Result<bool>;

    /// Search for the k nearest neighbors
    fn search(
        &self,
        query: &[f32],
        k: usize,
        storage: &dyn Storage,
        distance: Distance,
    ) -> Result<Vec<SearchResult>>;

    /// Rebuild the index from storage
    fn rebuild(&self, storage: &dyn Storage) -> Result<()>;

    /// Get the number of indexed vectors
    fn len(&self) -> usize;

    /// Check if index is empty
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clear the index
    fn clear(&self);
}
