//! # Índices Parciales
//!
//! Permite crear índices sobre subconjuntos de documentos basados en filtros.
//! Útil para optimizar consultas frecuentes sobre categorías específicas.
//!
//! ## Beneficios
//!
//! - **Menor uso de memoria**: Solo indexa documentos relevantes
//! - **Búsquedas más rápidas**: Índices más pequeños = menos comparaciones
//! - **Especialización**: Índices optimizados para patrones de consulta específicos
//!
//! ## Ejemplo
//!
//! ```rust,ignore
//! use minimemory::{VectorDB, Config, Filter};
//! use minimemory::partial_index::PartialIndexConfig;
//!
//! let db = VectorDB::new(Config::new(384)).unwrap();
//!
//! // Crear índice parcial para documentos de categoría "tech"
//! db.create_partial_index(
//!     "tech_docs",
//!     PartialIndexConfig::new(Filter::eq("category", "tech"))
//! ).unwrap();
//!
//! // Buscar solo en el índice parcial (más rápido)
//! let results = db.search_partial("tech_docs", &query_vector, 10).unwrap();
//! ```

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::distance::Distance;
use crate::error::{Error, Result};
use crate::index::{FlatIndex, HNSWIndex, Index, IndexType};
use crate::query::{Filter, FilterEvaluator};
use crate::storage::{MemoryStorage, Storage};
use crate::types::{Metadata, VectorId};

/// Configuración para un índice parcial.
#[derive(Debug, Clone)]
pub struct PartialIndexConfig {
    /// Filtro que determina qué documentos incluir en el índice
    pub filter: Filter,
    /// Tipo de índice a usar (Flat o HNSW)
    pub index_type: IndexType,
    /// Métrica de distancia para búsquedas
    pub distance: Distance,
    /// Descripción opcional del índice
    pub description: Option<String>,
}

impl PartialIndexConfig {
    /// Crea una nueva configuración de índice parcial.
    ///
    /// # Argumentos
    ///
    /// * `filter` - Filtro que determina qué documentos incluir
    ///
    /// # Ejemplo
    ///
    /// ```rust,ignore
    /// use minimemory::Filter;
    /// use minimemory::partial_index::PartialIndexConfig;
    ///
    /// // Índice para documentos activos
    /// let config = PartialIndexConfig::new(Filter::eq("status", "active"));
    ///
    /// // Índice para documentos con score > 0.8
    /// let config = PartialIndexConfig::new(Filter::gt("score", 0.8f64));
    /// ```
    pub fn new(filter: Filter) -> Self {
        Self {
            filter,
            index_type: IndexType::Flat,
            distance: Distance::Cosine,
            description: None,
        }
    }

    /// Establece el tipo de índice.
    pub fn with_index_type(mut self, index_type: IndexType) -> Self {
        self.index_type = index_type;
        self
    }

    /// Establece la métrica de distancia.
    pub fn with_distance(mut self, distance: Distance) -> Self {
        self.distance = distance;
        self
    }

    /// Establece una descripción para el índice.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Crea un índice HNSW para el índice parcial.
    pub fn with_hnsw(mut self, m: usize, ef_construction: usize) -> Self {
        self.index_type = IndexType::HNSW { m, ef_construction };
        self
    }
}

/// Estadísticas de un índice parcial.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialIndexStats {
    /// Nombre del índice
    pub name: String,
    /// Número de documentos en el índice
    pub document_count: usize,
    /// Tipo de índice
    pub index_type: String,
    /// Descripción del índice
    pub description: Option<String>,
}

/// Un índice parcial que contiene un subconjunto de documentos.
pub struct PartialIndex {
    /// Nombre único del índice
    pub name: String,
    /// Configuración del índice
    pub config: PartialIndexConfig,
    /// Índice vectorial subyacente
    index: Arc<dyn Index>,
    /// Storage local para los vectores del índice parcial
    storage: MemoryStorage,
    /// IDs de documentos incluidos en este índice
    document_ids: RwLock<Vec<VectorId>>,
}

impl PartialIndex {
    /// Crea un nuevo índice parcial.
    pub fn new(name: impl Into<String>, config: PartialIndexConfig) -> Result<Self> {
        let index: Arc<dyn Index> = match &config.index_type {
            IndexType::Flat => Arc::new(FlatIndex::new()),
            IndexType::HNSW { m, ef_construction } => {
                Arc::new(HNSWIndex::new(*m, *ef_construction))
            }
        };

        Ok(Self {
            name: name.into(),
            config,
            index,
            storage: MemoryStorage::new(),
            document_ids: RwLock::new(Vec::new()),
        })
    }

    /// Verifica si un documento cumple con el filtro del índice.
    pub fn matches(&self, metadata: Option<&Metadata>) -> bool {
        FilterEvaluator::evaluate(&self.config.filter, metadata)
    }

    /// Añade un documento al índice si cumple con el filtro.
    ///
    /// # Retorna
    ///
    /// `true` si el documento fue añadido, `false` si no cumple el filtro.
    pub fn try_add(&self, id: &str, vector: &[f32], metadata: Option<&Metadata>) -> Result<bool> {
        if !self.matches(metadata) {
            return Ok(false);
        }

        // Almacenar en storage local y en el índice
        self.storage
            .insert(id.to_string(), Some(vector.to_vec()), metadata.cloned())?;
        self.index
            .add(id, vector, &self.storage, self.config.distance)?;
        self.document_ids.write().push(id.to_string());
        Ok(true)
    }

    /// Elimina un documento del índice.
    pub fn remove(&self, id: &str) -> Result<()> {
        self.storage.delete(id)?;
        self.index.remove(id)?;
        self.document_ids.write().retain(|doc_id| doc_id != id);
        Ok(())
    }

    /// Busca los k vectores más cercanos en este índice parcial.
    pub fn search(&self, query: &[f32], k: usize) -> Result<Vec<(VectorId, f32)>> {
        let results = self
            .index
            .search(query, k, &self.storage, self.config.distance)?;
        Ok(results.into_iter().map(|r| (r.id, r.distance)).collect())
    }

    /// Retorna el número de documentos en el índice.
    pub fn len(&self) -> usize {
        self.document_ids.read().len()
    }

    /// Verifica si el índice está vacío.
    pub fn is_empty(&self) -> bool {
        self.document_ids.read().is_empty()
    }

    /// Retorna los IDs de todos los documentos en el índice.
    pub fn document_ids(&self) -> Vec<VectorId> {
        self.document_ids.read().clone()
    }

    /// Retorna estadísticas del índice.
    pub fn stats(&self) -> PartialIndexStats {
        PartialIndexStats {
            name: self.name.clone(),
            document_count: self.len(),
            index_type: match self.config.index_type {
                IndexType::Flat => "Flat".to_string(),
                IndexType::HNSW { m, ef_construction } => {
                    format!("HNSW(m={}, ef={})", m, ef_construction)
                }
            },
            description: self.config.description.clone(),
        }
    }

    /// Reconstruye el índice con nuevos documentos.
    ///
    /// Útil después de cambios masivos o para optimización.
    pub fn rebuild<'a>(
        &self,
        documents: impl Iterator<Item = (&'a str, &'a [f32], Option<&'a Metadata>)>,
    ) -> Result<usize> {
        // Limpiar índice actual
        let old_ids = self.document_ids.read().clone();
        for id in &old_ids {
            let _ = self.index.remove(id);
            let _ = self.storage.delete(id);
        }
        self.document_ids.write().clear();

        // Reindexar documentos que coincidan
        let mut count = 0;
        for (id, vector, metadata) in documents {
            if self.try_add(id, vector, metadata)? {
                count += 1;
            }
        }

        Ok(count)
    }
}

/// Gestor de índices parciales.
///
/// Mantiene múltiples índices parciales y los actualiza automáticamente.
pub struct PartialIndexManager {
    /// Índices parciales por nombre
    indexes: RwLock<HashMap<String, Arc<PartialIndex>>>,
}

impl Default for PartialIndexManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialIndexManager {
    /// Crea un nuevo gestor de índices parciales.
    pub fn new() -> Self {
        Self {
            indexes: RwLock::new(HashMap::new()),
        }
    }

    /// Crea un nuevo índice parcial.
    pub fn create_index(&self, name: &str, config: PartialIndexConfig) -> Result<()> {
        let mut indexes = self.indexes.write();

        if indexes.contains_key(name) {
            return Err(Error::AlreadyExists(name.to_string()));
        }

        let index = PartialIndex::new(name, config)?;
        indexes.insert(name.to_string(), Arc::new(index));
        Ok(())
    }

    /// Elimina un índice parcial.
    pub fn drop_index(&self, name: &str) -> Result<()> {
        let mut indexes = self.indexes.write();

        if indexes.remove(name).is_none() {
            return Err(Error::NotFound(name.to_string()));
        }

        Ok(())
    }

    /// Obtiene un índice parcial por nombre.
    pub fn get_index(&self, name: &str) -> Option<Arc<PartialIndex>> {
        self.indexes.read().get(name).cloned()
    }

    /// Lista todos los índices parciales.
    pub fn list_indexes(&self) -> Vec<PartialIndexStats> {
        self.indexes
            .read()
            .values()
            .map(|idx| idx.stats())
            .collect()
    }

    /// Notifica la inserción de un nuevo documento.
    ///
    /// Añade el documento a todos los índices parciales que coincidan.
    pub fn on_insert(
        &self,
        id: &str,
        vector: &[f32],
        metadata: Option<&Metadata>,
    ) -> Result<Vec<String>> {
        let indexes = self.indexes.read();
        let mut added_to = Vec::new();

        for (name, index) in indexes.iter() {
            if index.try_add(id, vector, metadata)? {
                added_to.push(name.clone());
            }
        }

        Ok(added_to)
    }

    /// Notifica la eliminación de un documento.
    ///
    /// Elimina el documento de todos los índices parciales.
    pub fn on_delete(&self, id: &str) -> Result<()> {
        let indexes = self.indexes.read();

        for index in indexes.values() {
            let _ = index.remove(id); // Ignorar si no está en el índice
        }

        Ok(())
    }

    /// Notifica la actualización de un documento.
    ///
    /// Actualiza el documento en los índices parciales correspondientes.
    pub fn on_update(&self, id: &str, vector: &[f32], metadata: Option<&Metadata>) -> Result<()> {
        let indexes = self.indexes.read();

        for index in indexes.values() {
            // Primero eliminar del índice
            let _ = index.remove(id);
            // Luego intentar añadir (si cumple el nuevo filtro)
            let _ = index.try_add(id, vector, metadata)?;
        }

        Ok(())
    }

    /// Busca en un índice parcial específico.
    pub fn search(
        &self,
        index_name: &str,
        query: &[f32],
        k: usize,
    ) -> Result<Vec<(VectorId, f32)>> {
        let indexes = self.indexes.read();

        match indexes.get(index_name) {
            Some(index) => index.search(query, k),
            None => Err(Error::NotFound(index_name.to_string())),
        }
    }

    /// Retorna el número de índices parciales.
    pub fn len(&self) -> usize {
        self.indexes.read().len()
    }

    /// Verifica si no hay índices parciales.
    pub fn is_empty(&self) -> bool {
        self.indexes.read().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_partial_index_creation() {
        let config = PartialIndexConfig::new(Filter::eq("category", "tech"));
        let index = PartialIndex::new("tech_index", config).unwrap();

        assert_eq!(index.name, "tech_index");
        assert!(index.is_empty());
    }

    #[test]
    fn test_partial_index_filtering() {
        let config = PartialIndexConfig::new(Filter::eq("category", "tech"));
        let index = PartialIndex::new("tech_index", config).unwrap();

        // Documento que coincide
        let mut meta1 = Metadata::new();
        meta1.insert("category", "tech");
        let vector1 = vec![0.1, 0.2, 0.3];

        let added = index.try_add("doc1", &vector1, Some(&meta1)).unwrap();
        assert!(added);
        assert_eq!(index.len(), 1);

        // Documento que NO coincide
        let mut meta2 = Metadata::new();
        meta2.insert("category", "sports");
        let vector2 = vec![0.4, 0.5, 0.6];

        let added = index.try_add("doc2", &vector2, Some(&meta2)).unwrap();
        assert!(!added);
        assert_eq!(index.len(), 1);
    }

    #[test]
    fn test_partial_index_search() {
        let config = PartialIndexConfig::new(Filter::eq("category", "tech"));
        let index = PartialIndex::new("tech_index", config).unwrap();

        // Añadir documentos que coinciden
        let mut meta = Metadata::new();
        meta.insert("category", "tech");

        index
            .try_add("doc1", &[1.0, 0.0, 0.0], Some(&meta))
            .unwrap();
        index
            .try_add("doc2", &[0.0, 1.0, 0.0], Some(&meta))
            .unwrap();
        index
            .try_add("doc3", &[0.0, 0.0, 1.0], Some(&meta))
            .unwrap();

        // Buscar
        let results = index.search(&[1.0, 0.1, 0.0], 2).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "doc1"); // Más cercano
    }

    #[test]
    fn test_partial_index_manager() {
        let manager = PartialIndexManager::new();

        // Crear índice
        let config = PartialIndexConfig::new(Filter::eq("type", "article"))
            .with_description("Artículos indexados");
        manager.create_index("articles", config).unwrap();

        assert_eq!(manager.len(), 1);

        // Insertar documento
        let mut meta = Metadata::new();
        meta.insert("type", "article");
        meta.insert("title", "Test Article");

        let added_to = manager
            .on_insert("art1", &[0.1, 0.2, 0.3], Some(&meta))
            .unwrap();
        assert_eq!(added_to, vec!["articles"]);

        // Buscar
        let results = manager.search("articles", &[0.1, 0.2, 0.3], 10).unwrap();
        assert_eq!(results.len(), 1);

        // Eliminar índice
        manager.drop_index("articles").unwrap();
        assert!(manager.is_empty());
    }

    #[test]
    fn test_partial_index_with_complex_filter() {
        let config = PartialIndexConfig::new(Filter::all(vec![
            Filter::eq("category", "tech"),
            Filter::gt("score", 0.5f64),
        ]));
        let index = PartialIndex::new("high_score_tech", config).unwrap();

        // Documento que cumple ambas condiciones
        let mut meta1 = Metadata::new();
        meta1.insert("category", "tech");
        meta1.insert("score", 0.8f64);
        assert!(index.try_add("doc1", &[0.1; 3], Some(&meta1)).unwrap());

        // Documento que solo cumple una condición
        let mut meta2 = Metadata::new();
        meta2.insert("category", "tech");
        meta2.insert("score", 0.3f64);
        assert!(!index.try_add("doc2", &[0.2; 3], Some(&meta2)).unwrap());

        assert_eq!(index.len(), 1);
    }
}
