//! Bindings Python para minimemory usando PyO3.
//!
//! ## Uso en Python
//!
//! ```python
//! from minimemory import VectorDB, Config, Distance
//!
//! # Crear base de datos
//! db = VectorDB(dimensions=384, distance="cosine")
//!
//! # Insertar vectores
//! db.insert("doc1", [0.1, 0.2, ...], {"title": "Mi documento"})
//!
//! # Buscar
//! results = db.search([0.1, 0.2, ...], k=10)
//! for r in results:
//!     print(f"{r.id}: {r.distance}")
//!
//! # Guardar/Cargar
//! db.save("database.mmdb")
//! db = VectorDB.load("database.mmdb")
//! ```

use pyo3::prelude::*;
use pyo3::exceptions::{PyValueError, PyIOError, PyKeyError};
use pyo3::types::{PyDict, PyList};
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

/// Base de datos vectorial embebida.
///
/// Args:
///     dimensions: Número de dimensiones de los vectores
///     distance: Métrica de distancia ("cosine", "euclidean", "dot")
///     index: Tipo de índice ("flat", "hnsw")
///     hnsw_m: Conexiones por nodo para HNSW (default: 16)
///     hnsw_ef: ef_construction para HNSW (default: 200)
#[pyclass(name = "VectorDB")]
pub struct PyVectorDB {
    inner: Arc<RustVectorDB>,
}

#[pymethods]
impl PyVectorDB {
    #[new]
    #[pyo3(signature = (dimensions, distance="cosine", index="flat", hnsw_m=16, hnsw_ef=200))]
    fn new(
        dimensions: usize,
        distance: &str,
        index: &str,
        hnsw_m: usize,
        hnsw_ef: usize,
    ) -> PyResult<Self> {
        let dist = match distance.to_lowercase().as_str() {
            "cosine" | "cos" => RustDistance::Cosine,
            "euclidean" | "l2" => RustDistance::Euclidean,
            "dot" | "dot_product" | "inner" => RustDistance::DotProduct,
            _ => return Err(PyValueError::new_err(format!(
                "Unknown distance: {}. Use 'cosine', 'euclidean', or 'dot'",
                distance
            ))),
        };

        let idx = match index.to_lowercase().as_str() {
            "flat" | "brute" | "exact" => RustIndexType::Flat,
            "hnsw" => RustIndexType::HNSW { m: hnsw_m, ef_construction: hnsw_ef },
            _ => return Err(PyValueError::new_err(format!(
                "Unknown index: {}. Use 'flat' or 'hnsw'",
                index
            ))),
        };

        let config = RustConfig::new(dimensions)
            .with_distance(dist)
            .with_index(idx);

        let db = RustVectorDB::new(config)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        Ok(Self { inner: Arc::new(db) })
    }

    /// Carga una base de datos desde archivo.
    ///
    /// Args:
    ///     path: Ruta al archivo .mmdb
    ///
    /// Returns:
    ///     VectorDB: Base de datos cargada
    #[staticmethod]
    fn load(path: &str) -> PyResult<Self> {
        let db = RustVectorDB::open(path)
            .map_err(|e| PyIOError::new_err(e.to_string()))?;

        Ok(Self { inner: Arc::new(db) })
    }

    /// Inserta un vector en la base de datos.
    ///
    /// Args:
    ///     id: Identificador único
    ///     vector: Lista de floats
    ///     metadata: Diccionario opcional con metadata
    #[pyo3(signature = (id, vector, metadata=None))]
    fn insert(
        &self,
        id: &str,
        vector: Vec<f32>,
        metadata: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        let meta = metadata.map(|m| dict_to_metadata(m)).transpose()?;

        self.inner
            .insert(id, &vector, meta)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Inserta múltiples vectores en lote.
    ///
    /// Args:
    ///     items: Lista de tuplas (id, vector, metadata)
    fn insert_batch(&self, items: Vec<(String, Vec<f32>, Option<&Bound<'_, PyDict>>)>) -> PyResult<()> {
        for (id, vector, metadata) in items {
            let meta = metadata.map(|m| dict_to_metadata(m)).transpose()?;
            self.inner
                .insert(&id, &vector, meta)
                .map_err(|e| PyValueError::new_err(e.to_string()))?;
        }
        Ok(())
    }

    /// Busca los k vectores más similares.
    ///
    /// Args:
    ///     query: Vector de consulta
    ///     k: Número de resultados
    ///
    /// Returns:
    ///     Lista de SearchResult con id, distance y metadata
    fn search(&self, query: Vec<f32>, k: usize) -> PyResult<Vec<PySearchResult>> {
        let results = self.inner
            .search(&query, k)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        Ok(results
            .into_iter()
            .map(|r| PySearchResult {
                id: r.id,
                distance: r.distance,
                metadata: r.metadata.map(metadata_to_dict),
            })
            .collect())
    }

    /// Obtiene un vector por su ID.
    ///
    /// Args:
    ///     id: ID del vector
    ///
    /// Returns:
    ///     Tupla (vector, metadata) o None si no existe
    fn get(&self, id: &str) -> PyResult<Option<(Vec<f32>, Option<HashMap<String, PyObject>>)>> {
        match self.inner.get(id).map_err(|e| PyValueError::new_err(e.to_string()))? {
            Some((vector, metadata)) => {
                let meta = metadata.map(metadata_to_dict);
                Ok(Some((vector, meta)))
            }
            None => Ok(None),
        }
    }

    /// Elimina un vector por su ID.
    ///
    /// Args:
    ///     id: ID del vector
    ///
    /// Returns:
    ///     True si fue eliminado, False si no existía
    fn delete(&self, id: &str) -> PyResult<bool> {
        self.inner
            .delete(id)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Actualiza un vector existente.
    ///
    /// Args:
    ///     id: ID del vector
    ///     vector: Nuevo vector
    ///     metadata: Nueva metadata opcional
    #[pyo3(signature = (id, vector, metadata=None))]
    fn update(
        &self,
        id: &str,
        vector: Vec<f32>,
        metadata: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        let meta = metadata.map(|m| dict_to_metadata(m)).transpose()?;

        self.inner
            .update(id, &vector, meta)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Verifica si un vector existe.
    fn contains(&self, id: &str) -> bool {
        self.inner.contains(id)
    }

    /// Guarda la base de datos a un archivo.
    ///
    /// Args:
    ///     path: Ruta donde guardar
    fn save(&self, path: &str) -> PyResult<()> {
        self.inner
            .save(path)
            .map_err(|e| PyIOError::new_err(e.to_string()))
    }

    /// Limpia todos los vectores.
    fn clear(&self) {
        self.inner.clear();
    }

    /// Número de vectores en la base de datos.
    #[getter]
    fn len(&self) -> usize {
        self.inner.len()
    }

    /// Dimensiones de los vectores.
    #[getter]
    fn dimensions(&self) -> usize {
        self.inner.dimensions()
    }

    /// Verifica si está vacía.
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    fn __contains__(&self, id: &str) -> bool {
        self.inner.contains(id)
    }

    fn __repr__(&self) -> String {
        format!(
            "VectorDB(len={}, dimensions={})",
            self.inner.len(),
            self.inner.dimensions()
        )
    }
}

/// Resultado de una búsqueda.
#[pyclass(name = "SearchResult")]
#[derive(Clone)]
pub struct PySearchResult {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub distance: f32,
    #[pyo3(get)]
    pub metadata: Option<HashMap<String, PyObject>>,
}

#[pymethods]
impl PySearchResult {
    fn __repr__(&self) -> String {
        format!("SearchResult(id='{}', distance={})", self.id, self.distance)
    }
}

/// Convierte un PyDict a Metadata de Rust
fn dict_to_metadata(dict: &Bound<'_, PyDict>) -> PyResult<RustMetadata> {
    let mut meta = RustMetadata::new();

    for (key, value) in dict.iter() {
        let key_str: String = key.extract()?;

        if let Ok(v) = value.extract::<String>() {
            meta.insert(key_str, v);
        } else if let Ok(v) = value.extract::<i64>() {
            meta.insert(key_str, v);
        } else if let Ok(v) = value.extract::<f64>() {
            meta.insert(key_str, v);
        } else if let Ok(v) = value.extract::<bool>() {
            meta.insert(key_str, v);
        } else {
            return Err(PyValueError::new_err(format!(
                "Unsupported metadata type for key '{}'",
                key_str
            )));
        }
    }

    Ok(meta)
}

/// Convierte Metadata de Rust a HashMap para Python
fn metadata_to_dict(meta: RustMetadata) -> HashMap<String, PyObject> {
    Python::with_gil(|py| {
        let mut map = HashMap::new();

        for (key, value) in meta.fields {
            let py_value: PyObject = match value {
                RustMetadataValue::String(s) => s.into_py(py),
                RustMetadataValue::Int(i) => i.into_py(py),
                RustMetadataValue::Float(f) => f.into_py(py),
                RustMetadataValue::Bool(b) => b.into_py(py),
                RustMetadataValue::List(_) => py.None(), // Simplificado
            };
            map.insert(key, py_value);
        }

        map
    })
}

/// Módulo Python minimemory
#[pymodule]
fn minimemory(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyVectorDB>()?;
    m.add_class::<PySearchResult>()?;

    // Constantes de distancia
    m.add("COSINE", "cosine")?;
    m.add("EUCLIDEAN", "euclidean")?;
    m.add("DOT_PRODUCT", "dot")?;

    // Constantes de índice
    m.add("FLAT", "flat")?;
    m.add("HNSW", "hnsw")?;

    Ok(())
}
