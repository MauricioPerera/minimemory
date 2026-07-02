//! Base de datos vectorial principal.

use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use parking_lot::Mutex;

use crate::distance::Distance;
use crate::error::{Error, Result};
use crate::index::{BM25Index, FlatIndex, HNSWIndex, IVFIndex, Index, IndexType};
use crate::metadata_index::{MetadataIndexManager, RangeOp};
use crate::partial_index::{PartialIndexConfig, PartialIndexManager, PartialIndexStats};
use crate::quantization::{QuantizationType, Quantizer};
use crate::query::{Filter, FilterOp, OrderBy};
use crate::search::{HybridSearch, HybridSearchParams};
use crate::storage::{disk, format::FileHeader, MemoryStorage, Storage};
use crate::types::{Config, HybridSearchResult, Metadata, PagedResult, SearchResult, VectorId};
#[cfg(not(target_arch = "wasm32"))]
use crate::wal::{WalConfig, WalOp, WalWriter};

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
    /// Índices de metadata opt-in por campo (aceleran `$eq`/rangos). No se
    /// persisten en `.mmdb` (v1): tras [`open`](VectorDB::open) hay que
    /// recrearlos con [`create_metadata_index`](VectorDB::create_metadata_index)
    /// (que indexa retroactivamente la metadata existente). Tampoco se
    /// restauran solos vía [`open_with_wal`](VectorDB::open_with_wal).
    metadata_indexes: MetadataIndexManager,
    /// Quantizer for vector compression (None when QuantizationType::None)
    quantizer: Option<Quantizer>,
    /// Write-Ahead Log para durabilidad por operación (None = sin WAL,
    /// comportamiento idéntico a antes de la integración).
    ///
    /// `Mutex` da interior mutabilidad: las mutaciones (`&self`) appendean sin
    /// necesidad de `&mut self`. Se inicializa en `None` y se activa con
    /// [`enable_wal`] / [`enable_wal_with`] / [`open_with_wal`] / [`new_with_wal`].
    #[cfg(not(target_arch = "wasm32"))]
    wal: Option<Mutex<WalWriter>>,
}

impl VectorDB {
    fn validate_vector(vector: &[f32]) -> Result<()> {
        match vector.iter().position(|v| !v.is_finite()) {
            Some(idx) => Err(Error::InvalidVector(format!(
                "non-finite value at index {}",
                idx
            ))),
            None => Ok(()),
        }
    }

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
            metadata_indexes: MetadataIndexManager::new(),
            quantizer,
            #[cfg(not(target_arch = "wasm32"))]
            wal: None,
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
            metadata_indexes: MetadataIndexManager::new(),
            quantizer,
            #[cfg(not(target_arch = "wasm32"))]
            wal: None,
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
            metadata_indexes: MetadataIndexManager::new(),
            quantizer,
            #[cfg(not(target_arch = "wasm32"))]
            wal: None,
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
            metadata_indexes: MetadataIndexManager::new(),
            quantizer,
            #[cfg(not(target_arch = "wasm32"))]
            wal: None,
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

        self.insert_document_inner(&id, Some(vector), &metadata)?;

        // Durabilidad: append al WAL tras mutar la memoria. Si el append falla
        // (disco lleno, etc.) la memoria YA fue mutada — el crate no es
        // transaccional, así que el error se propaga pero el cambio queda
        // (best-effort, coherente con la no-transaccionalidad documentada).
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.append_wal(WalOp::Insert {
                id,
                vector: Some(vector.to_vec()),
                metadata,
            })?;
        }

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

        self.insert_document_inner(&id, vector, &metadata)?;

        // Durabilidad: ver `insert` para la semántica de fallo del append.
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.append_wal(WalOp::Insert {
                id,
                vector: vector.map(|v| v.to_vec()),
                metadata,
            })?;
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
        Self::validate_vector(query)?;

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
    /// Esto es una eliminación **física**: quita el documento del storage, del
    /// índice vectorial, del índice BM25 y de los índices parciales. No es lo
    /// mismo que el soft-delete lógico que aplican los métodos de búsqueda
    /// (`metadata["deleted"] == true`), que sólo oculta el documento de
    /// `list_documents` y las búsquedas híbridas pero no lo remueve.
    ///
    /// # Retorna
    ///
    /// `true` si el vector existía y fue eliminado, `false` si no existía.
    ///
    /// # WAL
    ///
    /// Solo se appendea `WalOp::Delete` si el documento existía (borrado real).
    /// Un `delete` sobre un ID inexistente no muta nada y no genera entrada de
    /// WAL. Si el append falla tras un borrado exitoso, se propaga el error: la
    /// memoria ya mutó (best-effort, ver `insert`).
    pub fn delete(&self, id: &str) -> Result<bool> {
        let deleted = self.delete_inner(id)?;
        #[cfg(not(target_arch = "wasm32"))]
        if deleted {
            self.append_wal(WalOp::Delete {
                id: id.to_string(),
            })?;
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

        // Validar antes de mutar: si las dimensiones no cuadran, no tocamos el
        // documento existente.
        if vector.len() != self.config.dimensions {
            return Err(Error::DimensionMismatch {
                expected: self.config.dimensions,
                got: vector.len(),
            });
        }
        Self::validate_vector(vector)?;

        // Upsert idempotente sin loggear dos veces (delete + insert internos),
        // luego una sola `WalOp::Update`.
        self.delete_inner(&id)?;
        self.insert_document_inner(&id, Some(vector), &metadata)?;

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.append_wal(WalOp::Update {
                id,
                vector: Some(vector.to_vec()),
                metadata,
            })?;
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

        // Validar antes de mutar.
        if let Some(vec) = vector {
            if vec.len() != self.config.dimensions {
                return Err(Error::DimensionMismatch {
                    expected: self.config.dimensions,
                    got: vec.len(),
                });
            }
            Self::validate_vector(vec)?;
        }

        // Upsert idempotente (delete + insert internos) + una sola WalOp::Update.
        self.delete_inner(&id)?;
        self.insert_document_inner(&id, vector, &metadata)?;

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.append_wal(WalOp::Update {
                id,
                vector: vector.map(|v| v.to_vec()),
                metadata,
            })?;
        }

        Ok(())
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
    ///
    /// # WAL
    ///
    /// Appendea `WalOp::Clear` si el WAL está activo. A diferencia del resto de
    /// mutaciones, `clear` es infalible (no retorna `Result`) por contrato
    /// público histórico, así que un fallo del append **no se puede propagar**:
    /// se ignora silenciosamente. Es la única excepción al patrón "append falla
    /// ⇒ error" del resto de la API, impuesta por la firma existente. En la
    /// práctica el append sólo falla por I/O de disco y `clear` ya dejó la
    /// memoria vacía; el siguiente `open_with_wal` reconstruirá desde el
    /// snapshot + las ops posteriores al último checkpoint.
    pub fn clear(&self) {
        self.clear_inner();
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = self.append_wal(WalOp::Clear);
        }
    }

    // -----------------------------------------------------------------------
    // Núcleo interno de mutación (sin WAL).
    //
    // Estos helpers hacen exactamente el trabajo de storage/índice/BM25/parciales
    // que hacían los métodos públicos antes de la integración, sin tocar el WAL.
    // Los métodos públicos los llaman y luego appendean la `WalOp` que corresponde;
    // el replay los llama directamente (sin loggear, porque está leyendo DEL log).
    // -----------------------------------------------------------------------

    /// Inserción núcleo (sin WAL). Valida dimensiones/finitud, rechaza
    /// `AlreadyExists`, cuantiza si hay quantizer, e indexa en vectorial/BM25/
    /// parciales. `metadata` se pasa por referencia para que el llamador pueda
    /// además moverlo a la `WalOp`.
    fn insert_document_inner(
        &self,
        id: &str,
        vector: Option<&[f32]>,
        metadata: &Option<Metadata>,
    ) -> Result<()> {
        // Validar dimensiones si hay vector
        if let Some(vec) = vector {
            if vec.len() != self.config.dimensions {
                return Err(Error::DimensionMismatch {
                    expected: self.config.dimensions,
                    got: vec.len(),
                });
            }
            Self::validate_vector(vec)?;
        }

        if self.storage.contains(id) {
            return Err(Error::AlreadyExists(id.to_string()));
        }

        // Store with quantization if vector present and quantizer active
        if let (Some(vec), Some(ref quantizer)) = (vector, &self.quantizer) {
            let qvec = quantizer.quantize(vec)?;
            self.storage
                .insert_quantized(id.to_string(), qvec, metadata.clone())?;
        } else {
            let vec_data = vector.map(|v| v.to_vec());
            self.storage
                .insert(id.to_string(), vec_data, metadata.clone())?;
        }

        // Solo indexar en índice vectorial si hay vector
        if let Some(vec) = vector {
            self.index
                .add(id, vec, &*self.storage, self.config.distance)?;
            // Añadir a índices parciales que coincidan
            let _ = self.partial_indexes.on_insert(id, vec, metadata.as_ref());
        }

        // Indexar en BM25 si está habilitado
        if let Some(ref bm25) = self.bm25_index {
            bm25.add(id, metadata.as_ref())?;
        }

        // Mantener índices de metadata (sólo si hay alguno registrado: evita
        // el write lock en el caso común sin índices).
        if !self.metadata_indexes.is_empty() {
            let id_vec: VectorId = id.to_string();
            self.metadata_indexes.on_insert(&id_vec, metadata.as_ref());
        }

        Ok(())
    }

    /// Borrado núcleo (sin WAL). Devuelve `true` si el documento existía.
    fn delete_inner(&self, id: &str) -> Result<bool> {
        // Metadata vieja para los índices de metadata: se lee ANTES de mutar el
        // storage, porque on_delete la necesita para desindexar por valor.
        let md_idx_active = !self.metadata_indexes.is_empty();
        let old_metadata = if md_idx_active {
            self.storage.get(id)?.and_then(|d| d.metadata)
        } else {
            None
        };

        let deleted = self.storage.delete(id)?;
        if deleted {
            self.index.remove(id)?;
            // Remover de BM25 si está habilitado
            if let Some(ref bm25) = self.bm25_index {
                bm25.remove(id)?;
            }
            // Remover de índices parciales
            let _ = self.partial_indexes.on_delete(id);
            // Desindexar de metadata con la metadata vieja (desindexación por
            // valor, barata y con contadores exactos).
            if md_idx_active {
                let id_vec: VectorId = id.to_string();
                self.metadata_indexes.on_delete(&id_vec, old_metadata.as_ref());
            }
        }
        Ok(deleted)
    }

    /// `clear` núcleo (sin WAL).
    fn clear_inner(&self) {
        self.storage.clear();
        self.index.clear();
        if let Some(ref bm25) = self.bm25_index {
            bm25.clear();
        }
        let _ = self.partial_indexes.clear_all();
        // Vaciar buckets de metadata conservando los índices registrados.
        if !self.metadata_indexes.is_empty() {
            self.metadata_indexes.on_clear();
        }
    }

    /// Append de una `WalOp` al WAL activo, si lo hay. No-op si el WAL no está
    /// habilitado. Propaga errores de I/O/serialización del `WalWriter`.
    #[cfg(not(target_arch = "wasm32"))]
    fn append_wal(&self, op: WalOp) -> Result<()> {
        if let Some(ref wal) = self.wal {
            wal.lock().append(&op)?;
        }
        Ok(())
    }

    /// Aplica una `WalOp` leída del WAL sobre la DB **sin re-loggear** (durante
    /// el replay el `WalWriter` aún no está abierto, así que `append_wal` es
    /// no-op de todas formas; pero además los helpers `*_inner` no tocan el WAL).
    ///
    /// **Replay idempotente**: tolera ops ya reflejadas en el snapshot (crash
    /// entre `save` y `truncate` del checkpoint):
    /// - `Insert`/`Update` → upsert (delete + insert): si el id ya existe, se
    ///   reemplaza en vez de devolver `AlreadyExists`.
    /// - `Delete` de id inexistente → no-op (`delete_inner` devuelve `false`).
    /// - `Clear` → vacía todo (idempotente por definición).
    #[cfg(not(target_arch = "wasm32"))]
    fn apply_wal_op(&self, op: &WalOp) -> Result<()> {
        match op {
            WalOp::Insert { id, vector, metadata } => {
                self.replay_upsert(id, vector.as_deref(), metadata)?;
            }
            WalOp::Update { id, vector, metadata } => {
                self.replay_upsert(id, vector.as_deref(), metadata)?;
            }
            WalOp::Delete { id } => {
                // Idempotente: borrar un id ausente es no-op.
                self.delete_inner(id)?;
            }
            WalOp::Clear => {
                self.clear_inner();
            }
        }
        Ok(())
    }

    /// Upsert idempotente usado por el replay de `Insert`/`Update`: borra el id
    /// (si existe, sin error si no) y lo (re)inserta. Tras el `delete_inner` el
    /// id no está en storage, así que `insert_document_inner` no choca con
    /// `AlreadyExists`.
    #[cfg(not(target_arch = "wasm32"))]
    fn replay_upsert(
        &self,
        id: &str,
        vector: Option<&[f32]>,
        metadata: &Option<Metadata>,
    ) -> Result<()> {
        self.delete_inner(id)?;
        self.insert_document_inner(id, vector, metadata)
    }

    /// Reconstruye el índice principal a partir del storage completo.
    ///
    /// Reentrena o reconstruye el índice principal usando todos los vectores
    /// almacenados (dequantizándolos si la cuantización está activa). El
    /// comportamiento depende del tipo de índice configurado:
    ///
    /// - **IVF**: ejecuta K-means sobre todos los vectores para calcular los
    ///   centroides y asignar cada vector a su cluster. Sin este paso, IVF
    ///   cae a búsqueda brute-force para siempre (el clustering y `num_probes`
    ///   nunca se activan).
    /// - **HNSW**: reconstruye el grafo desde cero reinsertando todos los
    ///   vectores.
    /// - **Flat**: recarga el conjunto de IDs indexados (no-op efectivo).
    ///
    /// # Cuándo llamarlo
    ///
    /// **Obligatorio para IVF tras una carga masiva**: `rebuild_index()` debe
    /// llamarse después de insertar un conjunto grande de vectores para activar
    /// el clustering; de lo contrario la búsqueda opera por fuerza bruta sobre
    /// todos los vectores y `num_probes` no tiene efecto. Para HNSW y Flat es
    /// opcional (sirve para compactar/reorganizar el índice tras borrados
    /// masivos).
    ///
    /// # Errores
    ///
    /// Retorna un error si el índice subyacente falla la reconstrucción.
    pub fn rebuild_index(&self) -> Result<()> {
        self.index.rebuild(self.storage.as_ref())
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

    // ==================== WRITE-AHEAD LOG ====================

    /// Activa el WAL sobre `path` con configuración por defecto
    /// (`fsync_on_append = false`: sobrevive a crash de proceso, no a corte de
    /// energía). A partir de este momento, toda mutación (`insert`,
    /// `insert_document`, `update`, `update_document`, `delete`, `clear`)
    /// appendea su `WalOp` al log.
    ///
    /// Si `path` ya existe con un WAL válido, se reutiliza (y se trunca la cola
    /// rota si la hubiera). Toma `&mut self` porque activar el WAL es un cambio
    /// estructural (campo `Option` de `None` a `Some`), típico de setup antes
    /// de uso concurrente; las mutaciones posteriores siguen siendo `&self`
    /// vía la `Mutex` interior.
    ///
    /// Llamarlo dos veces reemplaza el writer anterior por uno nuevo sobre el
    /// nuevo `path`.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn enable_wal<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        self.enable_wal_with(path, WalConfig::default())
    }

    /// Activa el WAL sobre `path` con la [`WalConfig`] dada (por ejemplo
    /// `WalConfig::new().with_fsync_on_append(true)` para durabilidad ante corte
    /// de energía). Ver [`enable_wal`] para el resto de la semántica.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn enable_wal_with<P: AsRef<Path>>(&mut self, path: P, config: WalConfig) -> Result<()> {
        let writer = WalWriter::open_with(path, config)?;
        self.wal = Some(Mutex::new(writer));
        Ok(())
    }

    /// Checkpoint: persiste un snapshot `.mmdb` atómico en `snapshot_path`
    /// (reusa [`save`]) y **después** trunca el WAL.
    ///
    /// # Orden y por qué
    ///
    /// 1. `save(snapshot_path)` — vuelca el estado completo a disco atómicamente
    ///    (escritura a `.tmp` + rename).
    /// 2. `wal.truncate()` — vacía el log (el snapshot ya capturó todo).
    ///
    /// Este orden es el seguro: si el proceso crashea **entre** el snapshot y
    /// el truncate, el snapshot tiene el estado completo y el WAL todavía lleva
    /// las ops; un [`open_with_wal`] posterior las reaplica de forma
    /// **idempotente** (Insert→upsert, Delete→no-op, Update→upsert), llegando al
    /// mismo estado final. Si truncáramos antes de salvar y crasheáramos, se
    /// perdería la durabilidad no checkpointeada.
    ///
    /// Si no hay WAL activo, `checkpoint` equivale a un [`save`] plano (el
    /// truncate se omite).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn checkpoint<P: AsRef<Path>>(&self, snapshot_path: P) -> Result<()> {
        self.save(snapshot_path)?;
        if let Some(ref wal) = self.wal {
            wal.lock().truncate()?;
        }
        Ok(())
    }

    /// Abre una DB desde un snapshot `.mmdb` **existente** + un WAL, aplicando
    /// el replay, y deja el `WalWriter` abierto para seguir appendeando.
    ///
    /// # Flujo
    ///
    /// 1. Carga el snapshot con [`open`] (exige que exista; si no, error). De
    ///    él se derivan dimensiones, métrica, índice y cuantización.
    /// 2. `wal::replay(wal_path)` — lee las ops válidas (tolera cola rota).
    /// 3. Aplica las ops en orden via [`apply_wal_op`] (replay idempotente).
    /// 4. Abre el `WalWriter` sobre `wal_path` (trunca la cola rota si la hubo)
    ///    para seguir loggeando.
    ///
    /// # Cuándo usarla vs [`new_with_wal`]
    ///
    /// `open_with_wal` exige snapshot existente. Para una DB **nueva** (sin
    /// snapshot aún) con un WAL huérfano que quiera reaplicarse, usar
    /// [`new_with_wal`], que parte de una `Config` explícita.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn open_with_wal<P: AsRef<Path>, Q: AsRef<Path>>(
        snapshot_path: P,
        wal_path: Q,
    ) -> Result<Self> {
        let snapshot_path = snapshot_path.as_ref();
        if !snapshot_path.exists() {
            return Err(Error::InvalidConfig(format!(
                "open_with_wal: el snapshot no existe: {}",
                snapshot_path.display()
            )));
        }

        let mut db = Self::open(snapshot_path)?;
        db.replay_wal(wal_path)?;
        Ok(db)
    }

    /// Crea una DB **nueva** desde `config` y le aplica el replay de un WAL
    /// (típicamente huérfano, sin snapshot), dejando el `WalWriter` abierto.
    ///
    /// Caso de uso: una DB abierta con WAL que nunca hizo checkpoint (no hay
    /// `.mmdb`); al reabrir, se reconstruye desde la `Config` original + el
    /// replay completo del log.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new_with_wal<P: AsRef<Path>>(config: Config, wal_path: P) -> Result<Self> {
        let mut db = Self::new(config)?;
        db.replay_wal(wal_path)?;
        Ok(db)
    }

    /// Replay interno compartido por [`open_with_wal`] y [`new_with_wal`]:
    /// lee las ops válidas del WAL, las aplica de forma idempotente, y abre el
    /// `WalWriter` para seguir appendeando. El writer se abre **después** del
    /// replay, así que las ops aplicadas no se re-loggean.
    #[cfg(not(target_arch = "wasm32"))]
    fn replay_wal<P: AsRef<Path>>(&mut self, wal_path: P) -> Result<()> {
        let replay = crate::wal::replay(wal_path.as_ref())?;
        for op in &replay.ops {
            self.apply_wal_op(op)?;
        }
        let writer = WalWriter::open_with(wal_path, WalConfig::default())?;
        self.wal = Some(Mutex::new(writer));
        Ok(())
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

    // ==================== ÍNDICES DE METADATA ====================

    /// Crea un índice de metadata sobre `field` y lo popula **retroactivamente**
    /// con toda la metadata existente en storage.
    ///
    /// Los índices de metadata aceleran las hojas `$eq` y de rango
    /// (`$gt`/`$gte`/`$lt`/`$lte`) de [`Filter`] sobre el campo indexado: el
    /// query planner poda el conjunto de candidatos con el índice y luego
    /// re-evalúa el filtro completo (el índice nunca cambia resultados, sólo
    /// acelera).
    ///
    /// # Retroactividad
    ///
    /// A diferencia de un bug anterior con los índices parciales, este método
    /// indexa **después** la metadata de los documentos ya presentes (recorre
    /// el storage y alimenta al índice con `on_insert`). Sin este paso, los
    /// docs preexistentes quedarían fuera del índice y las consultas darían
    /// resultados incompletos. Llamar a `create_metadata_index` basta para
    /// indexar todo lo existente; no hay que reinsertar.
    ///
    /// # Persistencia (v1)
    ///
    /// Los índices de metadata **no se persisten** en `.mmdb` ni en el WAL:
    /// tras [`open`](Self::open) / [`open_with_wal`](Self::open_with_wal) el
    /// gestor arranca vacío y hay que recrearlos con este método (que indexa
    /// retroactivamente la metadata cargada del snapshot/WAL). Es una llamada
    /// por índice que se quiera mantener activo.
    ///
    /// # Errores
    ///
    /// - [`Error::AlreadyExists`] si ya existe un índice sobre `field`.
    /// - Propaga cualquier error de registro del índice.
    pub fn create_metadata_index(&self, field: &str) -> Result<()> {
        self.metadata_indexes.create_index(field)?;
        // Indexar retroactivamente toda la metadata existente. `on_insert` es
        // infalible, así que no hay error que propagar aquí; el único punto de
        // fallo (create_index) ya se ejecutó arriba con `?`.
        for doc in self.storage.iter() {
            self.metadata_indexes.on_insert(&doc.id, doc.metadata.as_ref());
        }
        Ok(())
    }

    /// Elimina el índice de metadata sobre `field`.
    ///
    /// Tras esto, las consultas sobre ese campo vuelven a resolverse por
    /// full-scan (mismos resultados, sólo más lento).
    ///
    /// # Errores
    ///
    /// - [`Error::NotFound`] si no existe un índice sobre `field`.
    pub fn drop_metadata_index(&self, field: &str) -> Result<()> {
        self.metadata_indexes.drop_index(field)
    }

    /// Lista los campos con índice de metadata registrado, en orden
    /// lexicográfico (determinista).
    pub fn list_metadata_indexes(&self) -> Vec<String> {
        self.metadata_indexes.list_indexes()
    }

    /// `true` si `field` tiene un índice de metadata registrado.
    pub fn has_metadata_index(&self, field: &str) -> bool {
        self.metadata_indexes.has_index(field)
    }

    // -------------------- Query planner --------------------

    /// Planifica un [`Filter`] contra los índices de metadata y devuelve el
    /// conjunto de ids candidatos, o `None` si no se puede podar.
    ///
    /// # Semántica
    ///
    /// - `None` ⇒ no hay forma de podar con los índices registrados (campo no
    ///   indexado, operador no indexable en v1, o un `$or` con alguna rama no
    ///   indexable). El caller hace full-scan y evalúa el filtro directo.
    /// - `Some(set)` ⇒ el caller puede restringir el escaneo a los ids del set
    ///   **y luego re-evaluar el filtro completo** sobre cada candidato. El set
    ///   es siempre un superset de los ids que cumplen el filtro (regla de oro:
    ///   el planner sólo poda, nunca decide resultados). `Some(vacío)` poda a
    ///   cero.
    ///
    /// # Reglas
    ///
    /// - Hoja `$eq` sobre campo indexado → `candidates_eq`. Hojas de rango →
    ///   `candidates_range` con el `RangeOp` correspondiente. Otros operadores
    ///   (`$ne`, `$in`, `$nin`, `$exists`, `$contains`, `$starts_with`,
    ///   `$ends_with`, `$regex`) → no indexables en v1.
    /// - `$and` → intersección de las ramas indexables; las ramas `None` se
    ///   verifican en la pasada final. Si ninguna rama es indexable → `None`.
    ///   Un `$and` vacío (siempre verdadero) → `None`.
    /// - `$or` → unión sólo si **todas** las ramas son indexables; si alguna es
    ///   `None` → `None` (fallback a full-scan del `$or` completo). Un `$or`
    ///   vacío (siempre verdadero) → `None`.
    /// - `$not` → no indexable en v1 → `None`.
    fn plan_filter_candidates(&self, filter: &Filter) -> Option<HashSet<VectorId>> {
        match filter {
            Filter::Condition { field, op } => self.plan_op_candidates(field, op),
            Filter::And(parts) => {
                // Intersección de las ramas indexables.
                let mut acc: Option<HashSet<VectorId>> = None;
                for part in parts {
                    if let Some(set) = self.plan_filter_candidates(part) {
                        acc = Some(match acc {
                            None => set,
                            Some(cur) => cur.intersection(&set).cloned().collect(),
                        });
                    }
                }
                acc
            }
            Filter::Or(parts) => {
                // `$or` vacío = siempre verdadero: no se puede podar.
                if parts.is_empty() {
                    return None;
                }
                // Unión sólo si todas las ramas son indexables.
                let mut acc: HashSet<VectorId> = HashSet::new();
                for part in parts {
                    match self.plan_filter_candidates(part) {
                        Some(set) => acc.extend(set),
                        None => return None,
                    }
                }
                Some(acc)
            }
            Filter::Not(_) => None,
        }
    }

    /// Candidatos de una hoja `Condition` según el operador.
    fn plan_op_candidates(&self, field: &str, op: &FilterOp) -> Option<HashSet<VectorId>> {
        match op {
            FilterOp::Eq(v) => self.metadata_indexes.candidates_eq(field, v),
            FilterOp::Gt(v) => self.metadata_indexes.candidates_range(field, RangeOp::Gt, v),
            FilterOp::Gte(v) => self.metadata_indexes.candidates_range(field, RangeOp::Gte, v),
            FilterOp::Lt(v) => self.metadata_indexes.candidates_range(field, RangeOp::Lt, v),
            FilterOp::Lte(v) => self.metadata_indexes.candidates_range(field, RangeOp::Lte, v),
            // $ne / $in / $nin / $exists / $contains / $starts_with /
            // $ends_with / $regex: no indexables en v1.
            _ => None,
        }
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
        self.partial_indexes.create_index(name, config)?;
        // Poblar retroactivamente con los documentos existentes que matchean
        // el filtro. Reusa el patrón de rebuild_partial_index.
        self.rebuild_partial_index(name).map(|_| ())
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
        Self::validate_vector(query)?;

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
            Self::validate_vector(vec)?;
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
            Self::validate_vector(vec)?;
        }

        // Un filtro con una regex inválida se rechaza aquí (antes de evaluarlo
        // documento a documento) para que el error sea visible y no se confunda
        // con "0 coincidencias".
        if let Some(ref filter) = params.filter {
            crate::query::FilterEvaluator::validate(filter)?;
        }

        // Query planner de índices de metadata: poda el conjunto de candidatos
        // cuando hay índices registrados que cubren (parte de) el filtro. La
        // verificación final del filtro se re-evalúa sobre cada candidato, así
        // el índice sólo acelera, nunca cambia resultados.
        let candidates = params
            .filter
            .as_ref()
            .and_then(|f| self.plan_filter_candidates(f));

        HybridSearch::search_with_candidates(
            &params,
            self.index.as_ref(),
            self.bm25_index.as_ref().map(|b| b.as_ref()),
            self.storage.as_ref(),
            self.config.distance,
            candidates.as_ref(),
        )
    }

    /// Búsqueda por keywords usando BM25.
    ///
    /// Requiere que la DB haya sido creada con `with_fulltext`.
    ///
    /// Los documentos con `metadata["deleted"] == true` se excluyen
    /// (soft-delete lógico); [`delete`](Self::delete) es eliminación física.
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
    /// Los documentos con `metadata["deleted"] == true` se excluyen
    /// (soft-delete lógico); [`delete`](Self::delete) es eliminación física.
    /// Un filtro con una regex que no compila se rechaza con
    /// [`Error::InvalidFilter`] en vez de devolverse "0 coincidencias".
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
    /// Los documentos con `metadata["deleted"] == true` se excluyen
    /// (soft-delete lógico); [`delete`](Self::delete) es eliminación física.
    /// Un filtro con una regex que no compila se rechaza con
    /// [`Error::InvalidFilter`] en vez de devolverse "0 coincidencias".
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
        Self::validate_vector(query)?;

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
    /// # Soft-delete
    ///
    /// Los documentos cuyo metadata contiene `"deleted" == true`
    /// (`MetadataValue::Bool(true)`) se excluyen de los resultados (soft-delete
    /// lógico). [`delete`](Self::delete) es distinto: elimina el documento
    /// físicamente (storage, índice y BM25). Un documento soft-deleted sigue
    /// siendo devuelto por [`get`](Self::get) y por la búsqueda vectorial pura
    /// [`search`](Self::search), pero no por este método ni por las búsquedas
    /// híbridas (`filter_search`, `search_with_filter`, `keyword_search`,
    /// `hybrid_search`).
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
        // Un filtro con una regex inválida se rechaza antes de evaluarlo para
        // que el error sea visible (y no se confunda con "0 coincidencias").
        if let Some(ref f) = filter {
            crate::query::FilterEvaluator::validate(f)?;
        }

        // Query planner de índices de metadata: poda candidatos cuando hay
        // índices que cubren (parte de) el filtro. El filtro se re-evalúa
        // completo sobre cada candidato (el índice sólo acelera).
        let candidates = filter.as_ref().and_then(|f| self.plan_filter_candidates(f));

        // Collect all matching documents
        let all: Vec<HybridSearchResult> = self
            .storage
            .iter()
            .filter(|doc| {
                // Poda del planner: si hay candidatos, descartar los ids fuera
                // del set (superset de los que cumplen el filtro completo).
                if let Some(ref cands) = candidates {
                    if !cands.contains(&doc.id) {
                        return false;
                    }
                }
                // Skip soft-deleted if metadata has deleted flag
                if let Some(ref meta) = doc.metadata {
                    if let Some(crate::types::MetadataValue::Bool(true)) = meta.get("deleted") {
                        return false;
                    }
                }
                // Apply filter if provided (verificación final, siempre).
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
        Self::validate_vector(query)?;

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

        // Fetch enough results for offset + limit. Saturate the addition to
        // avoid arithmetic overflow on extreme offset/limit, and clamp to the
        // searchable set so we never ask the index for more than exists (which
        // would also overflow the heap's `k + 1` capacity on huge `k`).
        let fetch_k = offset.saturating_add(limit).min(total);
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
    fn test_update_dimension_mismatch() {
        let db = create_test_db();

        db.insert("a", &[1.0, 0.0, 0.0], None).unwrap();
        let result = db.update("a", &[0.0, 1.0], None);
        assert!(matches!(result, Err(Error::DimensionMismatch { .. })));

        let (vector, _) = db.get("a").unwrap().unwrap();
        assert_eq!(vector, Some(vec![1.0, 0.0, 0.0]));

        let results = db.search(&[1.0, 0.0, 0.0], 1).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "a");
    }

    #[test]
    fn test_update_document_dimension_mismatch() {
        let db = create_test_db();

        db.insert("a", &[1.0, 0.0, 0.0], None).unwrap();
        let result = db.update_document("a", Some(&[0.0, 1.0]), None);
        assert!(matches!(result, Err(Error::DimensionMismatch { .. })));

        let (vector, _) = db.get("a").unwrap().unwrap();
        assert_eq!(vector, Some(vec![1.0, 0.0, 0.0]));

        let results = db.search(&[1.0, 0.0, 0.0], 1).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "a");
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
    fn test_regex_filter_invalid_surfaces_error() {
        let db = create_fulltext_db();

        let mut meta1 = Metadata::new();
        meta1.insert("title", "Tech Article");

        let mut meta2 = Metadata::new();
        meta2.insert("title", "Food Recipe");

        db.insert_document("doc-1", Some(&[1.0, 0.0, 0.0]), Some(meta1))
            .unwrap();
        db.insert_document("doc-2", Some(&[0.0, 1.0, 0.0]), Some(meta2))
            .unwrap();

        // Invalid regex must be reported as an error, not silently "0 results".
        let err = db
            .filter_search(Filter::regex("title", "[unclosed"), 10)
            .unwrap_err();
        assert!(matches!(err, Error::InvalidFilter(_)));

        // Same via search_with_filter and list_documents.
        let err = db
            .search_with_filter(&[1.0, 0.0, 0.0], 10, Filter::regex("title", "[unclosed"))
            .unwrap_err();
        assert!(matches!(err, Error::InvalidFilter(_)));

        let err = db
            .list_documents(Some(Filter::regex("title", "[unclosed")), None, 10, 0)
            .unwrap_err();
        assert!(matches!(err, Error::InvalidFilter(_)));
    }

    #[test]
    fn test_regex_filter_valid_still_works() {
        let db = create_fulltext_db();

        let mut meta1 = Metadata::new();
        meta1.insert("title", "Tech Article");

        let mut meta2 = Metadata::new();
        meta2.insert("title", "Food Recipe");

        db.insert_document("doc-1", Some(&[1.0, 0.0, 0.0]), Some(meta1))
            .unwrap();
        db.insert_document("doc-2", Some(&[0.0, 1.0, 0.0]), Some(meta2))
            .unwrap();

        let results = db
            .filter_search(Filter::regex("title", "^Tech"), 10)
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "doc-1");
    }

    #[test]
    fn test_search_with_filter_huge_k_no_overflow() {
        let db = create_test_db();

        let mut meta1 = Metadata::new();
        meta1.insert("category", "tech");
        db.insert("doc-1", &[1.0, 0.0, 0.0], Some(meta1)).unwrap();

        // k = usize::MAX with a filter must not panic (saturating fetch in the
        // hybrid path clamps to the index size).
        let results = db
            .search_with_filter(&[1.0, 0.0, 0.0], usize::MAX, Filter::eq("category", "tech"))
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "doc-1");
    }

    #[test]
    fn test_search_paged_extreme_offset_limit_no_overflow() {
        let db = create_test_db();
        db.insert("a", &[1.0, 0.0, 0.0], None).unwrap();
        db.insert("b", &[0.0, 1.0, 0.0], None).unwrap();

        // offset + limit that would overflow usize: must not panic.
        let page = db.search_paged(&[1.0, 0.0, 0.0], 1, usize::MAX).unwrap();
        // No results match such an offset, but it returns Ok without panicking.
        assert_eq!(page.items.len(), 0);
        assert_eq!(page.total, 2);
    }

    #[test]
    fn test_soft_delete_excluded_from_list_and_filter() {
        let db = create_fulltext_db();

        let mut meta1 = Metadata::new();
        meta1.insert("title", "Tech Article");
        meta1.insert("category", "tech");
        // Mark as soft-deleted via the magic metadata flag.
        meta1.insert("deleted", true);

        let mut meta2 = Metadata::new();
        meta2.insert("title", "Food Recipe");
        meta2.insert("category", "food");

        db.insert_document("doc-1", Some(&[1.0, 0.0, 0.0]), Some(meta1))
            .unwrap();
        db.insert_document("doc-2", Some(&[0.0, 1.0, 0.0]), Some(meta2))
            .unwrap();

        // list_documents hides the soft-deleted doc.
        let page = db.list_documents(None, None, 10, 0).unwrap();
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].id, "doc-2");

        // filter_search hides it too.
        let results = db.filter_search(Filter::eq("category", "tech"), 10).unwrap();
        assert!(results.is_empty());

        // But get() still returns it (physical delete is a different thing).
        assert!(db.get("doc-1").unwrap().is_some());
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

    // ========================================================================
    // rebuild_index tests
    // ========================================================================

    #[test]
    fn test_rebuild_index_ivf_activates_clustering() {
        let config = Config::new(4)
            .with_distance(Distance::Euclidean)
            .with_index(IndexType::IVF {
                num_clusters: 2,
                num_probes: 1,
            });
        let db = VectorDB::new(config).unwrap();

        // 2 clusters bien separados, 100 vectores cada uno
        let insert_cluster = |prefix: &str, base: f32| {
            for i in 0..100 {
                let id = format!("{}{}", prefix, i);
                // Offset estrictamente creciente (sin wrap) => a0 == base exacto y único
                let v = [base + i as f32 * 0.001; 4];
                db.insert(&id, &v, None).unwrap();
            }
        };
        insert_cluster("a", 10.0); // cluster A alrededor de [10,10,10,10]
        insert_cluster("b", -10.0); // cluster B alrededor de [-10,-10,-10,-10]
        assert_eq!(db.len(), 200);

        let query = [10.0, 10.0, 10.0, 10.0]; // == a0

        // Antes de rebuild, IVF no está entrenado -> brute force sobre los 200
        let before = db.search(&query, 200).unwrap();
        assert_eq!(before.len(), 200);

        // rebuild_index entrena k-means y activa nprobe=1
        db.rebuild_index().unwrap();

        // Con nprobe=1 solo se explora el cluster más cercano (A) -> 100 candidatos
        let after = db.search(&query, 200).unwrap();
        assert_eq!(
            after.len(),
            100,
            "nprobe=1 debería restringir la búsqueda a un solo cluster"
        );
        // El vecino más cercano es el match exacto a0
        assert_eq!(after[0].id, "a0");
        assert!(after[0].distance < 1e-6);
        // Todos los resultados pertenecen al cluster A
        for r in &after {
            assert!(
                r.id.starts_with('a'),
                "resultado inesperado del cluster B: {}",
                r.id
            );
        }
    }

    #[test]
    fn test_rebuild_index_empty_db() {
        let config = Config::new(4).with_index(IndexType::IVF {
            num_clusters: 4,
            num_probes: 2,
        });
        let db = VectorDB::new(config).unwrap();

        // DB vacía: rebuild no falla
        db.rebuild_index().unwrap();
        assert_eq!(db.len(), 0);

        let results = db.search(&[1.0, 1.0, 1.0, 1.0], 5).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_rebuild_index_with_int8_quantization() {
        let config = Config::new(64)
            .with_distance(Distance::Cosine)
            .with_index(IndexType::Flat)
            .with_quantization(crate::quantization::QuantizationType::Int8);
        let db = VectorDB::new(config).unwrap();

        let v1 = generate_test_vector(64, 10);
        let v2 = generate_test_vector(64, 20);
        let v3 = generate_test_vector(64, 30);

        db.insert("a", &v1, None).unwrap();
        db.insert("b", &v2, None).unwrap();
        db.insert("c", &v3, None).unwrap();

        // Búsqueda antes de rebuild
        let before = db.search(&v1, 1).unwrap();
        assert_eq!(before[0].id, "a");

        // rebuild_index con cuantización activa no rompe la búsqueda
        db.rebuild_index().unwrap();

        let after = db.search(&v1, 1).unwrap();
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].id, "a");
    }

    #[test]
    fn test_rebuild_index_hnsw_preserves_search() {
        // HNSW es aproximado: "no rompe la búsqueda" se verifica como recall
        // contra brute-force, no como match exacto del vecino más cercano.
        let dim = 16;
        let config = Config::new(dim)
            .with_distance(Distance::Euclidean)
            .with_index(IndexType::hnsw_with_params(16, 200));
        let db = VectorDB::new(config).unwrap();

        let n = 200;
        let vectors: Vec<Vec<f32>> = (0..n).map(|i| spread_vector(dim, i)).collect();
        for (i, v) in vectors.iter().enumerate() {
            db.insert(&format!("v{}", i), v, None).unwrap();
        }
        assert_eq!(db.len(), n);

        let recall_before = hnsw_recall(&db, &vectors, dim, 10, 10);
        // rebuild_index no debe romper la búsqueda existente
        db.rebuild_index().unwrap();
        assert_eq!(db.len(), n, "rebuild no debe cambiar el número de vectores");

        let recall_after = hnsw_recall(&db, &vectors, dim, 10, 10);
        assert!(
            recall_after >= 0.8,
            "recall tras rebuild demasiado bajo: {:.3}",
            recall_after
        );
        assert!(
            recall_after >= recall_before - 0.05,
            "rebuild degradó el recall: antes={:.3} después={:.3}",
            recall_before,
            recall_after
        );
    }

    /// Generador determinista de vectores bien repartidos en [-1,1]^dim
    /// (LCG; evita la cadena 1D que produce `generate_test_vector`).
    fn spread_vector(dim: usize, seed: usize) -> Vec<f32> {
        let mut s = (seed as u64).wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(1);
        (0..dim)
            .map(|_| {
                s = s
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                ((s >> 33) as f32 / (1u32 << 31) as f32) * 2.0 - 1.0
            })
            .collect()
    }

    /// Mide el recall@k promedio de la DB contra brute-force sobre `vectors`,
    /// usando `num_queries` consultas (semillas fuera del conjunto insertado).
    fn hnsw_recall(
        db: &VectorDB,
        vectors: &[Vec<f32>],
        dim: usize,
        k: usize,
        num_queries: usize,
    ) -> f32 {
        let mut total = 0.0f32;
        for q in 0..num_queries {
            let query = spread_vector(dim, vectors.len() + q + 1);

            // Brute-force top-k
            let mut dists: Vec<(usize, f32)> = vectors
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    let d: f32 = query
                        .iter()
                        .zip(v.iter())
                        .map(|(a, b)| (a - b) * (a - b))
                        .sum::<f32>()
                        .sqrt();
                    (i, d)
                })
                .collect();
            dists.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            let exact: std::collections::HashSet<String> = dists[..k]
                .iter()
                .map(|(i, _)| format!("v{}", i))
                .collect();

            let results = db.search(&query, k).unwrap();
            let got: std::collections::HashSet<String> =
                results.iter().map(|r| r.id.clone()).collect();

            total += exact.intersection(&got).count() as f32 / k as f32;
        }
        total / num_queries as f32
    }
}
