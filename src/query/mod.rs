//! Sistema de filtrado para consultas de metadata.
//!
//! Proporciona operadores de comparación y lógica para filtrar documentos
//! basándose en sus campos de metadata.
//!
//! # Ejemplo
//!
//! ```rust
//! use minimemory::Filter;
//!
//! // Filtro simple
//! let filter = Filter::eq("author", "Juan");
//!
//! // Encadenamiento con AND/OR
//! let filter = Filter::eq("category", "tech")
//!     .and(Filter::gte("score", 0.5f64));
//!
//! // Múltiples filtros con all/any
//! let filter = Filter::all(vec![
//!     Filter::eq("category", "tech"),
//!     Filter::gte("score", 0.5f64),
//! ]);
//!
//! // Filtro con dot notation para nested fields
//! let filter = Filter::eq("author.name", "Juan");
//! ```

mod filter;
mod operators;

pub use filter::{Filter, FilterEvaluator};
pub use operators::FilterOp;
