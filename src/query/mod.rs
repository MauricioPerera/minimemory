//! Sistema de filtrado para consultas de metadata.
//!
//! Proporciona operadores de comparación y lógica para filtrar documentos
//! basándose en sus campos de metadata.
//!
//! # Ejemplo
//!
//! ```rust
//! use minimemory::query::{Filter, FilterOp};
//!
//! // Filtro simple
//! let filter = Filter::eq("author", "Juan");
//!
//! // Filtro con operadores lógicos
//! let filter = Filter::and(vec![
//!     Filter::eq("category", "tech"),
//!     Filter::gte("score", 0.5f64),
//! ]);
//!
//! // Filtro con dot notation para nested fields
//! let filter = Filter::eq("author.name", "Juan");
//! ```

mod operators;
mod filter;

pub use operators::FilterOp;
pub use filter::{Filter, FilterEvaluator};
