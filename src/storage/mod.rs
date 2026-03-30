pub mod disk;
pub mod format;
mod memory;

pub use memory::MemoryStorage;

use crate::error::Result;
use crate::quantization::QuantizedVector;
use crate::types::{Metadata, StoredVector, VectorId};

/// Trait for document storage backends.
///
/// Supports both vectorized documents (with embeddings) and
/// metadata-only documents (for hybrid database use cases).
pub trait Storage: Send + Sync {
    /// Insert a document into storage.
    ///
    /// # Arguments
    /// * `id` - Unique identifier for the document
    /// * `vector` - Optional vector embedding (None for metadata-only documents)
    /// * `metadata` - Optional metadata associated with the document
    fn insert(
        &self,
        id: VectorId,
        vector: Option<Vec<f32>>,
        metadata: Option<Metadata>,
    ) -> Result<()>;

    /// Get a document by ID
    fn get(&self, id: &str) -> Result<Option<StoredVector>>;

    /// Delete a document by ID
    fn delete(&self, id: &str) -> Result<bool>;

    /// Check if a document exists
    fn contains(&self, id: &str) -> bool;

    /// Get the number of stored documents
    fn len(&self) -> usize;

    /// Check if storage is empty
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get all documents (with or without vectors)
    fn iter(&self) -> Box<dyn Iterator<Item = StoredVector> + '_>;

    /// Get only documents that have vectors (for vector indexing)
    fn iter_with_vectors(&self) -> Box<dyn Iterator<Item = StoredVector> + '_>;

    /// Get all document IDs
    fn ids(&self) -> Vec<VectorId>;

    /// Clear all documents
    fn clear(&self);

    /// Insert a document with a quantized vector (replacing the f32 vector).
    /// The f32 vector is discarded; only the quantized form is stored.
    fn insert_quantized(
        &self,
        id: VectorId,
        quantized: QuantizedVector,
        metadata: Option<Metadata>,
    ) -> Result<()>;

    /// Get the quantized vector for a document, if stored.
    fn get_quantized(&self, id: &str) -> Result<Option<QuantizedVector>>;
}
