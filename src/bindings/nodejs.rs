//! Bindings Node.js/TypeScript para minimemory usando napi-rs.
//!
//! ## Uso en TypeScript/JavaScript
//!
//! ```typescript
//! import { VectorDB } from 'minimemory';
//!
//! // Crear base de datos
//! const db = new VectorDB({
//!   dimensions: 384,
//!   distance: 'cosine',
//!   index: 'hnsw'
//! });
//!
//! // Insertar vectores
//! await db.insert('doc1', [0.1, 0.2, ...], { title: 'Mi documento' });
//!
//! // Buscar
//! const results = await db.search([0.1, 0.2, ...], 10);
//! results.forEach(r => console.log(`${r.id}: ${r.distance}`));
//!
//! // Guardar/Cargar
//! await db.save('database.mmdb');
//! const db2 = await VectorDB.load('database.mmdb');
//! ```

use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::collections::HashMap;
use std::sync::Arc;

use crate::{
    Config as RustConfig,
    Distance as RustDistance,
    IndexType as RustIndexType,
    Metadata as RustMetadata,
    VectorDB as RustVectorDB,
    types::MetadataValue as RustMetadataValue,
};

/// Opciones de configuración para VectorDB
#[napi(object)]
pub struct VectorDBOptions {
    /// Número de dimensiones de los vectores
    pub dimensions: u32,
    /// Métrica de distancia: "cosine", "euclidean", "dot"
    pub distance: Option<String>,
    /// Tipo de índice: "flat", "hnsw"
    pub index: Option<String>,
    /// Parámetro M para HNSW (default: 16)
    pub hnsw_m: Option<u32>,
    /// Parámetro ef_construction para HNSW (default: 200)
    pub hnsw_ef: Option<u32>,
}

/// Resultado de búsqueda
#[napi(object)]
pub struct SearchResult {
    /// ID del vector
    pub id: String,
    /// Distancia al query (menor = más similar)
    pub distance: f64,
    /// Metadata asociada
    pub metadata: Option<HashMap<String, String>>,
}

/// Base de datos vectorial embebida
#[napi]
pub struct VectorDB {
    inner: Arc<RustVectorDB>,
}

#[napi]
impl VectorDB {
    /// Crea una nueva base de datos vectorial.
    #[napi(constructor)]
    pub fn new(options: VectorDBOptions) -> Result<Self> {
        let distance = match options.distance.as_deref().unwrap_or("cosine") {
            "cosine" | "cos" => RustDistance::Cosine,
            "euclidean" | "l2" => RustDistance::Euclidean,
            "dot" | "dot_product" | "inner" => RustDistance::DotProduct,
            d => return Err(Error::new(
                Status::InvalidArg,
                format!("Unknown distance: {}. Use 'cosine', 'euclidean', or 'dot'", d),
            )),
        };

        let index = match options.index.as_deref().unwrap_or("flat") {
            "flat" | "brute" | "exact" => RustIndexType::Flat,
            "hnsw" => RustIndexType::HNSW {
                m: options.hnsw_m.unwrap_or(16) as usize,
                ef_construction: options.hnsw_ef.unwrap_or(200) as usize,
            },
            i => return Err(Error::new(
                Status::InvalidArg,
                format!("Unknown index: {}. Use 'flat' or 'hnsw'", i),
            )),
        };

        let config = RustConfig::new(options.dimensions as usize)
            .with_distance(distance)
            .with_index(index);

        let db = RustVectorDB::new(config)
            .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))?;

        Ok(Self { inner: Arc::new(db) })
    }

    /// Carga una base de datos desde archivo.
    #[napi(factory)]
    pub fn load(path: String) -> Result<Self> {
        let db = RustVectorDB::open(&path)
            .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))?;

        Ok(Self { inner: Arc::new(db) })
    }

    /// Inserta un vector en la base de datos.
    #[napi]
    pub fn insert(
        &self,
        id: String,
        vector: Vec<f64>,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<()> {
        let vector_f32: Vec<f32> = vector.iter().map(|&x| x as f32).collect();
        let meta = metadata.map(|m| hashmap_to_metadata(&m));

        self.inner
            .insert(&id, &vector_f32, meta)
            .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))
    }

    /// Busca los k vectores más similares.
    #[napi]
    pub fn search(&self, query: Vec<f64>, k: u32) -> Result<Vec<SearchResult>> {
        let query_f32: Vec<f32> = query.iter().map(|&x| x as f32).collect();

        let results = self.inner
            .search(&query_f32, k as usize)
            .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))?;

        Ok(results
            .into_iter()
            .map(|r| SearchResult {
                id: r.id,
                distance: r.distance as f64,
                metadata: r.metadata.map(metadata_to_hashmap),
            })
            .collect())
    }

    /// Obtiene un vector por su ID.
    #[napi]
    pub fn get(&self, id: String) -> Result<Option<Vec<f64>>> {
        match self.inner.get(&id).map_err(|e| Error::new(Status::GenericFailure, e.to_string()))? {
            Some((vector, _)) => Ok(Some(vector.iter().map(|&x| x as f64).collect())),
            None => Ok(None),
        }
    }

    /// Elimina un vector por su ID.
    #[napi]
    pub fn delete(&self, id: String) -> Result<bool> {
        self.inner
            .delete(&id)
            .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))
    }

    /// Actualiza un vector existente.
    #[napi]
    pub fn update(
        &self,
        id: String,
        vector: Vec<f64>,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<()> {
        let vector_f32: Vec<f32> = vector.iter().map(|&x| x as f32).collect();
        let meta = metadata.map(|m| hashmap_to_metadata(&m));

        self.inner
            .update(&id, &vector_f32, meta)
            .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))
    }

    /// Verifica si un vector existe.
    #[napi]
    pub fn contains(&self, id: String) -> bool {
        self.inner.contains(&id)
    }

    /// Guarda la base de datos a un archivo.
    #[napi]
    pub fn save(&self, path: String) -> Result<()> {
        self.inner
            .save(&path)
            .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))
    }

    /// Limpia todos los vectores.
    #[napi]
    pub fn clear(&self) {
        self.inner.clear();
    }

    /// Número de vectores en la base de datos.
    #[napi(getter)]
    pub fn length(&self) -> u32 {
        self.inner.len() as u32
    }

    /// Dimensiones de los vectores.
    #[napi(getter)]
    pub fn dimensions(&self) -> u32 {
        self.inner.dimensions() as u32
    }

    /// Verifica si está vacía.
    #[napi(getter)]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

/// Convierte HashMap de strings a Metadata
fn hashmap_to_metadata(map: &HashMap<String, String>) -> RustMetadata {
    let mut meta = RustMetadata::new();
    for (key, value) in map {
        meta.insert(key.clone(), value.clone());
    }
    meta
}

/// Convierte Metadata a HashMap de strings
fn metadata_to_hashmap(meta: RustMetadata) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for (key, value) in meta.fields {
        let str_value = match value {
            RustMetadataValue::String(s) => s,
            RustMetadataValue::Int(i) => i.to_string(),
            RustMetadataValue::Float(f) => f.to_string(),
            RustMetadataValue::Bool(b) => b.to_string(),
            RustMetadataValue::List(_) => "[list]".to_string(),
        };
        map.insert(key, str_value);
    }
    map
}
