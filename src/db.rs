//! Base de datos vectorial principal.

use std::path::Path;
use std::sync::Arc;

use crate::distance::Distance;
use crate::error::{Error, Result};
use crate::index::{BM25Index, FlatIndex, HNSWIndex, Index, IndexType};
use crate::partial_index::{PartialIndexConfig, PartialIndexManager, PartialIndexStats};
use crate::query::Filter;
use crate::search::{HybridSearch, HybridSearchParams};
use crate::storage::{disk, format::FileHeader, MemoryStorage, Storage};
use crate::types::{Config, HybridSearchResult, Metadata, SearchResult, VectorId};

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

        Ok(Self {
            config,
            storage,
            index,
            bm25_index: None,
            bm25_fields: Vec::new(),
            partial_indexes: PartialIndexManager::new(),
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

        Ok(Self {
            config,
            storage,
            index,
            bm25_index: Some(bm25_index),
            bm25_fields: indexed_fields,
            partial_indexes: PartialIndexManager::new(),
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
        let (header, vectors) = disk::load_vectors(path)?;

        let config = Config {
            dimensions: header.dimensions as usize,
            distance: header.get_distance(),
            index: header.get_index_type(),
            quantization: crate::quantization::QuantizationType::None, // Legacy files don't have quantization
        };

        let storage = Arc::new(MemoryStorage::new());
        let index = Self::create_index(&config.index)?;

        // Cargar documentos al storage e índice
        for stored in vectors {
            let id = stored.id.clone();
            // Insertar en storage primero
            storage.insert(stored.id.clone(), stored.vector.clone(), stored.metadata)?;
            // Solo indexar si tiene vector
            if let Some(ref vec) = stored.vector {
                index.add(&id, vec, &*storage, config.distance)?;
            }
        }

        Ok(Self {
            config,
            storage,
            index,
            bm25_index: None,
            bm25_fields: Vec::new(),
            partial_indexes: PartialIndexManager::new(),
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
        let (header, vectors) = disk::load_vectors(path)?;

        let config = Config {
            dimensions: header.dimensions as usize,
            distance: header.get_distance(),
            index: header.get_index_type(),
            quantization: crate::quantization::QuantizationType::None, // Legacy files don't have quantization
        };

        let storage = Arc::new(MemoryStorage::new());
        let index = Self::create_index(&config.index)?;
        let bm25_index = Arc::new(BM25Index::new(indexed_fields.clone()));

        // Cargar documentos al storage, índice vectorial y BM25
        for stored in vectors {
            let id = stored.id.clone();
            // Insertar en storage primero
            storage.insert(stored.id.clone(), stored.vector.clone(), stored.metadata.clone())?;
            // Solo indexar en HNSW/Flat si tiene vector
            if let Some(ref vec) = stored.vector {
                index.add(&id, vec, &*storage, config.distance)?;
            }
            // Siempre indexar en BM25 si tiene metadata
            bm25_index.add(&id, stored.metadata.as_ref())?;
        }

        Ok(Self {
            config,
            storage,
            index,
            bm25_index: Some(bm25_index),
            bm25_fields: indexed_fields,
            partial_indexes: PartialIndexManager::new(),
        })
    }

    /// Crea un índice basado en la configuración.
    fn create_index(index_type: &IndexType) -> Result<Arc<dyn Index>> {
        match index_type {
            IndexType::Flat => Ok(Arc::new(FlatIndex::new())),
            IndexType::HNSW { m, ef_construction } => {
                Ok(Arc::new(HNSWIndex::new(*m, *ef_construction)))
            }
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

        self.storage.insert(id.clone(), Some(vector.to_vec()), metadata.clone())?;
        self.index.add(&id, vector, &*self.storage, self.config.distance)?;

        // Indexar en BM25 si está habilitado
        if let Some(ref bm25) = self.bm25_index {
            bm25.add(&id, metadata.as_ref())?;
        }

        // Añadir a índices parciales que coincidan
        let _ = self.partial_indexes.on_insert(&id, vector, metadata.as_ref());

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

        let vec_data = vector.map(|v| v.to_vec());
        self.storage.insert(id.clone(), vec_data, metadata.clone())?;

        // Solo indexar en índice vectorial si hay vector
        if let Some(vec) = vector {
            self.index.add(&id, vec, &*self.storage, self.config.distance)?;
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
            Some(stored) => Ok(Some((stored.vector, stored.metadata))),
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
        self.delete(&id)?;

        self.storage.insert(id.clone(), Some(vector.to_vec()), metadata.clone())?;
        self.index.add(&id, vector, &*self.storage, self.config.distance)?;

        // Re-indexar en BM25 si está habilitado
        if let Some(ref bm25) = self.bm25_index {
            bm25.add(&id, metadata.as_ref())?;
        }

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
        );

        disk::save_vectors(path, &mut header, self.storage.iter())
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
    pub fn search_partial(&self, index_name: &str, query: &[f32], k: usize) -> Result<Vec<SearchResult>> {
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
        let index = self.partial_indexes.get_index(index_name)
            .ok_or_else(|| Error::NotFound(index_name.to_string()))?;

        // Obtener todos los documentos con vector
        let all_ids = self.storage.ids();
        let documents: Vec<_> = all_ids.iter()
            .filter_map(|id| {
                if let Ok(Some(sv)) = self.storage.get(id) {
                    sv.vector.map(|vec| (id.clone(), vec, sv.metadata))
                } else {
                    None
                }
            })
            .collect();

        // Reconstruir
        let docs_iter = documents.iter().map(|(id, vec, meta)| {
            (id.as_str(), vec.as_slice(), meta.as_ref())
        });

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

        let vec_data = vector.map(|v| v.to_vec());
        self.storage.insert(chunk.id.clone(), vec_data, Some(metadata.clone()))?;

        // Solo indexar en índice vectorial si hay vector
        if let Some(vec) = vector {
            self.index.add(&chunk.id, vec, &*self.storage, self.config.distance)?;
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
        db.insert_document("post-2", Some(&[0.1, 0.2, 0.3]), Some(meta)).unwrap();

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

        db.insert_document("doc-1", Some(&[1.0, 0.0, 0.0]), Some(meta1)).unwrap();
        db.insert_document("doc-2", Some(&[0.0, 1.0, 0.0]), Some(meta2)).unwrap();

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

        db.insert_document("doc-1", Some(&[1.0, 0.0, 0.0]), Some(meta1)).unwrap();
        db.insert_document("doc-2", Some(&[0.0, 1.0, 0.0]), Some(meta2)).unwrap();

        let results = db.filter_search(Filter::eq("category", "tech"), 10).unwrap();

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
        let results = db.search_with_filter(
            &[1.0, 0.0, 0.0],
            10,
            Filter::eq("category", "tech"),
        ).unwrap();

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

        db.insert_document("doc-1", Some(&[1.0, 0.0, 0.0]), Some(meta1)).unwrap();
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
}
