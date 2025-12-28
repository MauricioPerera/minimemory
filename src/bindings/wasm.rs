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
    Config as RustConfig, Distance as RustDistance, IndexType as RustIndexType,
    Metadata as RustMetadata, VectorDB as RustVectorDB,
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
