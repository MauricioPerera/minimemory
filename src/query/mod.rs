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

use crate::types::MetadataValue;
use std::cmp::Ordering;

/// Direction for sorting results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    /// Ascending (A→Z, 0→9, oldest→newest)
    Asc,
    /// Descending (Z→A, 9→0, newest→oldest)
    Desc,
}

/// Specifies how to order query results by a metadata field.
///
/// # Example
///
/// ```rust
/// use minimemory::query::OrderBy;
///
/// // Order by created_at descending (newest first)
/// let order = OrderBy::desc("created_at");
///
/// // Order by title ascending (A-Z)
/// let order = OrderBy::asc("title");
/// ```
#[derive(Debug, Clone)]
pub struct OrderBy {
    /// Metadata field name to sort by
    pub field: String,
    /// Sort direction
    pub direction: SortDirection,
}

impl OrderBy {
    /// Sort ascending by the given field.
    pub fn asc(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            direction: SortDirection::Asc,
        }
    }

    /// Sort descending by the given field.
    pub fn desc(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            direction: SortDirection::Desc,
        }
    }
}

/// Compare two optional MetadataValues for ordering.
/// Returns Ordering::Equal if either is None or types are incompatible.
pub fn compare_metadata_values(
    a: Option<&MetadataValue>,
    b: Option<&MetadataValue>,
) -> Ordering {
    match (a, b) {
        (Some(MetadataValue::String(sa)), Some(MetadataValue::String(sb))) => sa.cmp(sb),
        (Some(MetadataValue::Int(ia)), Some(MetadataValue::Int(ib))) => ia.cmp(ib),
        (Some(MetadataValue::Float(fa)), Some(MetadataValue::Float(fb))) => {
            fa.partial_cmp(fb).unwrap_or(Ordering::Equal)
        }
        (Some(MetadataValue::Bool(ba)), Some(MetadataValue::Bool(bb))) => ba.cmp(bb),
        // Cross-type numeric comparison
        (Some(MetadataValue::Int(i)), Some(MetadataValue::Float(f))) => {
            (*i as f64).partial_cmp(f).unwrap_or(Ordering::Equal)
        }
        (Some(MetadataValue::Float(f)), Some(MetadataValue::Int(i))) => {
            f.partial_cmp(&(*i as f64)).unwrap_or(Ordering::Equal)
        }
        // None or incompatible → equal (stable sort preserves original order)
        (None, Some(_)) => Ordering::Greater, // None sorts last
        (Some(_), None) => Ordering::Less,
        _ => Ordering::Equal,
    }
}
