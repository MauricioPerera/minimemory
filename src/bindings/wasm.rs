//! Bindings WebAssembly para minimemory usando wasm-bindgen.
//!
//! ## Uso en JavaScript/TypeScript
//!
//! ```javascript
//! import init, { WasmVectorDB } from 'minimemory';
//!
//! // Inicializar WASM
//! await init();
//!
//! // Crear base de datos (64 dimensiones, cosine distance)
//! const db = new WasmVectorDB(64, "cosine", "flat");
//!
//! // Insertar vectores
//! db.insert("doc1", new Float32Array([0.1, 0.2, ...]));
//! db.insert_with_metadata("doc2", new Float32Array([...]), { title: "Mi doc" });
//!
//! // Buscar
//! const results = db.search(new Float32Array([0.1, ...]), 10);
//! console.log(results); // [{ id: "doc1", distance: 0.05, metadata: {...} }, ...]
//!
//! // CRUD
//! db.update("doc1", new Float32Array([...]));
//! const exists = db.contains("doc1");
//! db.delete("doc1");
//!
//! // Exportar/Importar como JSON
//! const json = db.export_json();
//! db.import_json(json);
//! ```

use wasm_bindgen::prelude::*;

use crate::{
    chunking::{ChunkConfig as RustChunkConfig, ChunkStrategy},
    okf::{OkfConfig, OkfIndex as RustOkfIndex},
    quantization::QuantizationType, Config as RustConfig, Distance as RustDistance,
    IndexType as RustIndexType, Metadata as RustMetadata, VectorDB as RustVectorDB,
};

/// Base de datos vectorial para WebAssembly.
/// Permite almacenar y buscar vectores de alta dimensionalidad.
#[wasm_bindgen]
pub struct WasmVectorDB {
    inner: RustVectorDB,
}

#[wasm_bindgen]
impl WasmVectorDB {
    /// Crea una nueva base de datos vectorial.
    ///
    /// # Arguments
    /// * `dimensions` - Numero de dimensiones de los vectores
    /// * `distance` - Metrica de distancia: "cosine", "euclidean", "dot"
    /// * `index_type` - Tipo de indice: "flat", "hnsw"
    #[wasm_bindgen(constructor)]
    pub fn new(dimensions: usize, distance: &str, index_type: &str) -> Result<WasmVectorDB, JsError> {
        let dist = match distance {
            "cosine" | "cos" => RustDistance::Cosine,
            "euclidean" | "l2" => RustDistance::Euclidean,
            "dot" | "dot_product" | "inner" => RustDistance::DotProduct,
            "manhattan" | "l1" => RustDistance::Manhattan,
            d => return Err(JsError::new(&format!("Unknown distance: {}. Use 'cosine', 'euclidean', or 'dot'", d))),
        };

        let index = match index_type {
            "flat" | "brute" | "exact" => RustIndexType::Flat,
            "hnsw" => RustIndexType::HNSW {
                m: 16,
                ef_construction: 200,
            },
            i => return Err(JsError::new(&format!("Unknown index: {}. Use 'flat' or 'hnsw'", i))),
        };

        let config = RustConfig::new(dimensions)
            .with_distance(dist)
            .with_index(index);

        let db = RustVectorDB::new(config)
            .map_err(|e| JsError::new(&e.to_string()))?;

        Ok(Self { inner: db })
    }

    /// Crea una base de datos con configuracion HNSW personalizada.
    #[wasm_bindgen]
    pub fn new_hnsw(dimensions: usize, distance: &str, m: usize, ef_construction: usize) -> Result<WasmVectorDB, JsError> {
        let dist = parse_distance(distance)?;

        let config = RustConfig::new(dimensions)
            .with_distance(dist)
            .with_index(RustIndexType::HNSW { m, ef_construction });

        let db = RustVectorDB::new(config)
            .map_err(|e| JsError::new(&e.to_string()))?;

        Ok(Self { inner: db })
    }

    /// Crea una base de datos con cuantizacion Int8 (4x menos memoria).
    ///
    /// # Arguments
    /// * `dimensions` - Numero de dimensiones
    /// * `distance` - "cosine", "euclidean", "dot"
    /// * `index_type` - "flat" o "hnsw"
    #[wasm_bindgen]
    pub fn new_int8(dimensions: usize, distance: &str, index_type: &str) -> Result<WasmVectorDB, JsError> {
        let dist = parse_distance(distance)?;
        let index = parse_index(index_type)?;

        let config = RustConfig::new(dimensions)
            .with_distance(dist)
            .with_index(index)
            .with_quantization(QuantizationType::Int8);

        let db = RustVectorDB::new(config)
            .map_err(|e| JsError::new(&e.to_string()))?;

        Ok(Self { inner: db })
    }

    /// Crea una base de datos con cuantizacion 3-bit (~10.7x menos memoria).
    /// Buen balance entre compresion y precision (~96-98% accuracy).
    ///
    /// # Arguments
    /// * `dimensions` - Numero de dimensiones
    /// * `distance` - "cosine", "euclidean", "dot"
    /// * `index_type` - "flat" o "hnsw"
    #[wasm_bindgen]
    pub fn new_int3(dimensions: usize, distance: &str, index_type: &str) -> Result<WasmVectorDB, JsError> {
        let dist = parse_distance(distance)?;
        let index = parse_index(index_type)?;

        let config = RustConfig::new(dimensions)
            .with_distance(dist)
            .with_index(index)
            .with_quantization(QuantizationType::Int3);

        let db = RustVectorDB::new(config)
            .map_err(|e| JsError::new(&e.to_string()))?;

        Ok(Self { inner: db })
    }

    /// Crea una base de datos con cuantizacion binaria (32x menos memoria).
    /// Ideal para vectores de alta dimension (256+).
    ///
    /// # Arguments
    /// * `dimensions` - Numero de dimensiones
    /// * `distance` - "cosine", "euclidean", "dot"
    /// * `index_type` - "flat" o "hnsw"
    #[wasm_bindgen]
    pub fn new_binary(dimensions: usize, distance: &str, index_type: &str) -> Result<WasmVectorDB, JsError> {
        let dist = parse_distance(distance)?;
        let index = parse_index(index_type)?;

        let config = RustConfig::new(dimensions)
            .with_distance(dist)
            .with_index(index)
            .with_quantization(QuantizationType::Binary);

        let db = RustVectorDB::new(config)
            .map_err(|e| JsError::new(&e.to_string()))?;

        Ok(Self { inner: db })
    }

    /// Crea una base de datos con configuracion completa.
    ///
    /// # Arguments
    /// * `dimensions` - Numero de dimensiones
    /// * `distance` - "cosine", "euclidean", "dot"
    /// * `index_type` - "flat" o "hnsw"
    /// * `quantization` - "none", "int8", "binary"
    /// * `hnsw_m` - Parametro M para HNSW (default 16)
    /// * `hnsw_ef` - ef_construction para HNSW (default 200)
    #[wasm_bindgen]
    pub fn new_with_config(
        dimensions: usize,
        distance: &str,
        index_type: &str,
        quantization: &str,
        hnsw_m: Option<usize>,
        hnsw_ef: Option<usize>,
    ) -> Result<WasmVectorDB, JsError> {
        let dist = parse_distance(distance)?;

        let index = match index_type {
            "flat" | "brute" | "exact" => RustIndexType::Flat,
            "hnsw" => RustIndexType::HNSW {
                m: hnsw_m.unwrap_or(16),
                ef_construction: hnsw_ef.unwrap_or(200),
            },
            i => return Err(JsError::new(&format!("Unknown index: {}", i))),
        };

        let quant = match quantization {
            "none" | "f32" | "float32" => QuantizationType::None,
            "int8" | "i8" | "scalar" => QuantizationType::Int8,
            "int3" | "3bit" => QuantizationType::Int3,
            "binary" | "bit" | "1bit" => QuantizationType::Binary,
            "polar" | "angular" => QuantizationType::Polar,
            q => return Err(JsError::new(&format!("Unknown quantization: {}. Use 'none', 'int8', 'int3', 'binary', or 'polar'", q))),
        };

        let config = RustConfig::new(dimensions)
            .with_distance(dist)
            .with_index(index)
            .with_quantization(quant);

        let db = RustVectorDB::new(config)
            .map_err(|e| JsError::new(&e.to_string()))?;

        Ok(Self { inner: db })
    }

    /// Inserta un vector en la base de datos.
    #[wasm_bindgen]
    pub fn insert(&self, id: &str, vector: &[f32]) -> Result<(), JsError> {
        self.inner
            .insert(id, vector, None)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Inserta un vector con metadata (como JSON string).
    #[wasm_bindgen]
    pub fn insert_with_metadata(&self, id: &str, vector: &[f32], metadata_json: &str) -> Result<(), JsError> {
        let meta = parse_metadata_json(metadata_json)?;
        self.inner
            .insert(id, vector, Some(meta))
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Busca los k vectores mas similares.
    /// Retorna un JSON array con los resultados.
    #[wasm_bindgen]
    pub fn search(&self, query: &[f32], k: usize) -> Result<String, JsError> {
        let results = self.inner
            .search(query, k)
            .map_err(|e| JsError::new(&e.to_string()))?;

        let json_results: Vec<serde_json::Value> = results
            .into_iter()
            .map(|r| {
                let mut obj = serde_json::json!({
                    "id": r.id,
                    "distance": r.distance,
                });
                if let Some(meta) = r.metadata {
                    obj["metadata"] = metadata_to_json(&meta);
                }
                obj
            })
            .collect();

        serde_json::to_string(&json_results)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Obtiene un vector por su ID.
    /// Retorna null si no existe, o un JSON con vector y metadata.
    #[wasm_bindgen]
    pub fn get(&self, id: &str) -> Result<JsValue, JsError> {
        match self.inner.get(id).map_err(|e| JsError::new(&e.to_string()))? {
            Some((vector, metadata)) => {
                let result = serde_json::json!({
                    "vector": vector,
                    "metadata": metadata.map(|m| metadata_to_json(&m)),
                });
                let json = serde_json::to_string(&result)
                    .map_err(|e| JsError::new(&e.to_string()))?;
                Ok(JsValue::from_str(&json))
            }
            None => Ok(JsValue::NULL),
        }
    }

    /// Elimina un vector por su ID.
    #[wasm_bindgen]
    pub fn delete(&self, id: &str) -> Result<bool, JsError> {
        self.inner
            .delete(id)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Actualiza un vector existente.
    #[wasm_bindgen]
    pub fn update(&self, id: &str, vector: &[f32]) -> Result<(), JsError> {
        self.inner
            .update(id, vector, None)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Actualiza un vector con metadata.
    #[wasm_bindgen]
    pub fn update_with_metadata(&self, id: &str, vector: &[f32], metadata_json: &str) -> Result<(), JsError> {
        let meta = parse_metadata_json(metadata_json)?;
        self.inner
            .update(id, vector, Some(meta))
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Verifica si un vector existe.
    #[wasm_bindgen]
    pub fn contains(&self, id: &str) -> bool {
        self.inner.contains(id)
    }

    /// Numero de vectores en la base de datos.
    #[wasm_bindgen]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Verifica si esta vacia.
    #[wasm_bindgen]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Dimensiones de los vectores.
    #[wasm_bindgen]
    pub fn dimensions(&self) -> usize {
        self.inner.dimensions()
    }

    /// Limpia todos los vectores.
    #[wasm_bindgen]
    pub fn clear(&self) {
        self.inner.clear();
    }

    /// Obtiene todos los IDs como JSON array.
    #[wasm_bindgen]
    pub fn ids(&self) -> Result<String, JsError> {
        let ids = self.inner.list_ids()
            .map_err(|e| JsError::new(&e.to_string()))?;
        serde_json::to_string(&ids)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Busqueda por palabras clave (BM25).
    /// Retorna JSON array con resultados.
    #[wasm_bindgen]
    pub fn keyword_search(&self, query: &str, k: usize) -> Result<String, JsError> {
        let results = self.inner
            .keyword_search(query, k)
            .map_err(|e| JsError::new(&e.to_string()))?;

        let json_results: Vec<serde_json::Value> = results
            .into_iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.id,
                    "score": r.score,
                })
            })
            .collect();

        serde_json::to_string(&json_results)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    // =========================================================================
    // Metodos con truncado automatico para Matryoshka embeddings
    // =========================================================================

    /// Inserta un vector truncandolo automaticamente a las dimensiones de la DB.
    /// Ideal para embeddings Matryoshka (ej: Gemma 768d -> 256d).
    #[wasm_bindgen]
    pub fn insert_auto(&self, id: &str, full_vector: &[f32]) -> Result<(), JsError> {
        let truncated = truncate_and_normalize(full_vector, self.inner.dimensions());
        self.inner
            .insert(id, &truncated, None)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Inserta con metadata, truncando automaticamente.
    #[wasm_bindgen]
    pub fn insert_auto_with_metadata(&self, id: &str, full_vector: &[f32], metadata_json: &str) -> Result<(), JsError> {
        let truncated = truncate_and_normalize(full_vector, self.inner.dimensions());
        let meta = parse_metadata_json(metadata_json)?;
        self.inner
            .insert(id, &truncated, Some(meta))
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Busca truncando automaticamente el vector query.
    #[wasm_bindgen]
    pub fn search_auto(&self, full_query: &[f32], k: usize) -> Result<String, JsError> {
        let truncated = truncate_and_normalize(full_query, self.inner.dimensions());
        let results = self.inner
            .search(&truncated, k)
            .map_err(|e| JsError::new(&e.to_string()))?;

        let json_results: Vec<serde_json::Value> = results
            .into_iter()
            .map(|r| {
                let mut obj = serde_json::json!({
                    "id": r.id,
                    "distance": r.distance,
                });
                if let Some(meta) = r.metadata {
                    obj["metadata"] = metadata_to_json(&meta);
                }
                obj
            })
            .collect();

        serde_json::to_string(&json_results)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Actualiza truncando automaticamente.
    #[wasm_bindgen]
    pub fn update_auto(&self, id: &str, full_vector: &[f32]) -> Result<(), JsError> {
        let truncated = truncate_and_normalize(full_vector, self.inner.dimensions());
        self.inner
            .update(id, &truncated, None)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Actualiza con metadata, truncando automaticamente.
    #[wasm_bindgen]
    pub fn update_auto_with_metadata(&self, id: &str, full_vector: &[f32], metadata_json: &str) -> Result<(), JsError> {
        let truncated = truncate_and_normalize(full_vector, self.inner.dimensions());
        let meta = parse_metadata_json(metadata_json)?;
        self.inner
            .update(id, &truncated, Some(meta))
            .map_err(|e| JsError::new(&e.to_string()))
    }

    // =========================================================================
    // Document store methods (no vector required)
    // =========================================================================

    /// Insert a document with optional vector. Works as a document store when vector is null.
    /// metadata_json is required. vector is a Float32Array or null.
    #[wasm_bindgen]
    pub fn insert_document(&self, id: &str, vector: Option<Vec<f32>>, metadata_json: &str) -> Result<(), JsError> {
        let meta = parse_metadata_json(metadata_json)?;
        self.inner
            .insert_document(id, vector.as_deref(), Some(meta))
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Filter search: find documents matching metadata conditions.
    /// filter_json: MongoDB-style filter, e.g. '{"category": "tech"}'
    /// Returns JSON array of results.
    #[wasm_bindgen]
    pub fn filter_search(&self, filter_json: &str, limit: usize) -> Result<String, JsError> {
        let filter = parse_filter_json(filter_json)?;
        let results = self.inner
            .filter_search(filter, limit)
            .map_err(|e| JsError::new(&e.to_string()))?;

        let json_results: Vec<serde_json::Value> = results
            .into_iter()
            .map(|r| {
                let mut obj = serde_json::json!({ "id": r.id, "score": r.score });
                if let Some(meta) = r.metadata {
                    obj["metadata"] = metadata_to_json(&meta);
                }
                obj
            })
            .collect();

        serde_json::to_string(&json_results)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// List documents with optional filter, ordering, and pagination.
    /// Like SQL: SELECT * WHERE filter ORDER BY field LIMIT n OFFSET m
    /// order_field: metadata field to sort by (empty string = no ordering)
    /// order_desc: true for descending, false for ascending
    #[wasm_bindgen]
    pub fn list_documents(
        &self,
        filter_json: &str,
        order_field: &str,
        order_desc: bool,
        limit: usize,
        offset: usize,
    ) -> Result<String, JsError> {
        let filter = if filter_json.is_empty() || filter_json == "{}" {
            None
        } else {
            Some(parse_filter_json(filter_json)?)
        };

        let order = if order_field.is_empty() {
            None
        } else {
            Some(if order_desc {
                crate::query::OrderBy::desc(order_field)
            } else {
                crate::query::OrderBy::asc(order_field)
            })
        };

        let page = self.inner
            .list_documents(filter, order, limit, offset)
            .map_err(|e| JsError::new(&e.to_string()))?;

        let total = page.total;
        let has_more = page.has_more();

        let items: Vec<serde_json::Value> = page.items
            .into_iter()
            .map(|r| {
                let mut obj = serde_json::json!({ "id": r.id });
                if let Some(meta) = r.metadata {
                    obj["metadata"] = metadata_to_json(&meta);
                }
                obj
            })
            .collect();

        let result = serde_json::json!({
            "items": items,
            "total": total,
            "offset": offset,
            "limit": limit,
            "has_more": has_more,
        });

        serde_json::to_string(&result)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Vector search with metadata filter.
    /// Returns JSON array of results.
    #[wasm_bindgen]
    pub fn search_with_filter(&self, query: &[f32], k: usize, filter_json: &str) -> Result<String, JsError> {
        let filter = parse_filter_json(filter_json)?;
        let results = self.inner
            .search_with_filter(query, k, filter)
            .map_err(|e| JsError::new(&e.to_string()))?;

        let json_results: Vec<serde_json::Value> = results
            .into_iter()
            .map(|r| {
                let mut obj = serde_json::json!({ "id": r.id, "distance": r.distance });
                if let Some(meta) = r.metadata {
                    obj["metadata"] = metadata_to_json(&meta);
                }
                obj
            })
            .collect();

        serde_json::to_string(&json_results)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Paginated vector search. Returns JSON with items + pagination metadata.
    #[wasm_bindgen]
    pub fn search_paged(&self, query: &[f32], limit: usize, offset: usize) -> Result<String, JsError> {
        let page = self.inner
            .search_paged(query, limit, offset)
            .map_err(|e| JsError::new(&e.to_string()))?;

        let total = page.total;
        let has_more = page.has_more();

        let items: Vec<serde_json::Value> = page.items
            .into_iter()
            .map(|r| {
                let mut obj = serde_json::json!({ "id": r.id, "distance": r.distance });
                if let Some(meta) = r.metadata {
                    obj["metadata"] = metadata_to_json(&meta);
                }
                obj
            })
            .collect();

        let result = serde_json::json!({
            "items": items,
            "total": total,
            "offset": offset,
            "limit": limit,
            "has_more": has_more,
        });

        serde_json::to_string(&result)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    // =========================================================================
    // Persistence: export/import for IndexedDB, localStorage, R2, etc.
    // =========================================================================

    /// Export entire database as JSON snapshot for persistence.
    /// Returns JSON string that can be saved to IndexedDB, localStorage, etc.
    #[wasm_bindgen]
    pub fn export_snapshot(&self) -> Result<String, JsError> {
        db_export_snapshot(&self.inner)
    }

    /// Import database from a JSON snapshot (created by export_snapshot).
    /// Clears existing data before importing.
    #[wasm_bindgen]
    pub fn import_snapshot(&self, json: &str) -> Result<usize, JsError> {
        db_import_snapshot(&self.inner, json)
    }

    // =========================================================================
    // Metadata indexes (opt-in, accelerate $eq and range filters)
    // =========================================================================

    /// Crea un índice de metadata opt-in sobre `field`. Es retroactivo: indexa
    /// automáticamente los documentos ya presentes (no hay que reinsertar).
    ///
    /// Acelera los filtros `$eq` y de rango (`$gt`, `$gte`, `$lt`, `$lte`)
    /// resueltos por `filter_search`, `list_documents` y `search_with_filter`
    /// a través del query planner interno. Los resultados no cambian, sólo la
    /// velocidad: el índice nunca altera qué documentos coinciden.
    ///
    /// # Persistencia
    ///
    /// Los índices **no** se serializan en `export_snapshot` (éste sólo vuelca
    /// ids, vectores y metadata). `import_snapshot` sobre una `WasmVectorDB`
    /// que ya tenga índices registrados **los conserva**: el `clear` interno
    /// vacía los buckets pero mantiene los campos indexados, y las inserciones
    /// del import los repueblan. En cambio, importar el snapshot en una
    /// `WasmVectorDB` recién construida arranca sin índices y hay que
    /// recrearlos con este método (que indexa retroactivamente lo importado).
    #[wasm_bindgen]
    pub fn create_metadata_index(&self, field: &str) -> Result<(), JsError> {
        self.inner
            .create_metadata_index(field)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Elimina el índice de metadata sobre `field`. Las consultas sobre ese
    /// campo vuelven a resolverse por full-scan (mismos resultados, sólo más
    /// lento). Los índices restantes se mantienen intactos.
    #[wasm_bindgen]
    pub fn drop_metadata_index(&self, field: &str) -> Result<(), JsError> {
        self.inner
            .drop_metadata_index(field)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Lista los campos con índice de metadata registrado, en orden
    /// lexicográfico. Devuelve un JSON array de strings, p.ej. `["category","price"]`.
    #[wasm_bindgen]
    pub fn list_metadata_indexes(&self) -> String {
        let indexes = self.inner.list_metadata_indexes();
        serde_json::to_string(&indexes).unwrap_or_else(|_| "[]".to_string())
    }
}

// ============================================================================
// OKF — Open Knowledge Format
// ============================================================================

/// Índice OKF (Open Knowledge Format) para WebAssembly.
///
/// Ingiere conceptos OKF (markdown + frontmatter YAML con campo `type`) y los
/// busca por keywords (BM25) con filtro por `okf_type`.
///
/// # Limitación v1
///
/// Sólo modo BM25: sin vectores ni `embed_fn`. La búsqueda semántica/híbrida
/// requeriría un callback JS→Rust de embeddings, que queda fuera de esta v1.
/// En consecuencia todos los chunks se insertan sin vector.
#[wasm_bindgen]
pub struct WasmOkfIndex {
    inner: RustOkfIndex,
}

#[wasm_bindgen]
impl WasmOkfIndex {
    /// Crea un índice OKF en modo solo-BM25 con chunking por defecto.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<WasmOkfIndex, JsError> {
        let cfg = OkfConfig::new(RustChunkConfig::default());
        let idx = RustOkfIndex::new(cfg).map_err(|e| JsError::new(&e.to_string()))?;
        Ok(Self { inner: idx })
    }

    /// Crea un índice OKF con chunking de tamaño fijo + overlap.
    ///
    /// # Arguments
    /// * `target_size` - Tamaño objetivo de cada chunk (caracteres).
    /// * `overlap` - Caracteres de overlap entre chunks consecutivos.
    #[wasm_bindgen]
    pub fn with_chunk_size(target_size: usize, overlap: usize) -> Result<WasmOkfIndex, JsError> {
        let chunk = RustChunkConfig::new(ChunkStrategy::BySize { target_size, overlap });
        let cfg = OkfConfig::new(chunk);
        let idx = RustOkfIndex::new(cfg).map_err(|e| JsError::new(&e.to_string()))?;
        Ok(Self { inner: idx })
    }

    /// Ingerea un concepto desde string (portable). Reemplaza los chunks previos
    /// del mismo `concept_id` (upsert idempotente). Devuelve la cantidad de
    /// chunks insertados (`0` si se salta por falta de `type` o frontmatter roto).
    #[wasm_bindgen]
    pub fn ingest_concept(&self, concept_id: &str, content: &str) -> Result<usize, JsError> {
        self.inner
            .ingest_concept(concept_id, content)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Busca conceptos por keywords (BM25). Retorna un JSON array de hits:
    /// `[{ concept_id, chunk_id, score, title?, snippet }, ...]`.
    ///
    /// `type_filter` restringe a un `type` OKF concreto (`null` = sin filtro).
    #[wasm_bindgen]
    pub fn search(
        &self,
        query: &str,
        k: usize,
        type_filter: Option<String>,
    ) -> Result<String, JsError> {
        let hits = self
            .inner
            .search(query, k, type_filter.as_deref())
            .map_err(|e| JsError::new(&e.to_string()))?;

        let arr: Vec<serde_json::Value> = hits
            .iter()
            .map(|h| {
                let mut obj = serde_json::json!({
                    "concept_id": h.concept_id,
                    "chunk_id": h.chunk_id,
                    "score": h.score,
                    "snippet": h.snippet,
                });
                if let Some(ref t) = h.title {
                    obj["title"] = serde_json::Value::String(t.clone());
                }
                obj
            })
            .collect();

        serde_json::to_string(&arr).map_err(|e| JsError::new(&e.to_string()))
    }

    /// Lista los Concept IDs únicos ingeridos como JSON array de strings.
    #[wasm_bindgen]
    pub fn concepts(&self) -> String {
        let c = self.inner.concepts();
        serde_json::to_string(&c).unwrap_or_else(|_| "[]".to_string())
    }

    /// Borra todos los chunks de un concepto. Devuelve la cantidad borrada.
    #[wasm_bindgen]
    pub fn remove_concept(&self, concept_id: &str) -> Result<usize, JsError> {
        self.inner
            .remove_concept(concept_id)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Número de documentos (chunks) en el índice.
    #[wasm_bindgen]
    pub fn len(&self) -> usize {
        self.inner.db().len()
    }

    /// Verifica si el índice está vacío.
    #[wasm_bindgen]
    pub fn is_empty(&self) -> bool {
        self.inner.db().is_empty()
    }

    /// Exporta el índice como JSON snapshot (ids, vectores, metadata).
    ///
    /// # Round-trip del snapshot
    ///
    /// `OkfIndex` no mantiene un registro de conceptos separado: `concepts()`
    /// se deriva de los documentos de la [`RustVectorDB`] subyacente (campo de
    /// metadata `okf_concept`). El snapshot vuelca todos los documentos con su
    /// metadata, así que `import_snapshot` **restaura los conceptos**: vuelven
    /// a listarse y a ser buscables.
    ///
    /// El metadata index sobre `okf_type` (creado en `OkfIndex::new`) **no se
    /// serializa** en el snapshot, pero: (a) en la MISMA instancia, el `clear`
    /// interno preserva el registro del índice y las reinserciones lo repueblan,
    /// así que el filtro por `okf_type` sigue funcionando tras importar; (b) en
    /// una instancia RECIENTE construida con `new`/`with_chunk_size`, el
    /// constructor recrea el índice sobre la DB vacía antes del import, y las
    /// inserciones del import lo pueblan incrementalmente. En ambos casos el
    /// round-trip restaura por completo conceptos, búsqueda y filtro.
    #[wasm_bindgen]
    pub fn export_snapshot(&self) -> Result<String, JsError> {
        db_export_snapshot(self.inner.db())
    }

    /// Importa un JSON snapshot (de [`export_snapshot`](Self::export_snapshot)),
    /// reemplazando el contenido del índice. Devuelve la cantidad de documentos
    /// importados. Ver [`export_snapshot`](Self::export_snapshot) para el
    /// comportamiento del round-trip de conceptos e índice de metadata.
    #[wasm_bindgen]
    pub fn import_snapshot(&self, json: &str) -> Result<usize, JsError> {
        db_import_snapshot(self.inner.db(), json)
    }
}

/// Exporta una [`RustVectorDB`] como JSON snapshot (ids, vectores, metadata).
/// Lógica compartida entre [`WasmVectorDB`] y [`WasmOkfIndex`].
fn db_export_snapshot(db: &RustVectorDB) -> Result<String, JsError> {
    let ids = db.list_ids()
        .map_err(|e| JsError::new(&e.to_string()))?;

    let mut entries = Vec::new();
    for id in &ids {
        if let Ok(Some((vector, metadata))) = db.get(id) {
            let mut entry = serde_json::json!({ "id": id });
            if let Some(vec) = vector {
                entry["vector"] = serde_json::json!(vec);
            }
            if let Some(meta) = metadata {
                entry["metadata"] = metadata_to_json(&meta);
            }
            entries.push(entry);
        }
    }

    serde_json::to_string(&entries)
        .map_err(|e| JsError::new(&e.to_string()))
}

/// Importa un JSON snapshot (de [`db_export_snapshot`]) en una [`RustVectorDB`],
/// reemplazando su contenido. Valida y parsea TODO antes de tocar el estado.
fn db_import_snapshot(db: &RustVectorDB, json: &str) -> Result<usize, JsError> {
    let entries: Vec<serde_json::Value> = serde_json::from_str(json)
        .map_err(|e| JsError::new(&format!("Invalid JSON: {}", e)))?;

    let dimensions = db.dimensions();

    // Validar y parsear COMPLETAMENTE el snapshot antes de tocar el estado.
    let mut parsed: Vec<(String, Option<Vec<f32>>, RustMetadata)> =
        Vec::with_capacity(entries.len());
    for entry in &entries {
        let id = entry["id"].as_str()
            .ok_or_else(|| JsError::new("Missing 'id' field in snapshot entry"))?
            .to_string();

        let vector: Option<Vec<f32>> = entry
            .get("vector")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|x| {
                        x.as_f64()
                            .map(|f| f as f32)
                            .ok_or_else(|| JsError::new(&format!(
                                "Invalid vector element in entry '{}'",
                                id
                            )))
                    })
                    .collect::<Result<Vec<f32>, JsError>>()
            })
            .transpose()?;

        if let Some(ref vec) = vector {
            if vec.len() != dimensions {
                return Err(JsError::new(&format!(
                    "Vector dimension mismatch for entry '{}': expected {}, got {}",
                    id,
                    dimensions,
                    vec.len()
                )));
            }
        }

        let metadata_str = entry
            .get("metadata")
            .map(|m| m.to_string())
            .unwrap_or_else(|| "{}".to_string());
        let meta = parse_metadata_json(&metadata_str)?;

        parsed.push((id, vector, meta));
    }

    // Solo si todo es valido, reemplazamos el contenido.
    db.clear();

    let mut imported = 0;
    for (id, vector, meta) in parsed {
        if let Some(vec) = vector {
            db.insert(&id, &vec, Some(meta))
                .map_err(|e| JsError::new(&e.to_string()))?;
        } else {
            db.insert_document(&id, None, Some(meta))
                .map_err(|e| JsError::new(&e.to_string()))?;
        }
        imported += 1;
    }

    Ok(imported)
}

/// Trunca un vector a las dimensiones especificadas y lo normaliza.
/// Requerido para Matryoshka embeddings (ej: Gemma 768d -> 256d).
fn truncate_and_normalize(vector: &[f32], target_dims: usize) -> Vec<f32> {
    // Truncar a las dimensiones objetivo
    let truncated: Vec<f32> = vector.iter().take(target_dims).copied().collect();

    // Calcular norma L2
    let norm: f32 = truncated.iter().map(|x| x * x).sum::<f32>().sqrt();

    // Normalizar (evitar division por cero)
    if norm > 1e-10 {
        truncated.iter().map(|x| x / norm).collect()
    } else {
        truncated
    }
}

/// Parsea string de distancia a enum
fn parse_distance(distance: &str) -> Result<RustDistance, JsError> {
    match distance {
        "cosine" | "cos" => Ok(RustDistance::Cosine),
        "euclidean" | "l2" => Ok(RustDistance::Euclidean),
        "dot" | "dot_product" | "inner" => Ok(RustDistance::DotProduct),
        "manhattan" | "l1" => Ok(RustDistance::Manhattan),
        d => Err(JsError::new(&format!(
            "Unknown distance: {}. Use 'cosine', 'euclidean', 'dot', or 'manhattan'",
            d
        ))),
    }
}

/// Parsea string de indice a enum
fn parse_index(index_type: &str) -> Result<RustIndexType, JsError> {
    match index_type {
        "flat" | "brute" | "exact" => Ok(RustIndexType::Flat),
        "hnsw" => Ok(RustIndexType::HNSW {
            m: 16,
            ef_construction: 200,
        }),
        i => Err(JsError::new(&format!(
            "Unknown index: {}. Use 'flat' or 'hnsw'",
            i
        ))),
    }
}

/// Parsea un JSON string a Metadata
fn parse_metadata_json(json: &str) -> Result<RustMetadata, JsError> {
    let value: serde_json::Value = serde_json::from_str(json)
        .map_err(|e| JsError::new(&format!("Invalid JSON: {}", e)))?;

    let mut meta = RustMetadata::new();

    if let serde_json::Value::Object(map) = value {
        for (key, val) in map {
            meta.insert(&key, json_to_metadata_value(&val));
        }
    }

    Ok(meta)
}

/// Parse a JSON filter string into a Filter.
/// Supports: {"field": "value"}, {"field": {"$gt": 5}}, {"$and": [...]}
fn parse_filter_json(json: &str) -> Result<crate::query::Filter, JsError> {
    let value: serde_json::Value = serde_json::from_str(json)
        .map_err(|e| JsError::new(&format!("Invalid filter JSON: {}", e)))?;

    parse_filter_value(&value).map_err(|e| JsError::new(&e))
}

/// Lógica pura de parseo de filtro. Devuelve el mensaje de error como `String`
/// para ser testeable en targets no-wasm (sin construir `JsError`).
fn parse_filter_value(value: &serde_json::Value) -> Result<crate::query::Filter, String> {
    use crate::query::Filter;

    let serde_json::Value::Object(map) = value else {
        return Err("Filter must be a JSON object".to_string());
    };

    let mut filters: Vec<Filter> = Vec::new();

    for (key, val) in map {
        if key == "$and" || key == "$or" {
            let arr = val
                .as_array()
                .ok_or_else(|| format!("'{}' must be an array of filter objects", key))?;
            let sub: Vec<Filter> = arr
                .iter()
                .map(parse_filter_value)
                .collect::<Result<Vec<_>, _>>()?;
            filters.push(if key == "$and" {
                Filter::all(sub)
            } else {
                Filter::any(sub)
            });
        } else if let serde_json::Value::Object(ops) = val {
            // Operator: {"field": {"$gt": 5}}
            if ops.is_empty() {
                return Err(format!("Empty operator object for field '{}'", key));
            }
            for (op, target) in ops {
                let f = match op.as_str() {
                    "$eq" => Filter::eq(key.as_str(), json_to_metadata_value(target)),
                    "$ne" => Filter::ne(key.as_str(), json_to_metadata_value(target)),
                    "$gt" => Filter::gt(key.as_str(), json_to_metadata_value(target)),
                    "$gte" => Filter::gte(key.as_str(), json_to_metadata_value(target)),
                    "$lt" => Filter::lt(key.as_str(), json_to_metadata_value(target)),
                    "$lte" => Filter::lte(key.as_str(), json_to_metadata_value(target)),
                    "$contains" => {
                        let s = target.as_str().ok_or_else(|| {
                            format!("'$contains' for field '{}' must be a string", key)
                        })?;
                        Filter::contains(key.as_str(), s)
                    }
                    "$regex" => {
                        let s = target.as_str().ok_or_else(|| {
                            format!("'$regex' for field '{}' must be a string", key)
                        })?;
                        Filter::regex(key.as_str(), s)
                    }
                    other => {
                        return Err(format!(
                            "Unknown filter operator '{}' for field '{}'",
                            other, key
                        ))
                    }
                };
                filters.push(f);
            }
        } else {
            // Simple equality: {"field": "value"}
            filters.push(Filter::eq(key.as_str(), json_to_metadata_value(val)));
        }
    }

    if filters.is_empty() {
        Err("Empty filter".to_string())
    } else if filters.len() == 1 {
        Ok(filters.into_iter().next().unwrap())
    } else {
        Ok(Filter::all(filters))
    }
}

fn json_to_metadata_value(val: &serde_json::Value) -> crate::types::MetadataValue {
    match val {
        serde_json::Value::String(s) => crate::types::MetadataValue::String(s.clone()),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                crate::types::MetadataValue::Int(i)
            } else if let Some(f) = n.as_f64() {
                crate::types::MetadataValue::Float(f)
            } else {
                crate::types::MetadataValue::Int(0)
            }
        }
        serde_json::Value::Bool(b) => crate::types::MetadataValue::Bool(*b),
        serde_json::Value::Array(arr) => crate::types::MetadataValue::List(
            arr.iter().map(json_to_metadata_value).collect(),
        ),
        serde_json::Value::Object(obj) => crate::types::MetadataValue::Map(
            obj.iter()
                .map(|(k, v)| (k.clone(), json_to_metadata_value(v)))
                .collect(),
        ),
        serde_json::Value::Null => crate::types::MetadataValue::String("null".to_string()),
    }
}

/// Convierte un MetadataValue individual a JSON
fn metadata_value_to_json(value: &crate::types::MetadataValue) -> serde_json::Value {
    match value {
        crate::types::MetadataValue::String(s) => serde_json::Value::String(s.clone()),
        crate::types::MetadataValue::Int(i) => serde_json::Value::Number((*i).into()),
        crate::types::MetadataValue::Float(f) => {
            serde_json::Number::from_f64(*f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null)
        }
        crate::types::MetadataValue::Bool(b) => serde_json::Value::Bool(*b),
        crate::types::MetadataValue::List(l) => {
            serde_json::Value::Array(l.iter().map(|v| metadata_value_to_json(v)).collect())
        }
        crate::types::MetadataValue::Map(m) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in m {
                obj.insert(k.clone(), metadata_value_to_json(v));
            }
            serde_json::Value::Object(obj)
        }
    }
}

/// Convierte Metadata a JSON Value
fn metadata_to_json(meta: &RustMetadata) -> serde_json::Value {
    let mut map = serde_json::Map::new();

    for (key, value) in &meta.fields {
        let json_val = match value {
            crate::types::MetadataValue::String(s) => serde_json::Value::String(s.clone()),
            crate::types::MetadataValue::Int(i) => serde_json::Value::Number((*i).into()),
            crate::types::MetadataValue::Float(f) => {
                serde_json::Number::from_f64(*f)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            }
            crate::types::MetadataValue::Bool(b) => serde_json::Value::Bool(*b),
            crate::types::MetadataValue::List(l) => {
                serde_json::Value::Array(l.iter().map(|v| metadata_value_to_json(v)).collect())
            }
            crate::types::MetadataValue::Map(m) => {
                let mut obj = serde_json::Map::new();
                for (k, v) in m {
                    obj.insert(k.clone(), metadata_value_to_json(v));
                }
                serde_json::Value::Object(obj)
            }
        };
        map.insert(key.clone(), json_val);
    }

    serde_json::Value::Object(map)
}

#[cfg(all(test, feature = "wasm"))]
mod tests {
    use super::*;
    use crate::types::MetadataValue;
    use std::collections::HashMap;

    #[test]
    fn parse_metadata_preserves_scalars_list_and_map() {
        let json = r#"{"name":"x","score":5,"active":true,"tags":["a","b"],"nested":{"k":1}}"#;
        let meta = parse_metadata_json(json).unwrap();
        assert_eq!(meta.get("name").unwrap(), &MetadataValue::String("x".to_string()));
        assert_eq!(meta.get("score").unwrap(), &MetadataValue::Int(5));
        assert_eq!(meta.get("active").unwrap(), &MetadataValue::Bool(true));
        match meta.get("tags").unwrap() {
            MetadataValue::List(l) => {
                assert_eq!(l.len(), 2);
                assert_eq!(l[0], MetadataValue::String("a".to_string()));
                assert_eq!(l[1], MetadataValue::String("b".to_string()));
            }
            v => panic!("expected List, got {:?}", v),
        }
        match meta.get("nested").unwrap() {
            MetadataValue::Map(m) => {
                assert_eq!(m.get("k").unwrap(), &MetadataValue::Int(1));
            }
            v => panic!("expected Map, got {:?}", v),
        }
    }

    #[test]
    fn metadata_roundtrip_export_import_preserves_list_and_map() {
        let mut meta = RustMetadata::new();
        meta.insert(
            "tags",
            MetadataValue::List(vec![
                MetadataValue::String("a".into()),
                MetadataValue::Int(1),
            ]),
        );
        let mut nested = HashMap::new();
        nested.insert("k".to_string(), MetadataValue::Int(1));
        meta.insert("nested", MetadataValue::Map(nested));

        let exported = metadata_to_json(&meta);
        let s = serde_json::to_string(&exported).unwrap();
        let reimported = parse_metadata_json(&s).unwrap();
        let re_exported = metadata_to_json(&reimported);
        assert_eq!(exported, re_exported);
    }

    /// Helper: parsea JSON valido y aplica la logica pura de filtro
    /// (sin construir `JsError`, para correr en targets no-wasm).
    fn parse_filter(s: &str) -> Result<crate::query::Filter, String> {
        let v: serde_json::Value = serde_json::from_str(s).unwrap();
        parse_filter_value(&v)
    }

    #[test]
    fn parse_filter_rejects_unknown_operator() {
        assert!(parse_filter(r#"{"field":{"$foo":1}}"#).is_err());
    }

    #[test]
    fn parse_filter_rejects_and_not_array() {
        assert!(parse_filter(r#"{"$and":{"x":1}}"#).is_err());
        assert!(parse_filter(r#"{"$or":{"x":1}}"#).is_err());
    }

    #[test]
    fn parse_filter_rejects_contains_non_string() {
        assert!(parse_filter(r#"{"field":{"$contains":5}}"#).is_err());
    }

    #[test]
    fn parse_filter_rejects_regex_non_string() {
        assert!(parse_filter(r#"{"field":{"$regex":5}}"#).is_err());
    }

    #[test]
    fn parse_filter_rejects_empty_operator_object() {
        assert!(parse_filter(r#"{"field":{}}"#).is_err());
    }

    #[test]
    fn parse_filter_rejects_non_object_top_level() {
        assert!(parse_filter_value(&serde_json::json!([1, 2])).is_err());
    }

    #[test]
    fn parse_filter_valid_equality_still_works() {
        assert!(parse_filter(r#"{"category":"tech"}"#).is_ok());
    }

    #[test]
    fn parse_filter_valid_operators_still_works() {
        assert!(parse_filter(r#"{"field":{"$gt":5}}"#).is_ok());
        assert!(parse_filter(r#"{"field":{"$contains":"substr"}}"#).is_ok());
        assert!(parse_filter(r#"{"$and":[{"a":1},{"b":2}]}"#).is_ok());
        assert!(parse_filter(r#"{"$or":[{"a":1},{"b":2}]}"#).is_ok());
    }

    /// Helper: DB diminuta con 4 docs categorizados, para ejercitar índices.
    fn categorized_db() -> WasmVectorDB {
        let db = WasmVectorDB::new(2, "cosine", "flat").unwrap();
        db.insert_with_metadata("a", &[1.0, 0.0], r#"{"category":"tech"}"#).unwrap();
        db.insert_with_metadata("b", &[0.0, 1.0], r#"{"category":"tech"}"#).unwrap();
        db.insert_with_metadata("c", &[1.0, 1.0], r#"{"category":"sports"}"#).unwrap();
        db.insert_with_metadata("d", &[0.0, 0.0], r#"{"category":"news"}"#).unwrap();
        db
    }

    /// Extrae los IDs de un JSON array de resultados de filter_search, ordenados.
    /// El orden de filter_search con índice no es determinista (HashSet), así
    /// que comparamos como conjunto, no como string.
    fn result_ids(json: &str) -> Vec<String> {
        let arr: Vec<serde_json::Value> = serde_json::from_str(json).unwrap();
        let mut ids: Vec<String> = arr.iter().map(|v| v["id"].as_str().unwrap().to_string()).collect();
        ids.sort();
        ids
    }

    #[test]
    fn metadata_index_keeps_filter_results_and_lists_state() {
        let db = categorized_db();

        // Sin índice: la lista está vacía y filter_search funciona por full-scan.
        assert_eq!(db.list_metadata_indexes(), "[]");
        let without_idx = db.filter_search(r#"{"category":"tech"}"#, 100).unwrap();
        let expected = result_ids(&without_idx);
        assert_eq!(expected, vec!["a".to_string(), "b".to_string()]);

        // Crear índice retroactivo sobre "category" y verificar que list lo refleja.
        db.create_metadata_index("category").unwrap();
        let parsed: Vec<String> = serde_json::from_str(&db.list_metadata_indexes()).unwrap();
        assert_eq!(parsed, vec!["category".to_string()]);

        // Con índice: mismos resultados que sin índice (el planner sólo acelera).
        let with_idx = db.filter_search(r#"{"category":"tech"}"#, 100).unwrap();
        assert_eq!(result_ids(&with_idx), expected);

        // Drop: la lista vuelve a estar vacía y los resultados siguen idénticos.
        db.drop_metadata_index("category").unwrap();
        assert_eq!(db.list_metadata_indexes(), "[]");
        let after_drop = db.filter_search(r#"{"category":"tech"}"#, 100).unwrap();
        assert_eq!(result_ids(&after_drop), expected);
    }

    #[test]
    fn import_snapshot_into_fresh_db_loses_indexes_but_same_db_keeps_them() {
        let db = categorized_db();
        db.create_metadata_index("category").unwrap();
        let expected = vec!["a".to_string(), "b".to_string()];

        // Misma DB: export -> import conserva el índice (clear mantiene los
        // registros y las reinserciones los repueblan).
        let snap = db.export_snapshot().unwrap();
        db.import_snapshot(&snap).unwrap();
        assert_eq!(db.list_metadata_indexes(), r#"["category"]"#);
        let via_idx = db.filter_search(r#"{"category":"tech"}"#, 100).unwrap();
        assert_eq!(result_ids(&via_idx), expected);

        // DB fresca: el snapshot no lleva los índices, hay que recrearlos.
        let fresh = WasmVectorDB::new(2, "cosine", "flat").unwrap();
        fresh.import_snapshot(&snap).unwrap();
        assert_eq!(fresh.list_metadata_indexes(), "[]");
        fresh.create_metadata_index("category").unwrap();
        assert_eq!(fresh.list_metadata_indexes(), r#"["category"]"#);
        assert_eq!(
            result_ids(&fresh.filter_search(r#"{"category":"tech"}"#, 100).unwrap()),
            expected
        );
    }

    // =========================================================================
    // OKF (WasmOkfIndex)
    // =========================================================================

    fn okf_concept(_id: &str, type_: &str, title: &str, body: &str) -> String {
        format!("---\ntype: {type_}\ntitle: {title}\n---\n{body}\n")
    }

    /// IDs de concepto de un JSON array de hits de search, ordenados.
    fn hit_concepts(json: &str) -> Vec<String> {
        let arr: Vec<serde_json::Value> = serde_json::from_str(json).unwrap();
        let mut ids: Vec<String> = arr
            .iter()
            .map(|v| v["concept_id"].as_str().unwrap().to_string())
            .collect();
        ids.sort();
        ids.dedup();
        ids
    }

    /// Cantidad de hits en un JSON array de search.
    fn hit_len(json: &str) -> usize {
        let arr: Vec<serde_json::Value> = serde_json::from_str(json).unwrap();
        arr.len()
    }

    /// Concept IDs de `concepts()`, ordenados (el orden de list_documents no
    /// está garantizado).
    fn sorted_concepts(json: &str) -> Vec<String> {
        let mut v: Vec<String> = serde_json::from_str(json).unwrap();
        v.sort();
        v
    }

    #[test]
    fn okf_ingest_and_search_with_and_without_type_filter() {
        let idx = WasmOkfIndex::new().unwrap();
        assert!(idx.is_empty());
        idx.ingest_concept("a", &okf_concept("a", "doc", "Alpha", "rust programming language"))
            .unwrap();
        idx.ingest_concept("b", &okf_concept("b", "note", "Beta", "rust memory safety notes"))
            .unwrap();
        assert!(!idx.is_empty());
        assert_eq!(idx.len(), 2); // 2 chunks (un cuerpo corto = 1 chunk cado)

        // concepts() lista ambos (el orden de list_documents no es garantizado).
        assert_eq!(
            sorted_concepts(&idx.concepts()),
            vec!["a".to_string(), "b".to_string()]
        );

        // Sin filtro: ambos conceptos aparecen.
        let all = idx.search("rust", 10, None).unwrap();
        let both = hit_concepts(&all);
        assert!(both.contains(&"a".to_string()));
        assert!(both.contains(&"b".to_string()));

        // Con filtro doc: sólo "a".
        let only_doc = idx.search("rust", 10, Some("doc".to_string())).unwrap();
        let doc_hits = hit_concepts(&only_doc);
        assert_eq!(doc_hits, vec!["a".to_string()]);
    }

    #[test]
    fn okf_remove_concept_drops_chunks_and_search() {
        let idx = WasmOkfIndex::new().unwrap();
        idx.ingest_concept("c", &okf_concept("c", "doc", "C", "old content alpha"))
            .unwrap();
        assert_ne!(hit_len(&idx.search("alpha", 10, None).unwrap()), 0);

        let removed = idx.remove_concept("c").unwrap();
        assert!(removed >= 1);
        assert_eq!(hit_len(&idx.search("alpha", 10, None).unwrap()), 0);

        assert!(sorted_concepts(&idx.concepts()).is_empty());
    }

    #[test]
    fn okf_ingest_skipped_returns_zero_without_error() {
        let idx = WasmOkfIndex::new().unwrap();
        // Sin campo type → saltado, 0 chunks, sin error.
        let n = idx.ingest_concept("x", "---\ntitle: no type\n---\nbody").unwrap();
        assert_eq!(n, 0);
        assert!(sorted_concepts(&idx.concepts()).is_empty());
    }

    #[test]
    fn okf_with_chunk_size_constructs_and_ingests() {
        let idx = WasmOkfIndex::with_chunk_size(50, 10).unwrap();
        let n = idx
            .ingest_concept("big", &okf_concept("big", "doc", "Big", "alpha beta gamma delta epsilon zeta"))
            .unwrap();
        assert!(n >= 1);
        assert!(idx.len() >= 1);
    }

    #[test]
    fn okf_snapshot_roundtrip_restores_concepts_search_and_type_filter() {
        let idx = WasmOkfIndex::new().unwrap();
        idx.ingest_concept("a", &okf_concept("a", "doc", "Alpha", "rust programming"))
            .unwrap();
        idx.ingest_concept("b", &okf_concept("b", "note", "Beta", "rust memory notes"))
            .unwrap();
        let snap = idx.export_snapshot().unwrap();

        // Instancia fresca: el constructor recrea el índice okf_type sobre la
        // DB vacía; el import inserta los docs (con su metadata) y puebla el
        // índice. Conceptos, búsqueda y filtro quedan restaurados.
        let fresh = WasmOkfIndex::new().unwrap();
        let imported = fresh.import_snapshot(&snap).unwrap();
        assert_eq!(imported, 2);

        assert_eq!(
            sorted_concepts(&fresh.concepts()),
            vec!["a".to_string(), "b".to_string()]
        );

        // Búsqueda sin filtro encuentra ambos.
        let all = fresh.search("rust", 10, None).unwrap();
        let both = hit_concepts(&all);
        assert!(both.contains(&"a".to_string()));
        assert!(both.contains(&"b".to_string()));

        // Filtro por okf_type sigue operativo (índice repoblado por el import).
        let only_doc = fresh.search("rust", 10, Some("doc".to_string())).unwrap();
        assert_eq!(hit_concepts(&only_doc), vec!["a".to_string()]);

        // Misma instancia: export -> import conserva todo.
        let snap2 = fresh.export_snapshot().unwrap();
        fresh.import_snapshot(&snap2).unwrap();
        assert_eq!(fresh.len(), 2);
        assert_eq!(
            hit_concepts(&fresh.search("rust", 10, Some("note".to_string())).unwrap()),
            vec!["b".to_string()]
        );
    }
}
