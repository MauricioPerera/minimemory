//! Tipos base para minimemory.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Identificador único para un vector.
///
/// Se usa `String` para permitir IDs descriptivos como UUIDs, slugs, etc.
pub type VectorId = String;

/// Un vector es un slice de valores f32.
///
/// Representa un embedding o vector de características.
pub type Vector = [f32];

/// Metadata asociada a un vector.
///
/// Permite almacenar información adicional junto con cada vector,
/// como títulos, categorías, puntuaciones, etc.
///
/// # Ejemplo
///
/// ```rust
/// use minimemory::Metadata;
///
/// let mut meta = Metadata::new();
/// meta.insert("title", "Mi documento")
///     .insert("score", 42i64)
///     .insert("active", true);
///
/// assert!(meta.get("title").is_some());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Metadata {
    /// Campos de metadata como pares clave-valor.
    pub fields: HashMap<String, MetadataValue>,
}

impl Metadata {
    /// Crea una nueva instancia de Metadata vacía.
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
        }
    }

    /// Inserta un campo de metadata.
    ///
    /// Soporta encadenamiento fluido (builder pattern).
    ///
    /// # Argumentos
    ///
    /// * `key` - Nombre del campo
    /// * `value` - Valor (String, i64, f64, bool)
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<MetadataValue>) -> &mut Self {
        self.fields.insert(key.into(), value.into());
        self
    }

    /// Obtiene un campo de metadata por su clave.
    ///
    /// Retorna `None` si la clave no existe.
    pub fn get(&self, key: &str) -> Option<&MetadataValue> {
        self.fields.get(key)
    }
}

/// Tipos de valores soportados en metadata.
///
/// Soporta los tipos más comunes para almacenar información adicional.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MetadataValue {
    /// Cadena de texto
    String(String),
    /// Entero de 64 bits
    Int(i64),
    /// Punto flotante de 64 bits
    Float(f64),
    /// Booleano
    Bool(bool),
    /// Lista de valores (permite estructuras anidadas)
    List(Vec<MetadataValue>),
    /// Objeto anidado (para estructuras como author.name)
    Map(HashMap<String, MetadataValue>),
}

impl From<String> for MetadataValue {
    fn from(s: String) -> Self {
        MetadataValue::String(s)
    }
}

impl From<&str> for MetadataValue {
    fn from(s: &str) -> Self {
        MetadataValue::String(s.to_string())
    }
}

impl From<i64> for MetadataValue {
    fn from(i: i64) -> Self {
        MetadataValue::Int(i)
    }
}

impl From<f64> for MetadataValue {
    fn from(f: f64) -> Self {
        MetadataValue::Float(f)
    }
}

impl From<bool> for MetadataValue {
    fn from(b: bool) -> Self {
        MetadataValue::Bool(b)
    }
}

/// Resultado de una búsqueda de similitud.
///
/// Contiene el ID del vector encontrado, su distancia al query,
/// y opcionalmente la metadata asociada.
///
/// # Nota sobre distancia
///
/// Un valor de distancia **menor** indica **mayor similitud**.
/// - `distance = 0.0` significa vectores idénticos
/// - Para coseno: rango [0, 2] donde 0 = idéntico, 1 = ortogonal, 2 = opuesto
/// - Para euclidiana: rango [0, ∞)
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// ID del vector encontrado
    pub id: VectorId,
    /// Distancia al vector de consulta (menor = más similar)
    pub distance: f32,
    /// Metadata asociada al vector (si existe)
    pub metadata: Option<Metadata>,
}

/// Configuración para crear una VectorDB.
///
/// Usa el patrón builder para configuración fluida.
///
/// # Ejemplo
///
/// ```rust
/// use minimemory::{Config, Distance, IndexType};
///
/// let config = Config::new(384)           // 384 dimensiones
///     .with_distance(Distance::Cosine)    // Similitud coseno
///     .with_index(IndexType::Flat);       // Búsqueda exacta
/// ```
#[derive(Debug, Clone)]
pub struct Config {
    /// Número de dimensiones para los vectores
    pub dimensions: usize,
    /// Métrica de distancia a usar
    pub distance: crate::Distance,
    /// Tipo de índice para búsqueda
    pub index: crate::IndexType,
}

impl Config {
    /// Crea una nueva configuración con las dimensiones especificadas.
    ///
    /// Valores por defecto:
    /// - Distancia: `Cosine`
    /// - Índice: `Flat`
    ///
    /// # Argumentos
    ///
    /// * `dimensions` - Número de dimensiones de los vectores
    pub fn new(dimensions: usize) -> Self {
        Self {
            dimensions,
            distance: crate::Distance::Cosine,
            index: crate::IndexType::Flat,
        }
    }

    /// Establece la métrica de distancia.
    ///
    /// # Opciones
    ///
    /// - `Distance::Cosine` - Ideal para embeddings de texto
    /// - `Distance::Euclidean` - Para vectores normalizados
    /// - `Distance::DotProduct` - Cuando la magnitud importa
    pub fn with_distance(mut self, distance: crate::Distance) -> Self {
        self.distance = distance;
        self
    }

    /// Establece el tipo de índice.
    ///
    /// # Opciones
    ///
    /// - `IndexType::Flat` - Búsqueda exacta O(n)
    /// - `IndexType::HNSW { m, ef_construction }` - Búsqueda aproximada O(log n)
    pub fn with_index(mut self, index: crate::IndexType) -> Self {
        self.index = index;
        self
    }
}

/// Internal representation of a stored document.
///
/// Supports both vectorized documents (with embeddings) and
/// metadata-only documents (without vectors).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredVector {
    pub id: VectorId,
    /// Vector embedding (None for metadata-only documents)
    pub vector: Option<Vec<f32>>,
    pub metadata: Option<Metadata>,
}

/// Resultado de búsqueda híbrida (vector + keyword + filtros).
///
/// Unifica resultados de diferentes tipos de búsqueda.
#[derive(Debug, Clone)]
pub struct HybridSearchResult {
    /// ID del documento encontrado
    pub id: VectorId,
    /// Score unificado (menor = mejor, consistente con distance)
    pub score: f32,
    /// Distancia vectorial (si participó en búsqueda vectorial)
    pub vector_distance: Option<f32>,
    /// Score BM25 (si participó en búsqueda keyword)
    pub bm25_score: Option<f32>,
    /// Rank en búsqueda vectorial (para RRF)
    pub vector_rank: Option<usize>,
    /// Rank en búsqueda keyword (para RRF)
    pub keyword_rank: Option<usize>,
    /// Metadata asociada al documento
    pub metadata: Option<Metadata>,
}
