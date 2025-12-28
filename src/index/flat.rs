use parking_lot::RwLock;
use std::collections::HashSet;

use crate::distance::Distance;
use crate::error::Result;
use crate::storage::Storage;
use crate::types::SearchResult;

use super::Index;

/// Flat index that performs brute-force exact search
pub struct FlatIndex {
    /// Set of indexed vector IDs
    ids: RwLock<HashSet<String>>,
}

impl FlatIndex {
    pub fn new() -> Self {
        Self {
            ids: RwLock::new(HashSet::new()),
        }
    }
}

impl Default for FlatIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl Index for FlatIndex {
    fn add(
        &self,
        id: &str,
        _vector: &[f32],
        _storage: &dyn Storage,
        _distance: Distance,
    ) -> Result<()> {
        // FlatIndex doesn't build a graph, so storage/distance are unused
        self.ids.write().insert(id.to_string());
        Ok(())
    }

    fn remove(&self, id: &str) -> Result<bool> {
        Ok(self.ids.write().remove(id))
    }

    fn search(
        &self,
        query: &[f32],
        k: usize,
        storage: &dyn Storage,
        distance: Distance,
    ) -> Result<Vec<SearchResult>> {
        // Collect distances only from documents that have vectors
        let mut results: Vec<SearchResult> = storage
            .iter_with_vectors()
            .filter_map(|stored| {
                // Only process documents with vectors
                stored.vector.as_ref().map(|vec| {
                    let dist = distance.calculate(query, vec);
                    SearchResult {
                        id: stored.id,
                        distance: dist,
                        metadata: stored.metadata,
                    }
                })
            })
            .collect();

        // Sort by distance (ascending - smaller is better)
        results.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());

        // Take top k
        results.truncate(k);

        Ok(results)
    }

    fn rebuild(&self, storage: &dyn Storage) -> Result<()> {
        let mut ids = self.ids.write();
        ids.clear();
        for id in storage.ids() {
            ids.insert(id);
        }
        Ok(())
    }

    fn len(&self) -> usize {
        self.ids.read().len()
    }

    fn clear(&self) {
        self.ids.write().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::MemoryStorage;

    #[test]
    fn test_flat_index_search() {
        let storage = MemoryStorage::new();
        let index = FlatIndex::new();

        // Insert some vectors
        let vectors = vec![
            ("a", vec![1.0, 0.0, 0.0]),
            ("b", vec![0.0, 1.0, 0.0]),
            ("c", vec![0.0, 0.0, 1.0]),
            ("d", vec![1.0, 1.0, 0.0]),
        ];

        for (id, data) in &vectors {
            storage.insert(id.to_string(), Some(data.clone()), None).unwrap();
            index.add(id, data, &storage, Distance::Euclidean).unwrap();
        }

        // Search for vector closest to [1, 0, 0]
        let query = vec![1.0, 0.0, 0.0];
        let results = index.search(&query, 2, &storage, Distance::Euclidean).unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "a"); // Exact match
        assert!((results[0].distance - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_flat_index_cosine() {
        let storage = MemoryStorage::new();
        let index = FlatIndex::new();

        // Insert normalized vectors
        let vectors = vec![
            ("a", vec![1.0, 0.0]),
            ("b", vec![0.707, 0.707]), // ~45 degrees
            ("c", vec![0.0, 1.0]),     // 90 degrees
        ];

        for (id, data) in &vectors {
            storage.insert(id.to_string(), Some(data.clone()), None).unwrap();
            index.add(id, data, &storage, Distance::Cosine).unwrap();
        }

        let query = vec![1.0, 0.0];
        let results = index.search(&query, 3, &storage, Distance::Cosine).unwrap();

        // Order should be: a (0°), b (45°), c (90°)
        assert_eq!(results[0].id, "a");
        assert_eq!(results[1].id, "b");
        assert_eq!(results[2].id, "c");
    }
}
