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
        let dist = match distance {
            "cosine" | "cos" => RustDistance::Cosine,
            "euclidean" | "l2" => RustDistance::Euclidean,
            "dot" | "dot_product" => RustDistance::DotProduct,
            d => return Err(JsError::new(&format!("Unknown distance: {}", d))),
        };

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
                Ok(JsValue::from_str(&serde_json::to_string(&result).unwrap()))
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
        let ids = self.inner.list_ids()
            .map_err(|e| JsError::new(&e.to_string()))?;

        let mut entries = Vec::new();
        for id in &ids {
            if let Ok(Some((vector, metadata))) = self.inner.get(id) {
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

    /// Import database from a JSON snapshot (created by export_snapshot).
    /// Clears existing data before importing.
    #[wasm_bindgen]
    pub fn import_snapshot(&self, json: &str) -> Result<usize, JsError> {
        let entries: Vec<serde_json::Value> = serde_json::from_str(json)
            .map_err(|e| JsError::new(&format!("Invalid JSON: {}", e)))?;

        self.inner.clear();

        let mut imported = 0;
        for entry in &entries {
            let id = entry["id"].as_str()
                .ok_or_else(|| JsError::new("Missing 'id' field in snapshot entry"))?;

            let vector: Option<Vec<f32>> = entry.get("vector")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|x| x.as_f64().map(|f| f as f32)).collect());

            let metadata_str = entry.get("metadata")
                .map(|m| m.to_string())
                .unwrap_or_else(|| "{}".to_string());

            let meta = parse_metadata_json(&metadata_str)?;

            if let Some(vec) = vector {
                self.inner
                    .insert(id, &vec, Some(meta))
                    .map_err(|e| JsError::new(&e.to_string()))?;
            } else {
                self.inner
                    .insert_document(id, None, Some(meta))
                    .map_err(|e| JsError::new(&e.to_string()))?;
            }
            imported += 1;
        }

        Ok(imported)
    }
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
            match val {
                serde_json::Value::String(s) => {
                    meta.insert(&key, s);
                }
                serde_json::Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        meta.insert(&key, i);
                    } else if let Some(f) = n.as_f64() {
                        meta.insert(&key, f);
                    }
                }
                serde_json::Value::Bool(b) => {
                    meta.insert(&key, b);
                }
                _ => {} // Ignorar arrays y objetos anidados
            }
        }
    }

    Ok(meta)
}

/// Parse a JSON filter string into a Filter.
/// Supports: {"field": "value"}, {"field": {"$gt": 5}}, {"$and": [...]}
fn parse_filter_json(json: &str) -> Result<crate::query::Filter, JsError> {
    let value: serde_json::Value = serde_json::from_str(json)
        .map_err(|e| JsError::new(&format!("Invalid filter JSON: {}", e)))?;

    parse_filter_value(&value)
}

fn parse_filter_value(value: &serde_json::Value) -> Result<crate::query::Filter, JsError> {
    use crate::query::Filter;

    if let serde_json::Value::Object(map) = value {
        let mut filters: Vec<Filter> = Vec::new();

        for (key, val) in map {
            if key == "$and" {
                if let serde_json::Value::Array(arr) = val {
                    let sub: Result<Vec<Filter>, _> = arr.iter().map(parse_filter_value).collect();
                    filters.push(Filter::all(sub?));
                }
            } else if key == "$or" {
                if let serde_json::Value::Array(arr) = val {
                    let sub: Result<Vec<Filter>, _> = arr.iter().map(parse_filter_value).collect();
                    filters.push(Filter::any(sub?));
                }
            } else if let serde_json::Value::Object(ops) = val {
                // Operator: {"field": {"$gt": 5}}
                for (op, target) in ops {
                    let f = match op.as_str() {
                        "$eq" => Filter::eq(key.as_str(), json_to_metadata_value(target)),
                        "$ne" => Filter::ne(key.as_str(), json_to_metadata_value(target)),
                        "$gt" => Filter::gt(key.as_str(), json_to_metadata_value(target)),
                        "$gte" => Filter::gte(key.as_str(), json_to_metadata_value(target)),
                        "$lt" => Filter::lt(key.as_str(), json_to_metadata_value(target)),
                        "$lte" => Filter::lte(key.as_str(), json_to_metadata_value(target)),
                        "$contains" => {
                            if let Some(s) = target.as_str() {
                                Filter::contains(key.as_str(), s)
                            } else {
                                continue;
                            }
                        }
                        "$regex" => {
                            if let Some(s) = target.as_str() {
                                Filter::regex(key.as_str(), s)
                            } else {
                                continue;
                            }
                        }
                        _ => continue,
                    };
                    filters.push(f);
                }
            } else {
                // Simple equality: {"field": "value"}
                filters.push(Filter::eq(key.as_str(), json_to_metadata_value(val)));
            }
        }

        if filters.is_empty() {
            Err(JsError::new("Empty filter"))
        } else if filters.len() == 1 {
            Ok(filters.into_iter().next().unwrap())
        } else {
            Ok(Filter::all(filters))
        }
    } else {
        Err(JsError::new("Filter must be a JSON object"))
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
        _ => crate::types::MetadataValue::String(val.to_string()),
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
