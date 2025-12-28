use parking_lot::RwLock;
use std::collections::HashMap;

use crate::error::Result;
use crate::types::{Metadata, StoredVector, VectorId};

use super::Storage;

/// In-memory storage backend using a HashMap
pub struct MemoryStorage {
    vectors: RwLock<HashMap<VectorId, StoredVector>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            vectors: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl Storage for MemoryStorage {
    fn insert(
        &self,
        id: VectorId,
        vector: Option<Vec<f32>>,
        metadata: Option<Metadata>,
    ) -> Result<()> {
        let doc = StoredVector {
            id: id.clone(),
            vector,
            metadata,
        };
        self.vectors.write().insert(id, doc);
        Ok(())
    }

    fn get(&self, id: &str) -> Result<Option<StoredVector>> {
        Ok(self.vectors.read().get(id).cloned())
    }

    fn delete(&self, id: &str) -> Result<bool> {
        Ok(self.vectors.write().remove(id).is_some())
    }

    fn contains(&self, id: &str) -> bool {
        self.vectors.read().contains_key(id)
    }

    fn len(&self) -> usize {
        self.vectors.read().len()
    }

    fn iter(&self) -> Box<dyn Iterator<Item = StoredVector> + '_> {
        let docs: Vec<StoredVector> = self.vectors.read().values().cloned().collect();
        Box::new(docs.into_iter())
    }

    fn iter_with_vectors(&self) -> Box<dyn Iterator<Item = StoredVector> + '_> {
        let docs: Vec<StoredVector> = self
            .vectors
            .read()
            .values()
            .filter(|doc| doc.vector.is_some())
            .cloned()
            .collect();
        Box::new(docs.into_iter())
    }

    fn ids(&self) -> Vec<VectorId> {
        self.vectors.read().keys().cloned().collect()
    }

    fn clear(&self) {
        self.vectors.write().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_get() {
        let storage = MemoryStorage::new();
        let id = "test-1".to_string();
        let data = vec![1.0, 2.0, 3.0];

        storage
            .insert(id.clone(), Some(data.clone()), None)
            .unwrap();

        let result = storage.get(&id).unwrap().unwrap();
        assert_eq!(result.id, id);
        assert_eq!(result.vector, Some(data));
    }

    #[test]
    fn test_insert_metadata_only() {
        let storage = MemoryStorage::new();
        let id = "doc-1".to_string();
        let mut meta = Metadata::new();
        meta.insert("title", "Test Document");

        storage.insert(id.clone(), None, Some(meta)).unwrap();

        let result = storage.get(&id).unwrap().unwrap();
        assert_eq!(result.id, id);
        assert!(result.vector.is_none());
        assert!(result.metadata.is_some());
    }

    #[test]
    fn test_iter_with_vectors() {
        let storage = MemoryStorage::new();

        // Document with vector
        storage
            .insert("vec-1".to_string(), Some(vec![1.0]), None)
            .unwrap();
        // Document without vector (metadata only)
        storage.insert("doc-1".to_string(), None, None).unwrap();
        // Another document with vector
        storage
            .insert("vec-2".to_string(), Some(vec![2.0]), None)
            .unwrap();

        let with_vectors: Vec<_> = storage.iter_with_vectors().collect();
        assert_eq!(with_vectors.len(), 2);

        let all: Vec<_> = storage.iter().collect();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_delete() {
        let storage = MemoryStorage::new();
        let id = "test-1".to_string();

        storage.insert(id.clone(), Some(vec![1.0]), None).unwrap();
        assert!(storage.contains(&id));

        let deleted = storage.delete(&id).unwrap();
        assert!(deleted);
        assert!(!storage.contains(&id));
    }

    #[test]
    fn test_len_and_clear() {
        let storage = MemoryStorage::new();

        storage
            .insert("a".to_string(), Some(vec![1.0]), None)
            .unwrap();
        storage
            .insert("b".to_string(), Some(vec![2.0]), None)
            .unwrap();
        assert_eq!(storage.len(), 2);

        storage.clear();
        assert_eq!(storage.len(), 0);
        assert!(storage.is_empty());
    }
}
