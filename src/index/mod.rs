mod flat;
mod hnsw;
mod bm25;

pub use flat::FlatIndex;
pub use hnsw::HNSWIndex;
pub use bm25::{BM25Index, BM25SearchResult, BM25Stats};

use serde::{Deserialize, Serialize};

use crate::distance::Distance;
use crate::error::Result;
use crate::storage::Storage;
use crate::types::SearchResult;

/// Type of index to use for similarity search
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IndexType {
    /// Flat index with brute-force search (exact results)
    Flat,
    /// HNSW index for approximate nearest neighbor search
    HNSW {
        /// Number of connections per node (default: 16)
        m: usize,
        /// Size of dynamic candidate list during construction (default: 200)
        ef_construction: usize,
    },
}

impl Default for IndexType {
    fn default() -> Self {
        IndexType::Flat
    }
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
    fn add(&self, id: &str, vector: &[f32]) -> Result<()>;

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
