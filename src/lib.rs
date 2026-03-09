//! # minimemory
//!
//! Base de datos híbrida embebida para Rust.
//! Como SQLite para documentos + búsqueda vectorial + full-text search.
//!
//! ## Características
//!
//! - **Sin servidor**: Librería embebida, solo importar y usar
//! - **Ligera**: Sin dependencias pesadas
//! - **Rápida**: Optimizada para alto rendimiento
//! - **Híbrida**: Combina vectores, BM25 y filtros de metadata
//! - **Flexible**: Múltiples métricas de distancia (Cosine, Euclidean, DotProduct)
//!
//! ## Inicio Rápido
//!
//! ```rust
//! use minimemory::{VectorDB, Config, Distance, IndexType};
//!
//! // Crear base de datos en memoria
//! let config = Config::new(4)  // 4 dimensiones
//!     .with_distance(Distance::Cosine)
//!     .with_index(IndexType::Flat);
//!
//! let db = VectorDB::new(config).unwrap();
//!
//! // Insertar vectores
//! db.insert("doc-1", &[0.1, 0.2, 0.3, 0.4], None).unwrap();
//! db.insert("doc-2", &[0.2, 0.3, 0.4, 0.5], None).unwrap();
//!
//! // Buscar los 2 más similares
//! let query = [0.15, 0.25, 0.35, 0.45];
//! let results = db.search(&query, 2).unwrap();
//!
//! assert_eq!(results.len(), 2);
//! println!("Más cercano: {} (dist: {})", results[0].id, results[0].distance);
//! ```
//!
//! ## Documentos con Metadata (Sin Vector)
//!
//! ```rust
//! use minimemory::{VectorDB, Config, Metadata};
//!
//! // Crear DB con full-text search habilitado
//! let db = VectorDB::with_fulltext(
//!     Config::new(3),
//!     vec!["title".into(), "content".into()]
//! ).unwrap();
//!
//! // Insertar documento SIN vector (como MongoDB)
//! let mut meta = Metadata::new();
//! meta.insert("title", "Mi Post de Blog");
//! meta.insert("content", "Contenido del post...");
//! meta.insert("author", "Juan");
//!
//! db.insert_document("post-1", None, Some(meta)).unwrap();
//!
//! // Insertar documento CON vector (para búsqueda semántica)
//! let mut meta2 = Metadata::new();
//! meta2.insert("title", "Otro Post");
//! db.insert_document("post-2", Some(&[0.1, 0.2, 0.3]), Some(meta2)).unwrap();
//! ```
//!
//! ## Búsqueda Híbrida
//!
//! ```rust
//! use minimemory::{VectorDB, Config, Filter, HybridSearchParams};
//!
//! let db = VectorDB::with_fulltext(
//!     Config::new(3),
//!     vec!["title".into(), "content".into()]
//! ).unwrap();
//!
//! // Búsqueda por keywords (BM25)
//! let results = db.keyword_search("rust programming", 10).unwrap();
//!
//! // Búsqueda por filtros de metadata
//! let results = db.filter_search(
//!     Filter::eq("author", "Juan"),
//!     10
//! ).unwrap();
//!
//! // Búsqueda vectorial con filtro
//! let results = db.search_with_filter(
//!     &[0.1, 0.2, 0.3],
//!     10,
//!     Filter::eq("category", "tech")
//! ).unwrap();
//!
//! // Búsqueda híbrida: vector + keyword + filtro
//! let params = HybridSearchParams::hybrid(
//!     vec![0.1, 0.2, 0.3],
//!     "rust",
//!     10
//! ).with_filter(Filter::eq("category", "tech"));
//! let results = db.hybrid_search(params).unwrap();
//! ```
//!
//! ## Filtros de Metadata
//!
//! ```rust
//! use minimemory::Filter;
//!
//! // Operadores básicos
//! Filter::eq("status", "active");
//! Filter::ne("status", "deleted");
//! Filter::gt("score", 0.5f64);
//! Filter::gte("count", 10i64);
//! Filter::lt("price", 100.0f64);
//!
//! // Operadores de colección
//! Filter::contains("tags", "rust");
//! Filter::starts_with("title", "How to");
//!
//! // Operadores lógicos
//! Filter::eq("category", "tech")
//!     .and(Filter::gt("score", 0.5f64))
//!     .or(Filter::eq("featured", true));
//!
//! // Acceso a campos anidados (dot notation)
//! Filter::eq("author.name", "Juan");
//! Filter::gt("metadata.views", 1000i64);
//! ```
//!
//! ## Métricas de Distancia
//!
//! - [`Distance::Cosine`] - Similitud coseno (ideal para embeddings de texto)
//! - [`Distance::Euclidean`] - Distancia L2 (para vectores normalizados)
//! - [`Distance::DotProduct`] - Producto punto (cuando la magnitud importa)
//!
//! ## Tipos Principales
//!
//! - [`VectorDB`] - Interfaz principal de la base de datos
//! - [`Config`] - Configuración de la base de datos
//! - [`Distance`] - Métricas de distancia disponibles
//! - [`IndexType`] - Tipos de índice (Flat, HNSW)
//! - [`Metadata`] - Metadata asociada a documentos
//! - [`SearchResult`] - Resultado de búsqueda vectorial
//! - [`HybridSearchResult`] - Resultado de búsqueda híbrida
//! - [`Filter`] - Filtros de metadata
//! - [`HybridSearchParams`] - Parámetros de búsqueda híbrida

pub mod agent_memory;
pub mod chunking;
mod db;
mod distance;
mod error;
pub mod index;
pub mod memory_traits;
pub mod partial_index;
pub mod quantization;
pub mod query;
pub mod replication;
pub mod search;
mod storage;
pub mod transfer;
mod types;

// Local embeddings (optional)
#[cfg(feature = "embeddings")]
pub mod embeddings;

// Bindings para otros lenguajes
#[cfg(any(feature = "python", feature = "nodejs", feature = "ffi"))]
pub mod bindings;

pub use db::VectorDB;
pub use distance::Distance;
pub use error::{Error, Result};
pub use index::IndexType;
pub use quantization::{QuantizationType, QuantizedVector, Quantizer};
pub use query::{Filter, FilterOp};
pub use search::{HybridSearchParams, SearchMode};
pub use types::{
    Config, HybridSearchResult, Metadata, MetadataValue, SearchResult, StoredVector, Vector,
    VectorId,
};
