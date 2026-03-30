//! Base de datos vectorial principal.

use std::path::Path;
use std::sync::Arc;

use crate::distance::Distance;
use crate::error::{Error, Result};
use crate::index::{BM25Index, FlatIndex, HNSWIndex, IVFIndex, Index, IndexType};
use crate::partial_index::{PartialIndexConfig, PartialIndexManager, PartialIndexStats};
use crate::quantization::{QuantizationType, Quantizer};
use crate::query::{Filter, OrderBy};
use crate::search::{HybridSearch, HybridSearchParams};
use crate::storage::{disk, format::FileHeader, MemoryStorage, Storage};
use crate::types::{Config, HybridSearchResult, Metadata, PagedResult, SearchResult, VectorId};

/// Base de datos vectorial embebida.
///
/// `VectorDB` es la interfaz principal para interactuar con la base de datos.
/// Permite insertar, buscar, actualizar y eliminar vectores con sus metadatos.
///
/// # Características
///
/// - **Thread-safe**: Puede usarse desde múltiples hilos
/// - **Búsqueda rápida**: Encuentra los k vecinos más cercanos
/// - **Metadata flexible**: Asocia información adicional a cada vector
///
/// # Ejemplo
///
/// ```rust
/// use minimemory::{VectorDB, Config, Distance};
///
/// let db = VectorDB::new(Config::new(3)).unwrap();
///
/// // Insertar vectores
/// db.insert("vec1", &[1.0, 0.0, 0.0], None).unwrap();
/// db.insert("vec2", &[0.0, 1.0, 0.0], None).unwrap();
///
/// // Buscar similares
/// let results = db.search(&[1.0, 0.1, 0.0], 1).unwrap();
/// assert_eq!(results[0].id, "vec1");
/// ```
pub struct VectorDB {
    config: Config,
    storage: Arc<dyn Storage>,
    index: Arc<dyn Index>,
    /// Índice BM25 para full-text search (opcional)
    bm25_index: Option<Arc<BM25Index>>,
    /// Campos indexados por BM25
    bm25_fields: Vec<String>,
    /// Gestor de índices parciales
    partial_indexes: PartialIndexManager,
    /// Quantizer for vector compression (None when QuantizationType::None)
    quantizer: Option<Quantizer>,
}

impl VectorDB {
    /// Crea una nueva base de datos vectorial en memoria.
    ///
    /// # Argumentos
    ///
    /// * `config` - Configuración con dimensiones, métrica y tipo de índice
    ///
    /// # Ejemplo
    ///
    /// ```rust
    /// use minimemory::{VectorDB, Config};
    ///
    /// let db = VectorDB::new(Config::new(128)).unwrap();
    /// ```
    pub fn new(config: Config) -> Result<Self> {
        let storage = Arc::new(MemoryStorage::new());
        let index = Self::create_index(&config.index)?;
        let quantizer = Self::create_quantizer(&config);

        Ok(Self {
            config,
            storage,
            index,
            bm25_index: None,
            bm25_fields: Vec::new(),
            partial_indexes: PartialIndexManager::new(),
            quantizer,
        })
    }

    /// Crea una nueva base de datos con soporte para full-text search.
    ///
    /// # Argumentos
    ///
    /// * `config` - Configuración con dimensiones, métrica y tipo de índice
    /// * `indexed_fields` - Campos de metadata a indexar para BM25 (ej: ["title", "content"])
    ///
    /// # Ejemplo
    ///
    /// ```rust
    /// use minimemory::{VectorDB, Config};
    ///
    /// let db = VectorDB::with_fulltext(
    ///     Config::new(384),
    ///     vec!["title".into(), "content".into()]
    /// ).unwrap();
    /// ```
    pub fn with_fulltext(config: Config, indexed_fields: Vec<String>) -> Result<Self> {
        let storage = Arc::new(MemoryStorage::new());
        let index = Self::create_index(&config.index)?;
        let bm25_index = Arc::new(BM25Index::new(indexed_fields.clone()));
        let quantizer = Self::create_quantizer(&config);

        Ok(Self {
            config,
            storage,
            index,
            bm25_index: Some(bm25_index),
            bm25_fields: indexed_fields,
            partial_indexes: PartialIndexManager::new(),
            quantizer,
        })
    }

    /// Abre una base de datos desde un archivo.
    ///
    /// Carga los vectores del archivo a memoria y reconstruye el índice.
    ///
    /// # Argumentos
    ///
    /// * `path` - Ruta al archivo .mmdb
    ///
    /// # Ejemplo
    ///
    /// ```rust,no_run
    /// use minimemory::VectorDB;
    ///
    /// let db = VectorDB::open("my_database.mmdb").unwrap();
    /// ```
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let (header, vectors, index_blocks) = disk::load_vectors(path)?;

        let config = Config {
            dimensions: header.dimensions as usize,
            distance: header.get_distance(),
            index: header.get_index_type(),
            quantization: header.get_quantization_type(),
        };

        let storage = Arc::new(MemoryStorage::new());
        let index = Self::create_index(&config.index)?;
        let quantizer = Self::create_quantizer(&config);

        // Load documents into storage (preserving quantized vectors)
        for stored in &vectors {
            if stored.quantized.is_some() {
                storage.insert_quantized(
                    stored.id.clone(),
                    stored.quantized.clone().unwrap(),
                    stored.metadata.clone(),
                )?;
            } else {
                storage.insert(
                    stored.id.clone(),
                    stored.vector.clone(),
                    stored.metadata.clone(),
                )?;
            }
        }

        // Try to load serialized HNSW index (v2+), fall back to rebuilding
        let need_rebuild = if let Some(data) = index_blocks.hnsw {
            index.load_index(&data).is_err()
        } else {
            true
        };

        if need_rebuild {
            // Rebuild index from vectors (dequantizing if needed)
            for stored in &vectors {
                let vec_data = stored
                    .vector
                    .as_ref()
                    .or_else(|| None)
                    .cloned()
                    .or_else(|| stored.quantized.as_ref().map(|q| q.to_f32()));
                if let Some(vec) = vec_data {
                    index.add(&stored.id, &vec, &*storage, config.distance)?;
                }
            }
        }

        Ok(Self {
            config,
            storage,
            index,
            bm25_index: None,
            bm25_fields: Vec::new(),
            partial_indexes: PartialIndexManager::new(),
            quantizer,
        })
    }

    /// Abre una base de datos con soporte full-text.
    ///
    /// Carga los vectores y reconstruye el índice BM25.
    ///
    /// # Argumentos
    ///
    /// * `path` - Ruta al archivo .mmdb
    /// * `indexed_fields` - Campos de metadata a indexar para BM25
    pub fn open_with_fulltext<P: AsRef<Path>>(
        path: P,
        indexed_fields: Vec<String>,
    ) -> Result<Self> {
        let (header, vectors, index_blocks) = disk::load_vectors(path)?;

        let config = Config {
            dimensions: header.dimensions as usize,
            distance: header.get_distance(),
            index: header.get_index_type(),
            quantization: header.get_quantization_type(),
        };

        let storage = Arc::new(MemoryStorage::new());
        let index = Self::create_index(&config.index)?;
        let quantizer = Self::create_quantizer(&config);

        // Load documents into storage (preserving quantized vectors)
        for stored in &vectors {
            if stored.quantized.is_some() {
                storage.insert_quantized(
                    stored.id.clone(),
                    stored.quantized.clone().unwrap(),
                    stored.metadata.clone(),
                )?;
            } else {
                storage.insert(
                    stored.id.clone(),
                    stored.vector.clone(),
                    stored.metadata.clone(),
                )?;
            }
        }

        // Try to load serialized HNSW index, fall back to rebuilding
        let need_rebuild = if let Some(data) = index_blocks.hnsw {
            index.load_index(&data).is_err()
        } else {
            true
        };

        if need_rebuild {
            for stored in &vectors {
                let vec_data = stored
                    .vector
                    .as_ref()
                    .cloned()
                    .or_else(|| stored.quantized.as_ref().map(|q| q.to_f32()));
                if let Some(vec) = vec_data {
                    index.add(&stored.id, &vec, &*storage, config.distance)?;
                }
            }
        }

        // Try to load persisted BM25 index, fall back to rebuilding
        let bm25_index = if let Some(data) = index_blocks.bm25 {
            match BM25Index::deserialize(indexed_fields.clone(), &data) {
                Ok(idx) => Arc::new(idx),
                Err(_) => {
                    let idx = Arc::new(BM25Index::new(indexed_fields.clone()));
                    for stored in &vectors {
                        idx.add(&stored.id, stored.metadata.as_ref())?;
                    }
                    idx
                }
            }
        } else {
            let idx = Arc::new(BM25Index::new(indexed_fields.clone()));
            for stored in &vectors {
                idx.add(&stored.id, stored.metadata.as_ref())?;
            }
            idx
        };

        Ok(Self {
            config,
            storage,
            index,
            bm25_index: Some(bm25_index),
            bm25_fields: indexed_fields,
            partial_indexes: PartialIndexManager::new(),
            quantizer,
        })
    }

    /// Crea un índice basado en la configuración.
    fn create_index(index_type: &IndexType) -> Result<Arc<dyn Index>> {
        match index_type {
            IndexType::Flat => Ok(Arc::new(FlatIndex::new())),
            IndexType::HNSW { m, ef_construction } => {
                Ok(Arc::new(HNSWIndex::new(*m, *ef_construction)))
            }
            IndexType::IVF {
                num_clusters,
                num_probes,
            } => Ok(Arc::new(IVFIndex::new(*num_clusters, *num_probes))),
        }
    }

    /// Creates a quantizer from config (None if no quantization).
    fn create_quantizer(config: &Config) -> Option<Quantizer> {
        match config.quantization {
            QuantizationType::None => None,
            QuantizationType::Int8 => Some(Quantizer::int8(config.dimensions)),
            QuantizationType::Int3 => Some(Quantizer::int3(config.dimensions)),
            QuantizationType::Binary => Some(Quantizer::binary(config.dimensions)),
            QuantizationType::Polar => Some(Quantizer::polar(config.dimensions)),
        }
    }

    /// Inserta un vector con ID y metadata opcional.
    ///
    /// # Argumentos
    ///
    /// * `id` - Identificador único del vector
    /// * `vector` - Slice de f32 con las dimensiones correctas
    /// * `metadata` - Metadata opcional asociada al vector
    ///
    /// # Errores
    ///
    /// - `DimensionMismatch`: Si el vector no tiene las dimensiones configuradas
    /// - `AlreadyExists`: Si ya existe un vector con ese ID
    ///
    /// # Ejemplo
    ///
    /// ```rust
    /// use minimemory::{VectorDB, Config, Metadata};
    ///
    /// let db = VectorDB::new(Config::new(3)).unwrap();
    ///
    /// // Sin metadata
    /// db.insert("vec1", &[1.0, 2.0, 3.0], None).unwrap();
    ///
    /// // Con metadata
    /// let mut meta = Metadata::new();
    /// meta.insert("label", "importante");
    /// db.insert("vec2", &[4.0, 5.0, 6.0], Some(meta)).unwrap();
    /// ```
    pub fn insert(
        &self,
        id: impl Into<VectorId>,
        vector: &[f32],
        metadata: Option<Metadata>,
    ) -> Result<()> {
        let id = id.into();

        if vector.len() != self.config.dimensions {
            return Err(Error::DimensionMismatch {
                expected: self.config.dimensions,
                got: vector.len(),
            });
        }

        if self.storage.contains(&id) {
            return Err(Error::AlreadyExists(id));
        }

        // Store quantized or full vector
        if let Some(ref quantizer) = self.quantizer {
            let qvec = quantizer.quantize(vector)?;
            self.storage
                .insert_quantized(id.clone(), qvec, metadata.clone())?;
        } else {
            self.storage
                .insert(id.clone(), Some(vector.to_vec()), metadata.clone())?;
        }

        // Index always uses f32 for graph construction (HNSW needs precise distances)
        self.index
            .add(&id, vector, &*self.storage, self.config.distance)?;

        // Indexar en BM25 si está habilitado
        if let Some(ref bm25) = self.bm25_index {
            bm25.add(&id, metadata.as_ref())?;
        }

        // Añadir a índices parciales que coincidan
        let _ = self
            .partial_indexes
            .on_insert(&id, vector, metadata.as_ref());

        Ok(())
    }

    /// Inserta un documento con vector opcional.
    ///
    /// Permite insertar documentos solo con metadata (sin vector),
    /// útil para almacenar datos como en MongoDB.
    ///
    /// # Argumentos
    ///
    /// * `id` - Identificador único del documento
    /// * `vector` - Vector opcional (None para documentos metadata-only)
    /// * `metadata` - Metadata del documento
    ///
    /// # Ejemplo
    ///
    /// ```rust
    /// use minimemory::{VectorDB, Config, Metadata};
    ///
    /// let db = VectorDB::with_fulltext(
    ///     Config::new(3),
    ///     vec!["title".into(), "content".into()]
    /// ).unwrap();
    ///
    /// // Documento sin vector (solo metadata)
    /// let mut meta = Metadata::new();
    /// meta.insert("title", "Mi post de blog");
    /// meta.insert("content", "Contenido del post...");
    /// db.insert_document("post-1", None, Some(meta)).unwrap();
    ///
    /// // Documento con vector
    /// let mut meta2 = Metadata::new();
    /// meta2.insert("title", "Post con embedding");
    /// db.insert_document("post-2", Some(&[0.1, 0.2, 0.3]), Some(meta2)).unwrap();
    /// ```
    pub fn insert_document(
        &self,
        id: impl Into<VectorId>,
        vector: Option<&[f32]>,
        metadata: Option<Metadata>,
    ) -> Result<()> {
        let id = id.into();

        // Validar dimensiones si hay vector
        if let Some(vec) = vector {
            if vec.len() != self.config.dimensions {
                return Err(Error::DimensionMismatch {
                    expected: self.config.dimensions,
                    got: vec.len(),
                });
            }
        }

        if self.storage.contains(&id) {
            return Err(Error::AlreadyExists(id));
        }

        // Store with quantization if vector present and quantizer active
        if let (Some(vec), Some(ref quantizer)) = (vector, &self.quantizer) {
            let qvec = quantizer.quantize(vec)?;
            self.storage
                .insert_quantized(id.clone(), qvec, metadata.clone())?;
        } else {
            let vec_data = vector.map(|v| v.to_vec());
            self.storage
                .insert(id.clone(), vec_data, metadata.clone())?;
        }

        // Solo indexar en índice vectorial si hay vector
        if let Some(vec) = vector {
            self.index
                .add(&id, vec, &*self.storage, self.config.distance)?;
            // Añadir a índices parciales que coincidan
            let _ = self.partial_indexes.on_insert(&id, vec, metadata.as_ref());
        }

        // Indexar en BM25 si está habilitado
        if let Some(ref bm25) = self.bm25_index {
            bm25.add(&id, metadata.as_ref())?;
        }

        Ok(())
    }

    /// Inserta múltiples vectores en lote.
    ///
    /// # Argumentos
    ///
    /// * `vectors` - Slice de tuplas (id, vector, metadata)
    ///
    /// # Nota
    ///
    /// Si alguna inserción falla, las anteriores no se revierten.
    pub fn insert_batch(
        &self,
        vectors: &[(impl AsRef<str>, &[f32], Option<Metadata>)],
    ) -> Result<()> {
        for (id, vector, metadata) in vectors {
            self.insert(id.as_ref(), vector, metadata.clone())?;
        }
        Ok(())
    }

    /// Busca los k vecinos más cercanos al vector de consulta.
    ///
    /// # Argumentos
    ///
    /// * `query` - Vector de consulta
    /// * `k` - Número de resultados a retornar
    ///
    /// # Retorna
    ///
    /// Vector de `SearchResult` ordenado por distancia (menor primero).
    ///
    /// # Errores
    ///
    /// - `DimensionMismatch`: Si el query no tiene las dimensiones correctas
    ///
    /// # Ejemplo
    ///
    /// ```rust
    /// use minimemory::{VectorDB, Config};
    ///
    /// let db = VectorDB::new(Config::new(3)).unwrap();
    /// db.insert("a", &[1.0, 0.0, 0.0], None).unwrap();
    /// db.insert("b", &[0.0, 1.0, 0.0], None).unwrap();
    ///
    /// let results = db.search(&[1.0, 0.1, 0.0], 2).unwrap();
    /// println!("Más cercano: {}", results[0].id);
    /// ```
    pub fn search(&self, query: &[f32], k: usize) -> Result<Vec<SearchResult>> {
        if query.len() != self.config.dimensions {
            return Err(Error::DimensionMismatch {
                expected: self.config.dimensions,
                got: query.len(),
            });
        }

        if self.storage.is_empty() {
            return Ok(vec![]);
        }

        self.index
            .search(query, k, self.storage.as_ref(), self.config.distance)
    }

    /// Obtiene un documento por su ID.
    ///
    /// # Retorna
    ///
    /// `Some((vector, metadata))` si existe, `None` si no.
    /// El vector puede ser `None` para documentos metadata-only.
    pub fn get(&self, id: &str) -> Result<Option<(Option<Vec<f32>>, Option<Metadata>)>> {
        match self.storage.get(id)? {
            Some(stored) => {
                // If vector is stored quantized, dequantize for the caller
                let vector = stored.vector.or_else(|| {
                    stored.quantized.as_ref().map(|q| q.to_f32())
                });
                Ok(Some((vector, stored.metadata)))
            }
            None => Ok(None),
        }
    }

    /// Elimina un vector por su ID.
    ///
    /// # Retorna
    ///
    /// `true` si el vector existía y fue eliminado, `false` si no existía.
    pub fn delete(&self, id: &str) -> Result<bool> {
        let deleted = self.storage.delete(id)?;
        if deleted {
            self.index.remove(id)?;
            // Remover de BM25 si está habilitado
            if let Some(ref bm25) = self.bm25_index {
                bm25.remove(id)?;
            }
            // Remover de índices parciales
            let _ = self.partial_indexes.on_delete(id);
        }
        Ok(deleted)
    }

    /// Actualiza un documento existente.
    ///
    /// Internamente elimina el documento anterior e inserta el nuevo.
    ///
    /// # Argumentos
    ///
    /// * `id` - ID del documento a actualizar
    /// * `vector` - Nuevo vector
    /// * `metadata` - Nueva metadata (reemplaza la anterior)
    pub fn update(
        &self,
        id: impl Into<VectorId>,
        vector: &[f32],
        metadata: Option<Metadata>,
    ) -> Result<()> {
        let id = id.into();

        // Step 1: Update storage in-place (overwrite, no gap)
        // First delete old entry, then immediately insert new one
        self.storage.delete(&id)?;
        if let Some(ref quantizer) = self.quantizer {
            let qvec = quantizer.quantize(vector)?;
            self.storage
                .insert_quantized(id.clone(), qvec, metadata.clone())?;
        } else {
            self.storage
                .insert(id.clone(), Some(vector.to_vec()), metadata.clone())?;
        }

        // Step 2: Update index (remove old, add new)
        self.index.remove(&id)?;
        self.index
            .add(&id, vector, &*self.storage, self.config.distance)?;

        // Step 3: Update BM25
        if let Some(ref bm25) = self.bm25_index {
            bm25.remove(&id)?;
            bm25.add(&id, metadata.as_ref())?;
        }

        // Step 4: Update partial indexes
        let _ = self.partial_indexes.on_delete(&id);
        let _ = self
            .partial_indexes
            .on_insert(&id, vector, metadata.as_ref());

        Ok(())
    }

    /// Actualiza un documento con vector opcional.
    ///
    /// # Argumentos
    ///
    /// * `id` - ID del documento a actualizar
    /// * `vector` - Nuevo vector (opcional)
    /// * `metadata` - Nueva metadata (reemplaza la anterior)
    pub fn update_document(
        &self,
        id: impl Into<VectorId>,
        vector: Option<&[f32]>,
        metadata: Option<Metadata>,
    ) -> Result<()> {
        let id = id.into();
        self.delete(&id)?;
        self.insert_document(id, vector, metadata)
    }

    /// Verifica si un vector existe en la base de datos.
    pub fn contains(&self, id: &str) -> bool {
        self.storage.contains(id)
    }

    /// Retorna el número de vectores en la base de datos.
    pub fn len(&self) -> usize {
        self.storage.len()
    }

    /// Retorna todos los IDs de documentos.
    pub fn list_ids(&self) -> Result<Vec<VectorId>> {
        Ok(self.storage.ids())
    }

    /// Verifica si la base de datos está vacía.
    pub fn is_empty(&self) -> bool {
        self.storage.is_empty()
    }

    /// Elimina todos los vectores de la base de datos.
    pub fn clear(&self) {
        self.storage.clear();
        self.index.clear();
        if let Some(ref bm25) = self.bm25_index {
            bm25.clear();
        }
    }

    /// Guarda la base de datos a un archivo .mmdb.
    ///
    /// # Argumentos
    ///
    /// * `path` - Ruta donde guardar el archivo
    ///
    /// # Ejemplo
    ///
    /// ```rust,no_run
    /// use minimemory::{VectorDB, Config};
    ///
    /// let db = VectorDB::new(Config::new(3)).unwrap();
    /// db.insert("a", &[1.0, 2.0, 3.0], None).unwrap();
    /// db.save("my_database.mmdb").unwrap();
    /// ```
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let mut header = FileHeader::new(
            self.config.dimensions,
            self.storage.len(),
            self.config.distance,
            &self.config.index,
        )
        .with_quantization(self.config.quantization);

        // Serialize indices for persistence
        let hnsw_data = self.index.serialize_index()?;
        let bm25_data = self
            .bm25_index
            .as_ref()
            .map(|idx| idx.serialize())
            .transpose()?;

        let index_blocks = disk::IndexBlocks {
            hnsw: hnsw_data.as_deref(),
            bm25: bm25_data.as_deref(),
        };

        disk::save_vectors(path, &mut header, self.storage.iter(), &index_blocks)
    }

    /// Retorna las dimensiones configuradas.
    pub fn dimensions(&self) -> usize {
        self.config.dimensions
    }

    /// Retorna la métrica de distancia configurada.
    pub fn distance(&self) -> Distance {
        self.config.distance
    }

    /// Verifica si el full-text search está habilitado.
    pub fn has_fulltext(&self) -> bool {
        self.bm25_index.is_some()
    }

    /// Retorna los campos indexados para BM25.
    pub fn bm25_fields(&self) -> &[String] {
        &self.bm25_fields
    }

    // ==================== ÍNDICES PARCIALES ====================

    /// Crea un nuevo índice parcial.
    ///
    /// Un índice parcial solo incluye documentos que cumplen con el filtro especificado.
    /// Esto mejora el rendimiento de búsquedas sobre subconjuntos específicos de datos.
    ///
    /// # Argumentos
    ///
    /// * `name` - Nombre único para el índice
    /// * `config` - Configuración con filtro y tipo de índice
    ///
    /// # Ejemplo
    ///
    /// ```rust,ignore
    /// use minimemory::{VectorDB, Config, Filter};
    /// use minimemory::partial_index::PartialIndexConfig;
    ///
    /// let db = VectorDB::new(Config::new(384)).unwrap();
    ///
    /// // Crear índice parcial para documentos de categoría "tech"
    /// db.create_partial_index(
    ///     "tech_docs",
    ///     PartialIndexConfig::new(Filter::eq("category", "tech"))
    /// ).unwrap();
    ///
    /// // Crear índice HNSW para documentos activos
    /// db.create_partial_index(
    ///     "active_docs",
    ///     PartialIndexConfig::new(Filter::eq("status", "active"))
    ///         .with_hnsw(16, 200)
    /// ).unwrap();
    /// ```
    pub fn create_partial_index(&self, name: &str, config: PartialIndexConfig) -> Result<()> {
        self.partial_indexes.create_index(name, config)
    }

    /// Elimina un índice parcial.
    ///
    /// # Errores
    ///
    /// Retorna error si el índice no existe.
    pub fn drop_partial_index(&self, name: &str) -> Result<()> {
        self.partial_indexes.drop_index(name)
    }

    /// Lista todos los índices parciales y sus estadísticas.
    pub fn list_partial_indexes(&self) -> Vec<PartialIndexStats> {
        self.partial_indexes.list_indexes()
    }

    /// Busca en un índice parcial específico.
    ///
    /// Esta búsqueda es más rápida que buscar en todo el índice principal
    /// cuando se trabaja con subconjuntos de datos.
    ///
    /// # Argumentos
    ///
    /// * `index_name` - Nombre del índice parcial
    /// * `query` - Vector de consulta
    /// * `k` - Número máximo de resultados
    ///
    /// # Ejemplo
    ///
    /// ```rust,ignore
    /// // Buscar solo en documentos de tecnología (más rápido)
    /// let results = db.search_partial("tech_docs", &query_vector, 10).unwrap();
    ///
    /// for result in results {
    ///     println!("{}: {:.4}", result.id, result.distance);
    /// }
    /// ```
    pub fn search_partial(
        &self,
        index_name: &str,
        query: &[f32],
        k: usize,
    ) -> Result<Vec<SearchResult>> {
        // Validar dimensiones
        if query.len() != self.config.dimensions {
            return Err(Error::DimensionMismatch {
                expected: self.config.dimensions,
                got: query.len(),
            });
        }

        let results = self.partial_indexes.search(index_name, query, k)?;

        // Convertir a SearchResult con metadata
        let mut search_results = Vec::with_capacity(results.len());
        for (id, distance) in results {
            let metadata = self.storage.get(&id)?.and_then(|sv| sv.metadata);
            search_results.push(SearchResult {
                id,
                distance,
                metadata,
            });
        }

        Ok(search_results)
    }

    /// Reconstruye un índice parcial con los documentos actuales.
    ///
    /// Útil después de cambios masivos en los datos o para optimizar el índice.
    ///
    /// # Retorna
    ///
    /// Número de documentos añadidos al índice.
    pub fn rebuild_partial_index(&self, index_name: &str) -> Result<usize> {
        let index = self
            .partial_indexes
            .get_index(index_name)
            .ok_or_else(|| Error::NotFound(index_name.to_string()))?;

        // Obtener todos los documentos con vector
        let all_ids = self.storage.ids();
        let documents: Vec<_> = all_ids
            .iter()
            .filter_map(|id| {
                if let Ok(Some(sv)) = self.storage.get(id) {
                    // Use f32 vector, or dequantize from quantized if needed
                    let vector = sv
                        .vector
                        .or_else(|| sv.quantized.as_ref().map(|q| q.to_f32()));
                    vector.map(|vec| (id.clone(), vec, sv.metadata))
                } else {
                    None
                }
            })
            .collect();

        // Reconstruir
        let docs_iter = documents
            .iter()
            .map(|(id, vec, meta)| (id.as_str(), vec.as_slice(), meta.as_ref()));

        index.rebuild(docs_iter)
    }

    /// Verifica si existe un índice parcial con el nombre dado.
    pub fn has_partial_index(&self, name: &str) -> bool {
        self.partial_indexes.get_index(name).is_some()
    }

    // ==================== INTEGRACIÓN CON CHUNKING ====================

    /// Inserta un chunk de documento con su metadata.
    ///
    /// Método de conveniencia para insertar chunks generados por el módulo `chunking`.
    /// El contenido del chunk se almacena en el campo "content" de la metadata.
    ///
    /// # Argumentos
    ///
    /// * `chunk` - Chunk a insertar
    /// * `vector` - Vector embedding opcional para el chunk
    ///
    /// # Ejemplo
    ///
    /// ```rust,ignore
    /// use minimemory::{VectorDB, Config};
    /// use minimemory::chunking::{chunk_markdown, ChunkConfig};
    ///
    /// let db = VectorDB::with_fulltext(
    ///     Config::new(384),
    ///     vec!["content".into(), "heading".into()]
    /// ).unwrap();
    ///
    /// let result = chunk_markdown("# Title\nContent here", &ChunkConfig::default()).unwrap();
    ///
    /// for chunk in result.chunks {
    ///     // Generar embedding con tu modelo preferido
    ///     let embedding = generate_embedding(&chunk.content);
    ///     db.insert_chunk(&chunk, Some(&embedding)).unwrap();
    /// }
    /// ```
    pub fn insert_chunk(
        &self,
        chunk: &crate::chunking::Chunk,
        vector: Option<&[f32]>,
    ) -> Result<()> {
        // Validar dimensiones si hay vector
        if let Some(vec) = vector {
            if vec.len() != self.config.dimensions {
                return Err(Error::DimensionMismatch {
                    expected: self.config.dimensions,
                    got: vec.len(),
                });
            }
        }

        if self.storage.contains(&chunk.id) {
            return Err(Error::AlreadyExists(chunk.id.clone()));
        }

        // Construir metadata combinando la del chunk con el contenido
        let mut metadata = chunk.metadata.to_metadata();
        metadata.insert("content", chunk.content.as_str());

        // Store with quantization if vector present and quantizer active
        if let (Some(vec), Some(ref quantizer)) = (vector, &self.quantizer) {
            let qvec = quantizer.quantize(vec)?;
            self.storage
                .insert_quantized(chunk.id.clone(), qvec, Some(metadata.clone()))?;
        } else {
            let vec_data = vector.map(|v| v.to_vec());
            self.storage
                .insert(chunk.id.clone(), vec_data, Some(metadata.clone()))?;
        }

        // Solo indexar en índice vectorial si hay vector
        if let Some(vec) = vector {
            self.index
                .add(&chunk.id, vec, &*self.storage, self.config.distance)?;
            // Añadir a índices parciales
            let _ = self
                .partial_indexes
                .on_insert(&chunk.id, vec, Some(&metadata));
        }

        // Indexar en BM25 si está habilitado
        if let Some(ref bm25) = self.bm25_index {
            bm25.add(&chunk.id, Some(&metadata))?;
        }

        Ok(())
    }

    /// Inserta múltiples chunks en lote.
    ///
    /// # Argumentos
    ///
    /// * `chunks` - Iterator de tuplas (chunk, vector_opcional)
    ///
    /// # Ejemplo
    ///
    /// ```rust,ignore
    /// use minimemory::{VectorDB, Config};
    /// use minimemory::chunking::{chunk_markdown, ChunkConfig};
    ///
    /// let db = VectorDB::with_fulltext(Config::new(384), vec!["content".into()]).unwrap();
    /// let result = chunk_markdown(content, &ChunkConfig::default()).unwrap();
    ///
    /// // Con embeddings pregenerados
    /// let chunks_with_vectors: Vec<_> = result.chunks.iter()
    ///     .map(|c| (c, Some(generate_embedding(&c.content))))
    ///     .collect();
    ///
    /// db.insert_chunks(chunks_with_vectors).unwrap();
    /// ```
    pub fn insert_chunks<'a>(
        &self,
        chunks: impl IntoIterator<Item = (&'a crate::chunking::Chunk, Option<Vec<f32>>)>,
    ) -> Result<()> {
        for (chunk, vector) in chunks {
            self.insert_chunk(chunk, vector.as_deref())?;
        }
        Ok(())
    }

    /// Procesa e inserta un documento Markdown completo.
    ///
    /// Combina chunking + inserción en una sola operación.
    /// Útil para documentos sin embeddings (solo BM25/keyword search).
    ///
    /// # Argumentos
    ///
    /// * `content` - Contenido Markdown
    /// * `config` - Configuración de chunking
    ///
    /// # Retorna
    ///
    /// Número de chunks insertados.
    ///
    /// # Ejemplo
    ///
    /// ```rust,ignore
    /// use minimemory::{VectorDB, Config};
    /// use minimemory::chunking::ChunkConfig;
    ///
    /// let db = VectorDB::with_fulltext(
    ///     Config::new(3), // dimensiones no importan para keyword search
    ///     vec!["content".into(), "heading".into()]
    /// ).unwrap();
    ///
    /// let markdown = "# Title\nContent...";
    /// let count = db.ingest_markdown(markdown, &ChunkConfig::default()).unwrap();
    /// println!("Ingested {} chunks", count);
    /// ```
    pub fn ingest_markdown(
        &self,
        content: &str,
        config: &crate::chunking::ChunkConfig,
    ) -> Result<usize> {
        let result = crate::chunking::chunk_markdown(content, config)?;
        let count = result.chunks.len();

        for chunk in &result.chunks {
            self.insert_chunk(chunk, None)?;
        }

        Ok(count)
    }

    /// Procesa e inserta un archivo Markdown.
    ///
    /// # Argumentos
    ///
    /// * `path` - Ruta al archivo Markdown
    /// * `config` - Configuración de chunking
    ///
    /// # Retorna
    ///
    /// Número de chunks insertados.
    pub fn ingest_markdown_file(
        &self,
        path: &std::path::Path,
        config: &crate::chunking::ChunkConfig,
    ) -> Result<usize> {
        let result = crate::chunking::chunk_markdown_file(path, config)?;
        let count = result.chunks.len();

        for chunk in &result.chunks {
            self.insert_chunk(chunk, None)?;
        }

        Ok(count)
    }

    // ==================== BÚSQUEDA HÍBRIDA ====================

    /// Búsqueda híbrida con parámetros configurables.
    ///
    /// Permite combinar búsqueda vectorial, keyword (BM25), y filtros de metadata.
    ///
    /// # Argumentos
    ///
    /// * `params` - Parámetros de búsqueda híbrida
    ///
    /// # Ejemplo
    ///
    /// ```rust
    /// use minimemory::{VectorDB, Config, HybridSearchParams, Filter};
    ///
    /// let db = VectorDB::with_fulltext(
    ///     Config::new(3),
    ///     vec!["title".into(), "content".into()]
    /// ).unwrap();
    ///
    /// // Búsqueda híbrida: vector + keyword
    /// let params = HybridSearchParams::hybrid(
    ///     vec![0.1, 0.2, 0.3],
    ///     "rust programming",
    ///     10
    /// );
    /// let results = db.hybrid_search(params).unwrap();
    /// ```
    pub fn hybrid_search(&self, params: HybridSearchParams) -> Result<Vec<HybridSearchResult>> {
        // Validar dimensiones si hay vector
        if let Some(ref vec) = params.vector {
            if vec.len() != self.config.dimensions {
                return Err(Error::DimensionMismatch {
                    expected: self.config.dimensions,
                    got: vec.len(),
                });
            }
        }

        HybridSearch::search(
            &params,
            self.index.as_ref(),
            self.bm25_index.as_ref().map(|b| b.as_ref()),
            self.storage.as_ref(),
            self.config.distance,
        )
    }

    /// Búsqueda por keywords usando BM25.
    ///
    /// Requiere que la DB haya sido creada con `with_fulltext`.
    ///
    /// # Argumentos
    ///
    /// * `query` - Texto a buscar
    /// * `k` - Número de resultados
    ///
    /// # Ejemplo
    ///
    /// ```rust
    /// use minimemory::{VectorDB, Config};
    ///
    /// let db = VectorDB::with_fulltext(
    ///     Config::new(3),
    ///     vec!["title".into(), "content".into()]
    /// ).unwrap();
    ///
    /// let results = db.keyword_search("rust programming", 10).unwrap();
    /// ```
    pub fn keyword_search(&self, query: &str, k: usize) -> Result<Vec<HybridSearchResult>> {
        let params = HybridSearchParams::keyword(query, k);
        self.hybrid_search(params)
    }

    /// Búsqueda solo por filtros de metadata.
    ///
    /// No realiza ranking por similitud, solo filtra documentos.
    ///
    /// # Argumentos
    ///
    /// * `filter` - Filtro de metadata
    /// * `limit` - Número máximo de resultados
    ///
    /// # Ejemplo
    ///
    /// ```rust
    /// use minimemory::{VectorDB, Config, Filter};
    ///
    /// let db = VectorDB::new(Config::new(3)).unwrap();
    ///
    /// let results = db.filter_search(
    ///     Filter::eq("category", "tech"),
    ///     100
    /// ).unwrap();
    /// ```
    pub fn filter_search(&self, filter: Filter, limit: usize) -> Result<Vec<HybridSearchResult>> {
        let params = HybridSearchParams::filter_only(filter, limit);
        self.hybrid_search(params)
    }

    /// Búsqueda vectorial con filtro de metadata.
    ///
    /// Combina búsqueda por similitud vectorial con filtrado de metadata.
    ///
    /// # Argumentos
    ///
    /// * `query` - Vector de consulta
    /// * `k` - Número de resultados
    /// * `filter` - Filtro de metadata
    ///
    /// # Ejemplo
    ///
    /// ```rust
    /// use minimemory::{VectorDB, Config, Filter};
    ///
    /// let db = VectorDB::new(Config::new(3)).unwrap();
    ///
    /// let results = db.search_with_filter(
    ///     &[0.1, 0.2, 0.3],
    ///     10,
    ///     Filter::eq("category", "tech")
    /// ).unwrap();
    /// ```
    pub fn search_with_filter(
        &self,
        query: &[f32],
        k: usize,
        filter: Filter,
    ) -> Result<Vec<SearchResult>> {
        if query.len() != self.config.dimensions {
            return Err(Error::DimensionMismatch {
                expected: self.config.dimensions,
                got: query.len(),
            });
        }

        let params = HybridSearchParams::vector(query.to_vec(), k).with_filter(filter);
        let hybrid_results = self.hybrid_search(params)?;

        // Convertir a SearchResult para compatibilidad
        Ok(hybrid_results
            .into_iter()
            .map(|hr| SearchResult {
                id: hr.id,
                distance: hr.vector_distance.unwrap_or(hr.score),
                metadata: hr.metadata,
            })
            .collect())
    }
    // ========================================================================
    // Paged / ordered search methods
    // ========================================================================

    /// List documents with optional filter, ordering, and pagination.
    ///
    /// This is the most SQL-like query method: SELECT * WHERE filter ORDER BY field LIMIT k OFFSET n.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use minimemory::{VectorDB, Config, Filter, OrderBy, PagedResult};
    ///
    /// let db = VectorDB::with_fulltext(Config::new(3), vec!["title".into()])?;
    ///
    /// // SELECT * WHERE status='active' ORDER BY created_at DESC LIMIT 10 OFFSET 0
    /// let page = db.list_documents(
    ///     Some(Filter::eq("status", "active")),
    ///     Some(OrderBy::desc("created_at")),
    ///     10,
    ///     0,
    /// )?;
    /// ```
    pub fn list_documents(
        &self,
        filter: Option<Filter>,
        order: Option<OrderBy>,
        limit: usize,
        offset: usize,
    ) -> Result<PagedResult<HybridSearchResult>> {
        // Collect all matching documents
        let all: Vec<HybridSearchResult> = self
            .storage
            .iter()
            .filter(|doc| {
                // Skip soft-deleted if metadata has deleted flag
                if let Some(ref meta) = doc.metadata {
                    if let Some(crate::types::MetadataValue::Bool(true)) = meta.get("deleted") {
                        return false;
                    }
                }
                // Apply filter if provided
                match &filter {
                    Some(f) => crate::query::FilterEvaluator::evaluate(f, doc.metadata.as_ref()),
                    None => true,
                }
            })
            .map(|doc| HybridSearchResult {
                id: doc.id,
                score: 0.0,
                vector_distance: None,
                bm25_score: None,
                vector_rank: None,
                keyword_rank: None,
                metadata: doc.metadata,
            })
            .collect();

        let total = all.len();

        // Apply ORDER BY
        let mut sorted = all;
        if let Some(ref order) = order {
            let field = &order.field;
            sorted.sort_by(|a, b| {
                let val_a = a.metadata.as_ref().and_then(|m| m.get(field));
                let val_b = b.metadata.as_ref().and_then(|m| m.get(field));
                let cmp = crate::query::compare_metadata_values(val_a, val_b);
                match order.direction {
                    crate::query::SortDirection::Asc => cmp,
                    crate::query::SortDirection::Desc => cmp.reverse(),
                }
            });
        }

        // Apply OFFSET + LIMIT
        let items: Vec<_> = sorted.into_iter().skip(offset).take(limit).collect();

        Ok(PagedResult {
            items,
            total,
            offset,
            limit,
        })
    }

    /// Filter search with ordering and pagination.
    ///
    /// Like `filter_search` but with ORDER BY and OFFSET support.
    pub fn filter_search_ordered(
        &self,
        filter: Filter,
        order: OrderBy,
        limit: usize,
        offset: usize,
    ) -> Result<PagedResult<HybridSearchResult>> {
        // Don't pass offset to hybrid_search — we need total count first
        let params = HybridSearchParams::filter_only(filter, usize::MAX)
            .with_order_by(order);

        // Get ALL matching results (ordered, no pagination yet)
        let all_results = self.hybrid_search(params)?;
        let total = all_results.len();

        // Apply offset + limit here
        let items: Vec<_> = all_results
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect();

        Ok(PagedResult {
            items,
            total,
            offset,
            limit,
        })
    }

    /// Vector search with pagination.
    ///
    /// Like `search` but returns PagedResult with total count.
    pub fn search_paged(
        &self,
        query: &[f32],
        limit: usize,
        offset: usize,
    ) -> Result<PagedResult<SearchResult>> {
        if query.len() != self.config.dimensions {
            return Err(Error::DimensionMismatch {
                expected: self.config.dimensions,
                got: query.len(),
            });
        }

        if self.storage.is_empty() {
            return Ok(PagedResult {
                items: vec![],
                total: 0,
                offset,
                limit,
            });
        }

        // Total is all documents with vectors (the searchable set)
        let total = self.storage.iter_with_vectors().count();

        // Fetch enough results for offset + limit
        let fetch_k = offset + limit;
        let all_results = self
            .index
            .search(query, fetch_k, self.storage.as_ref(), self.config.distance)?;

        let items: Vec<_> = all_results.into_iter().skip(offset).take(limit).collect();

        Ok(PagedResult {
            items,
            total,
            offset,
            limit,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_db() -> VectorDB {
        let config = Config::new(3)
            .with_distance(Distance::Euclidean)
            .with_index(IndexType::Flat);
        VectorDB::new(config).unwrap()
    }

    #[test]
    fn test_insert_and_search() {
        let db = create_test_db();

        db.insert("a", &[1.0, 0.0, 0.0], None).unwrap();
        db.insert("b", &[0.0, 1.0, 0.0], None).unwrap();
        db.insert("c", &[0.0, 0.0, 1.0], None).unwrap();

        let results = db.search(&[1.0, 0.0, 0.0], 2).unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "a");
        assert!((results[0].distance - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_dimension_mismatch() {
        let db = create_test_db();

        let result = db.insert("a", &[1.0, 2.0], None); // Wrong dimensions
        assert!(matches!(result, Err(Error::DimensionMismatch { .. })));
    }

    #[test]
    fn test_duplicate_insert() {
        let db = create_test_db();

        db.insert("a", &[1.0, 0.0, 0.0], None).unwrap();
        let result = db.insert("a", &[0.0, 1.0, 0.0], None);
        assert!(matches!(result, Err(Error::AlreadyExists(_))));
    }

    #[test]
    fn test_delete() {
        let db = create_test_db();

        db.insert("a", &[1.0, 0.0, 0.0], None).unwrap();
        assert!(db.contains("a"));

        let deleted = db.delete("a").unwrap();
        assert!(deleted);
        assert!(!db.contains("a"));
    }

    #[test]
    fn test_update() {
        let db = create_test_db();

        db.insert("a", &[1.0, 0.0, 0.0], None).unwrap();
        db.update("a", &[0.0, 1.0, 0.0], None).unwrap();

        let (vector, _) = db.get("a").unwrap().unwrap();
        assert_eq!(vector, Some(vec![0.0, 1.0, 0.0]));
    }

    #[test]
    fn test_metadata() {
        let db = create_test_db();

        let mut metadata = Metadata::new();
        metadata.insert("title", "Test document");
        metadata.insert("score", 42i64);

        db.insert("a", &[1.0, 0.0, 0.0], Some(metadata)).unwrap();

        let (_, meta) = db.get("a").unwrap().unwrap();
        let meta = meta.unwrap();

        assert!(matches!(
            meta.get("title"),
            Some(crate::types::MetadataValue::String(s)) if s == "Test document"
        ));
    }

    #[test]
    fn test_search_empty_db() {
        let db = create_test_db();
        let results = db.search(&[1.0, 0.0, 0.0], 10).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_cosine_similarity() {
        let config = Config::new(3)
            .with_distance(Distance::Cosine)
            .with_index(IndexType::Flat);
        let db = VectorDB::new(config).unwrap();

        db.insert("a", &[1.0, 0.0, 0.0], None).unwrap();
        db.insert("b", &[0.5, 0.5, 0.0], None).unwrap();
        db.insert("c", &[0.0, 1.0, 0.0], None).unwrap();

        let results = db.search(&[1.0, 0.0, 0.0], 3).unwrap();

        // Should be ordered by cosine similarity (a closest, c farthest)
        assert_eq!(results[0].id, "a");
    }

    // ==================== HYBRID SEARCH TESTS ====================

    fn create_fulltext_db() -> VectorDB {
        let config = Config::new(3)
            .with_distance(Distance::Euclidean)
            .with_index(IndexType::Flat);
        VectorDB::with_fulltext(config, vec!["title".into(), "content".into()]).unwrap()
    }

    #[test]
    fn test_insert_document_without_vector() {
        let db = create_fulltext_db();

        let mut meta = Metadata::new();
        meta.insert("title", "My Blog Post");
        meta.insert("content", "This is the content of my blog post");

        // Insert without vector
        db.insert_document("post-1", None, Some(meta)).unwrap();

        assert!(db.contains("post-1"));
        let (vec, meta) = db.get("post-1").unwrap().unwrap();
        assert!(vec.is_none()); // No vector
        assert!(meta.is_some()); // Has metadata
    }

    #[test]
    fn test_insert_document_with_vector() {
        let db = create_fulltext_db();

        let mut meta = Metadata::new();
        meta.insert("title", "Post with embedding");

        // Insert with vector
        db.insert_document("post-2", Some(&[0.1, 0.2, 0.3]), Some(meta))
            .unwrap();

        let (vec, _) = db.get("post-2").unwrap().unwrap();
        assert_eq!(vec, Some(vec![0.1, 0.2, 0.3]));
    }

    #[test]
    fn test_keyword_search() {
        let db = create_fulltext_db();

        let mut meta1 = Metadata::new();
        meta1.insert("title", "Rust Programming");
        meta1.insert("content", "Learn Rust systems programming");

        let mut meta2 = Metadata::new();
        meta2.insert("title", "Python Guide");
        meta2.insert("content", "Python for beginners");

        db.insert_document("doc-1", Some(&[1.0, 0.0, 0.0]), Some(meta1))
            .unwrap();
        db.insert_document("doc-2", Some(&[0.0, 1.0, 0.0]), Some(meta2))
            .unwrap();

        let results = db.keyword_search("rust programming", 10).unwrap();

        assert!(!results.is_empty());
        assert_eq!(results[0].id, "doc-1"); // Rust doc should be first
    }

    #[test]
    fn test_filter_search() {
        let db = create_fulltext_db();

        let mut meta1 = Metadata::new();
        meta1.insert("title", "Tech Article");
        meta1.insert("category", "tech");

        let mut meta2 = Metadata::new();
        meta2.insert("title", "Food Recipe");
        meta2.insert("category", "food");

        db.insert_document("doc-1", Some(&[1.0, 0.0, 0.0]), Some(meta1))
            .unwrap();
        db.insert_document("doc-2", Some(&[0.0, 1.0, 0.0]), Some(meta2))
            .unwrap();

        let results = db
            .filter_search(Filter::eq("category", "tech"), 10)
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "doc-1");
    }

    #[test]
    fn test_search_with_filter() {
        let db = create_fulltext_db();

        let mut meta1 = Metadata::new();
        meta1.insert("category", "tech");

        let mut meta2 = Metadata::new();
        meta2.insert("category", "food");

        db.insert("doc-1", &[1.0, 0.0, 0.0], Some(meta1)).unwrap();
        db.insert("doc-2", &[0.9, 0.1, 0.0], Some(meta2)).unwrap();

        // Search for vectors close to [1.0, 0.0, 0.0] but only in "tech" category
        let results = db
            .search_with_filter(&[1.0, 0.0, 0.0], 10, Filter::eq("category", "tech"))
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "doc-1");
    }

    #[test]
    fn test_hybrid_search() {
        let db = create_fulltext_db();

        let mut meta1 = Metadata::new();
        meta1.insert("title", "Rust Programming");
        meta1.insert("content", "Learn Rust");

        let mut meta2 = Metadata::new();
        meta2.insert("title", "Python Guide");
        meta2.insert("content", "Python basics");

        db.insert("doc-1", &[1.0, 0.0, 0.0], Some(meta1)).unwrap();
        db.insert("doc-2", &[0.0, 1.0, 0.0], Some(meta2)).unwrap();

        // Hybrid: vector close to doc-2, but keyword matches doc-1
        let params = HybridSearchParams::hybrid(
            vec![0.0, 1.0, 0.0], // Close to doc-2
            "rust",              // Matches doc-1
            10,
        );

        let results = db.hybrid_search(params).unwrap();

        // Both should appear in results (RRF combines both signals)
        assert!(results.len() >= 1);
    }

    #[test]
    fn test_metadata_only_documents_not_in_vector_search() {
        let db = create_fulltext_db();

        let mut meta1 = Metadata::new();
        meta1.insert("title", "Doc with vector");

        let mut meta2 = Metadata::new();
        meta2.insert("title", "Doc without vector");

        db.insert_document("doc-1", Some(&[1.0, 0.0, 0.0]), Some(meta1))
            .unwrap();
        db.insert_document("doc-2", None, Some(meta2)).unwrap();

        // Vector search should only find doc-1
        let results = db.search(&[1.0, 0.0, 0.0], 10).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "doc-1");

        // But keyword search should find doc-2
        let keyword_results = db.keyword_search("without vector", 10).unwrap();
        assert!(!keyword_results.is_empty());
        assert_eq!(keyword_results[0].id, "doc-2");
    }

    // ========================================================================
    // Quantization integration tests
    // ========================================================================

    fn create_quantized_db(quant: crate::quantization::QuantizationType) -> VectorDB {
        let config = Config::new(64)
            .with_distance(Distance::Cosine)
            .with_index(IndexType::Flat)
            .with_quantization(quant);
        VectorDB::new(config).unwrap()
    }

    fn generate_test_vector(dim: usize, seed: usize) -> Vec<f32> {
        let mut v: Vec<f32> = (0..dim)
            .map(|i| ((seed * dim + i) % 1000) as f32 / 1000.0 - 0.5)
            .collect();
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in &mut v {
                *x /= norm;
            }
        }
        v
    }

    #[test]
    fn test_int3_quantized_insert_and_search() {
        let db = create_quantized_db(crate::quantization::QuantizationType::Int3);

        // Use more distinct vectors so quantization doesn't confuse rankings
        let mut v1 = vec![0.0f32; 64];
        v1[0] = 1.0; // mostly in dim 0
        let mut v2 = vec![0.0f32; 64];
        v2[32] = 1.0; // mostly in dim 32
        let mut v3 = vec![0.0f32; 64];
        v3[63] = 1.0; // mostly in dim 63

        db.insert("a", &v1, None).unwrap();
        db.insert("b", &v2, None).unwrap();
        db.insert("c", &v3, None).unwrap();

        assert_eq!(db.len(), 3);

        // Query near v1 should find "a" closest
        let mut query = vec![0.0f32; 64];
        query[0] = 0.9;
        query[1] = 0.1;
        let results = db.search(&query, 1).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "a");
    }

    #[test]
    fn test_int8_quantized_insert_and_search() {
        let db = create_quantized_db(crate::quantization::QuantizationType::Int8);

        let v1 = generate_test_vector(64, 10);
        let v2 = generate_test_vector(64, 20);

        db.insert("x", &v1, None).unwrap();
        db.insert("y", &v2, None).unwrap();

        let results = db.search(&v1, 1).unwrap();
        assert_eq!(results[0].id, "x");
    }

    #[test]
    fn test_binary_quantized_insert_and_search() {
        let db = create_quantized_db(crate::quantization::QuantizationType::Binary);

        let v1 = generate_test_vector(64, 100);
        let v2 = generate_test_vector(64, 200);

        db.insert("p", &v1, None).unwrap();
        db.insert("q", &v2, None).unwrap();

        let results = db.search(&v1, 1).unwrap();
        assert_eq!(results[0].id, "p");
    }

    #[test]
    fn test_quantized_get_dequantizes() {
        let db = create_quantized_db(crate::quantization::QuantizationType::Int3);

        let v = generate_test_vector(64, 42);
        db.insert("doc", &v, None).unwrap();

        let (vector, _meta) = db.get("doc").unwrap().unwrap();
        let restored = vector.unwrap();
        assert_eq!(restored.len(), 64);

        // Dequantized vector should be approximately correct
        let error: f32 = v
            .iter()
            .zip(restored.iter())
            .map(|(a, b)| (a - b).abs())
            .sum::<f32>()
            / 64.0;
        assert!(
            error < 0.15,
            "Average dequantization error too large: {}",
            error
        );
    }

    #[test]
    fn test_quantized_update() {
        let db = create_quantized_db(crate::quantization::QuantizationType::Int3);

        let v1 = generate_test_vector(64, 1);
        let v2 = generate_test_vector(64, 2);

        db.insert("doc", &v1, None).unwrap();
        db.update("doc", &v2, None).unwrap();

        // Search should now match v2
        let results = db.search(&v2, 1).unwrap();
        assert_eq!(results[0].id, "doc");
    }

    #[test]
    fn test_quantized_with_metadata() {
        let db = create_quantized_db(crate::quantization::QuantizationType::Int3);

        let v = generate_test_vector(64, 5);
        let mut meta = Metadata::new();
        meta.insert("title", "Test Document");
        meta.insert("score", 42i64);

        db.insert("doc", &v, Some(meta)).unwrap();

        let (_, metadata) = db.get("doc").unwrap().unwrap();
        let meta = metadata.unwrap();
        assert_eq!(
            meta.get("title").unwrap().as_str().unwrap(),
            "Test Document"
        );
        assert_eq!(meta.get("score").unwrap().as_i64().unwrap(), 42);
    }

    #[test]
    fn test_no_quantization_unchanged() {
        // Ensure None quantization works exactly as before
        let db = create_quantized_db(crate::quantization::QuantizationType::None);

        let v = vec![0.1f32; 64];
        db.insert("doc", &v, None).unwrap();

        let (vector, _) = db.get("doc").unwrap().unwrap();
        let restored = vector.unwrap();

        // With no quantization, vector should be exactly the same
        for (a, b) in v.iter().zip(restored.iter()) {
            assert_eq!(a, b);
        }
    }

    #[test]
    fn test_quantized_save_and_load() {
        let path = {
            let mut p = std::env::temp_dir();
            p.push(format!(
                "minimemory_quant_test_{}.mmdb",
                std::process::id()
            ));
            p
        };

        // Create DB with Int3 quantization, insert data, save
        {
            let config = Config::new(64)
                .with_distance(Distance::Cosine)
                .with_index(IndexType::Flat)
                .with_quantization(crate::quantization::QuantizationType::Int3);
            let db = VectorDB::new(config).unwrap();

            let mut v1 = vec![0.0f32; 64];
            v1[0] = 1.0;
            let mut v2 = vec![0.0f32; 64];
            v2[32] = 1.0;

            let mut meta = Metadata::new();
            meta.insert("label", "first");
            db.insert("a", &v1, Some(meta)).unwrap();
            db.insert("b", &v2, None).unwrap();

            db.save(&path).unwrap();
        }

        // Load and verify
        let db = VectorDB::open(&path).unwrap();

        assert_eq!(db.len(), 2);
        assert!(db.contains("a"));
        assert!(db.contains("b"));

        // Verify dequantized vector is approximately correct
        let (vector, metadata) = db.get("a").unwrap().unwrap();
        let v = vector.unwrap();
        assert_eq!(v.len(), 64);
        assert!(v[0] > 0.5, "First dim should be large: {}", v[0]);
        let meta = metadata.unwrap();
        assert_eq!(meta.get("label").unwrap().as_str().unwrap(), "first");

        // Search should still work after reload
        let mut query = vec![0.0f32; 64];
        query[0] = 0.9;
        query[1] = 0.1;
        let results = db.search(&query, 1).unwrap();
        assert_eq!(results[0].id, "a");

        // New inserts should still use quantization
        let mut v3 = vec![0.0f32; 64];
        v3[63] = 1.0;
        db.insert("c", &v3, None).unwrap();
        assert_eq!(db.len(), 3);

        std::fs::remove_file(&path).ok();
    }

    // ========================================================================
    // Pagination and ORDER BY tests
    // ========================================================================

    fn create_articles_db() -> VectorDB {
        let db = VectorDB::with_fulltext(
            Config::new(3),
            vec!["title".into(), "content".into()],
        )
        .unwrap();

        let articles = vec![
            ("art-1", "Rust Guide", "programming", 3i64),
            ("art-2", "Alpha Basics", "science", 1i64),
            ("art-3", "Zebra Facts", "nature", 2i64),
            ("art-4", "AI Revolution", "tech", 5i64),
            ("art-5", "Cooking Tips", "lifestyle", 4i64),
        ];

        for (id, title, category, priority) in articles {
            let mut meta = Metadata::new();
            meta.insert("title", title);
            meta.insert("category", category);
            meta.insert("priority", priority);
            db.insert_document(id, None, Some(meta)).unwrap();
        }

        db
    }

    #[test]
    fn test_list_documents_order_by_string_asc() {
        let db = create_articles_db();

        let page = db
            .list_documents(None, Some(crate::query::OrderBy::asc("title")), 10, 0)
            .unwrap();

        assert_eq!(page.total, 5);
        assert_eq!(page.items.len(), 5);
        // Alphabetical order by title
        assert_eq!(page.items[0].id, "art-4"); // AI Revolution
        assert_eq!(page.items[1].id, "art-2"); // Alpha Basics
        assert_eq!(page.items[2].id, "art-5"); // Cooking Tips
        assert_eq!(page.items[3].id, "art-1"); // Rust Guide
        assert_eq!(page.items[4].id, "art-3"); // Zebra Facts
    }

    #[test]
    fn test_list_documents_order_by_int_desc() {
        let db = create_articles_db();

        let page = db
            .list_documents(None, Some(crate::query::OrderBy::desc("priority")), 10, 0)
            .unwrap();

        assert_eq!(page.total, 5);
        // Descending by priority: 5, 4, 3, 2, 1
        assert_eq!(page.items[0].id, "art-4"); // priority 5
        assert_eq!(page.items[1].id, "art-5"); // priority 4
        assert_eq!(page.items[2].id, "art-1"); // priority 3
        assert_eq!(page.items[3].id, "art-3"); // priority 2
        assert_eq!(page.items[4].id, "art-2"); // priority 1
    }

    #[test]
    fn test_list_documents_offset_limit() {
        let db = create_articles_db();

        // Page 1: offset 0, limit 2
        let page1 = db
            .list_documents(
                None,
                Some(crate::query::OrderBy::asc("priority")),
                2,
                0,
            )
            .unwrap();
        assert_eq!(page1.total, 5);
        assert_eq!(page1.items.len(), 2);
        assert_eq!(page1.items[0].id, "art-2"); // priority 1
        assert_eq!(page1.items[1].id, "art-3"); // priority 2
        assert!(page1.has_more());

        // Page 2: offset 2, limit 2
        let page2 = db
            .list_documents(
                None,
                Some(crate::query::OrderBy::asc("priority")),
                2,
                2,
            )
            .unwrap();
        assert_eq!(page2.total, 5);
        assert_eq!(page2.items.len(), 2);
        assert_eq!(page2.items[0].id, "art-1"); // priority 3
        assert_eq!(page2.items[1].id, "art-5"); // priority 4
        assert!(page2.has_more());

        // Page 3: offset 4, limit 2 → only 1 result left
        let page3 = db
            .list_documents(
                None,
                Some(crate::query::OrderBy::asc("priority")),
                2,
                4,
            )
            .unwrap();
        assert_eq!(page3.total, 5);
        assert_eq!(page3.items.len(), 1);
        assert_eq!(page3.items[0].id, "art-4"); // priority 5
        assert!(!page3.has_more());
    }

    #[test]
    fn test_list_documents_offset_beyond_results() {
        let db = create_articles_db();

        let page = db.list_documents(None, None, 10, 100).unwrap();
        assert_eq!(page.total, 5);
        assert_eq!(page.items.len(), 0);
        assert!(!page.has_more());
    }

    #[test]
    fn test_list_documents_with_filter_and_order() {
        let db = create_articles_db();

        // Filter: only tech and programming categories, ordered by title
        let page = db
            .list_documents(
                Some(
                    Filter::eq("category", "tech")
                        .or(Filter::eq("category", "programming")),
                ),
                Some(crate::query::OrderBy::asc("title")),
                10,
                0,
            )
            .unwrap();

        assert_eq!(page.total, 2);
        assert_eq!(page.items[0].id, "art-4"); // AI Revolution
        assert_eq!(page.items[1].id, "art-1"); // Rust Guide
    }

    #[test]
    fn test_filter_search_ordered() {
        let db = create_articles_db();

        let page = db
            .filter_search_ordered(
                Filter::gt("priority", 2i64),
                crate::query::OrderBy::desc("priority"),
                2,
                0,
            )
            .unwrap();

        assert_eq!(page.total, 3); // priority 3, 4, 5
        assert_eq!(page.items.len(), 2); // limit 2
        assert_eq!(page.items[0].id, "art-4"); // priority 5
        assert_eq!(page.items[1].id, "art-5"); // priority 4
        assert!(page.has_more());
    }

    #[test]
    fn test_paged_result_total_pages() {
        let db = create_articles_db();

        let page = db.list_documents(None, None, 2, 0).unwrap();
        assert_eq!(page.total_pages(), 3); // 5 items / 2 per page = 3 pages
        assert_eq!(page.current_page(), 0);
    }
}
