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
            "binary" | "bit" | "1bit" => QuantizationType::Binary,
            q => return Err(JsError::new(&format!("Unknown quantization: {}. Use 'none', 'int8', or 'binary'", q))),
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

    /// Exporta la base de datos como JSON.
    #[wasm_bindgen]
    pub fn export_json(&self) -> Result<String, JsError> {
        self.inner
            .export_json()
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Importa datos desde JSON.
    #[wasm_bindgen]
    pub fn import_json(&self, json: &str) -> Result<(), JsError> {
        self.inner
            .import_json(json)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Obtiene todos los IDs.
    #[wasm_bindgen]
    pub fn ids(&self) -> Result<String, JsError> {
        let ids: Vec<String> = self.inner.ids().into_iter().collect();
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
        d => Err(JsError::new(&format!(
            "Unknown distance: {}. Use 'cosine', 'euclidean', or 'dot'",
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
                serde_json::Value::Array(l.iter().map(|s| serde_json::Value::String(s.clone())).collect())
            }
        };
        map.insert(key.clone(), json_val);
    }

    serde_json::Value::Object(map)
}
