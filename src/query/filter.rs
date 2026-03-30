//! Filtros de metadata con operadores lógicos y soporte para dot notation.

use super::FilterOp;
use crate::types::{Metadata, MetadataValue};

/// Filtro de metadata con operadores lógicos.
///
/// Permite construir consultas complejas combinando condiciones simples
/// con operadores AND/OR.
///
/// # Ejemplo
///
/// ```rust
/// use minimemory::Filter;
///
/// // Filtro simple
/// let filter = Filter::eq("author", "Juan");
///
/// // Encadenamiento con AND
/// let filter = Filter::eq("category", "tech")
///     .and(Filter::gte("score", 0.5f64));
///
/// // Encadenamiento con OR
/// let filter = Filter::eq("status", "published")
///     .or(Filter::eq("status", "featured"));
///
/// // Múltiples filtros con all/any
/// let filter = Filter::all(vec![
///     Filter::eq("category", "tech"),
///     Filter::gte("score", 0.5f64),
/// ]);
///
/// // Dot notation para campos anidados
/// let filter = Filter::eq("author.name", "Juan");
/// ```
#[derive(Debug, Clone)]
pub enum Filter {
    /// Condición simple: campo + operador
    Condition { field: String, op: FilterOp },
    /// AND lógico: todas las condiciones deben cumplirse
    And(Vec<Filter>),
    /// OR lógico: al menos una condición debe cumplirse
    Or(Vec<Filter>),
    /// NOT lógico: invierte el resultado
    Not(Box<Filter>),
}

impl Filter {
    // ========== Constructores para operadores de igualdad ==========

    /// Crea un filtro de igualdad simple.
    ///
    /// # Ejemplo
    /// ```rust
    /// use minimemory::query::Filter;
    /// let filter = Filter::eq("author", "Juan");
    /// ```
    pub fn eq(field: impl Into<String>, value: impl Into<MetadataValue>) -> Self {
        Filter::Condition {
            field: field.into(),
            op: FilterOp::Eq(value.into()),
        }
    }

    /// Crea un filtro de desigualdad.
    pub fn ne(field: impl Into<String>, value: impl Into<MetadataValue>) -> Self {
        Filter::Condition {
            field: field.into(),
            op: FilterOp::Ne(value.into()),
        }
    }

    // ========== Constructores para comparaciones numéricas ==========

    /// Crea un filtro "mayor que".
    pub fn gt(field: impl Into<String>, value: impl Into<MetadataValue>) -> Self {
        Filter::Condition {
            field: field.into(),
            op: FilterOp::Gt(value.into()),
        }
    }

    /// Crea un filtro "mayor o igual que".
    pub fn gte(field: impl Into<String>, value: impl Into<MetadataValue>) -> Self {
        Filter::Condition {
            field: field.into(),
            op: FilterOp::Gte(value.into()),
        }
    }

    /// Crea un filtro "menor que".
    pub fn lt(field: impl Into<String>, value: impl Into<MetadataValue>) -> Self {
        Filter::Condition {
            field: field.into(),
            op: FilterOp::Lt(value.into()),
        }
    }

    /// Crea un filtro "menor o igual que".
    pub fn lte(field: impl Into<String>, value: impl Into<MetadataValue>) -> Self {
        Filter::Condition {
            field: field.into(),
            op: FilterOp::Lte(value.into()),
        }
    }

    /// Crea un filtro de rango (min <= x <= max).
    pub fn range(
        field: impl Into<String>,
        min: Option<impl Into<MetadataValue>>,
        max: Option<impl Into<MetadataValue>>,
    ) -> Self {
        let field = field.into();
        let mut conditions = Vec::new();

        if let Some(min_val) = min {
            conditions.push(Filter::Condition {
                field: field.clone(),
                op: FilterOp::Gte(min_val.into()),
            });
        }

        if let Some(max_val) = max {
            conditions.push(Filter::Condition {
                field: field.clone(),
                op: FilterOp::Lte(max_val.into()),
            });
        }

        match conditions.len() {
            0 => Filter::And(vec![]), // Always true
            1 => conditions.remove(0),
            _ => Filter::And(conditions),
        }
    }

    // ========== Constructores para listas ==========

    /// Crea un filtro "valor en lista".
    pub fn in_list(field: impl Into<String>, values: Vec<impl Into<MetadataValue>>) -> Self {
        Filter::Condition {
            field: field.into(),
            op: FilterOp::In(values.into_iter().map(|v| v.into()).collect()),
        }
    }

    /// Crea un filtro "valor NO en lista".
    pub fn not_in_list(field: impl Into<String>, values: Vec<impl Into<MetadataValue>>) -> Self {
        Filter::Condition {
            field: field.into(),
            op: FilterOp::Nin(values.into_iter().map(|v| v.into()).collect()),
        }
    }

    // ========== Constructores para strings ==========

    /// Crea un filtro "campo contiene substring".
    pub fn contains(field: impl Into<String>, substr: impl Into<String>) -> Self {
        Filter::Condition {
            field: field.into(),
            op: FilterOp::Contains(substr.into()),
        }
    }

    /// Crea un filtro "campo empieza con".
    pub fn starts_with(field: impl Into<String>, prefix: impl Into<String>) -> Self {
        Filter::Condition {
            field: field.into(),
            op: FilterOp::StartsWith(prefix.into()),
        }
    }

    /// Crea un filtro "campo termina con".
    pub fn ends_with(field: impl Into<String>, suffix: impl Into<String>) -> Self {
        Filter::Condition {
            field: field.into(),
            op: FilterOp::EndsWith(suffix.into()),
        }
    }

    /// Crea un filtro de regex sobre un campo string.
    ///
    /// # Ejemplo
    ///
    /// ```rust
    /// use minimemory::Filter;
    ///
    /// // Matches strings starting with "Hello"
    /// Filter::regex("title", "^Hello");
    /// ```
    pub fn regex(field: impl Into<String>, pattern: impl Into<String>) -> Self {
        Filter::Condition {
            field: field.into(),
            op: FilterOp::Regex(pattern.into()),
        }
    }

    // ========== Constructores para existencia ==========

    /// Crea un filtro "campo existe".
    pub fn exists(field: impl Into<String>) -> Self {
        Filter::Condition {
            field: field.into(),
            op: FilterOp::Exists(true),
        }
    }

    /// Crea un filtro "campo no existe".
    pub fn not_exists(field: impl Into<String>) -> Self {
        Filter::Condition {
            field: field.into(),
            op: FilterOp::Exists(false),
        }
    }

    // ========== Combinadores lógicos ==========

    /// Niega un filtro.
    pub fn not(filter: Filter) -> Self {
        Filter::Not(Box::new(filter))
    }

    // ========== Métodos encadenables ==========

    /// Combina este filtro con otro usando AND.
    ///
    /// # Ejemplo
    /// ```rust
    /// use minimemory::Filter;
    /// let filter = Filter::eq("category", "tech")
    ///     .and(Filter::gt("score", 0.5f64));
    /// ```
    pub fn and(self, other: Filter) -> Self {
        match self {
            Filter::And(mut filters) => {
                filters.push(other);
                Filter::And(filters)
            }
            _ => Filter::And(vec![self, other]),
        }
    }

    /// Combina este filtro con otro usando OR.
    ///
    /// # Ejemplo
    /// ```rust
    /// use minimemory::Filter;
    /// let filter = Filter::eq("status", "published")
    ///     .or(Filter::eq("status", "featured"));
    /// ```
    pub fn or(self, other: Filter) -> Self {
        match self {
            Filter::Or(mut filters) => {
                filters.push(other);
                Filter::Or(filters)
            }
            _ => Filter::Or(vec![self, other]),
        }
    }

    /// Combina múltiples filtros con AND (versión estática).
    pub fn all(filters: Vec<Filter>) -> Self {
        Filter::And(filters)
    }

    /// Combina múltiples filtros con OR (versión estática).
    pub fn any(filters: Vec<Filter>) -> Self {
        Filter::Or(filters)
    }
}

/// Evaluador de filtros sobre metadata.
pub struct FilterEvaluator;

impl FilterEvaluator {
    /// Evalúa si un documento cumple el filtro.
    ///
    /// # Arguments
    /// * `filter` - El filtro a evaluar
    /// * `metadata` - La metadata del documento (puede ser None)
    ///
    /// # Returns
    /// `true` si el documento cumple el filtro.
    pub fn evaluate(filter: &Filter, metadata: Option<&Metadata>) -> bool {
        match filter {
            Filter::Condition { field, op } => {
                let value = Self::get_nested_value(metadata, field);
                op.evaluate(value)
            }
            Filter::And(filters) => filters.iter().all(|f| Self::evaluate(f, metadata)),
            Filter::Or(filters) => {
                if filters.is_empty() {
                    return true; // Empty OR is vacuously true
                }
                filters.iter().any(|f| Self::evaluate(f, metadata))
            }
            Filter::Not(filter) => !Self::evaluate(filter, metadata),
        }
    }

    /// Obtiene un valor anidado usando dot notation.
    ///
    /// Soporta:
    /// - Campos simples: "author"
    /// - Campos anidados: "author.name"
    /// - Índices de array: "tags.0"
    ///
    /// # Arguments
    /// * `metadata` - La metadata del documento
    /// * `path` - El path al valor (e.g., "author.name")
    fn get_nested_value<'a>(
        metadata: Option<&'a Metadata>,
        path: &str,
    ) -> Option<&'a MetadataValue> {
        let metadata = metadata?;
        let parts: Vec<&str> = path.split('.').collect();

        if parts.is_empty() {
            return None;
        }

        // Obtener el valor inicial
        let mut current: Option<&MetadataValue> = metadata.get(parts[0]);

        // Navegar por los campos anidados
        for part in &parts[1..] {
            current = match current? {
                MetadataValue::Map(map) => map.get(*part),
                MetadataValue::List(list) => {
                    // Intentar parsear como índice
                    part.parse::<usize>().ok().and_then(|idx| list.get(idx))
                }
                _ => None,
            };
        }

        current
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_metadata() -> Metadata {
        let mut meta = Metadata::new();
        meta.insert("title", "Test Document");
        meta.insert("score", 0.8f64);
        meta.insert("count", 42i64);
        meta.insert("published", true);
        meta.insert("category", "tech");
        meta
    }

    fn create_nested_metadata() -> Metadata {
        let mut author = HashMap::new();
        author.insert(
            "name".to_string(),
            MetadataValue::String("Juan".to_string()),
        );
        author.insert("age".to_string(), MetadataValue::Int(30));

        let mut meta = Metadata::new();
        meta.insert("title", "Nested Document");
        meta.fields
            .insert("author".to_string(), MetadataValue::Map(author));
        meta.fields.insert(
            "tags".to_string(),
            MetadataValue::List(vec![
                MetadataValue::String("rust".to_string()),
                MetadataValue::String("programming".to_string()),
            ]),
        );
        meta
    }

    #[test]
    fn test_simple_eq() {
        let meta = create_test_metadata();
        let filter = Filter::eq("category", "tech");
        assert!(FilterEvaluator::evaluate(&filter, Some(&meta)));

        let filter = Filter::eq("category", "sports");
        assert!(!FilterEvaluator::evaluate(&filter, Some(&meta)));
    }

    #[test]
    fn test_numeric_comparison() {
        let meta = create_test_metadata();

        let filter = Filter::gte("score", 0.5f64);
        assert!(FilterEvaluator::evaluate(&filter, Some(&meta)));

        let filter = Filter::lt("score", 0.5f64);
        assert!(!FilterEvaluator::evaluate(&filter, Some(&meta)));

        let filter = Filter::gt("count", 40i64);
        assert!(FilterEvaluator::evaluate(&filter, Some(&meta)));
    }

    #[test]
    fn test_and_filter() {
        let meta = create_test_metadata();

        let filter = Filter::all(vec![
            Filter::eq("category", "tech"),
            Filter::gte("score", 0.5f64),
        ]);
        assert!(FilterEvaluator::evaluate(&filter, Some(&meta)));

        let filter = Filter::all(vec![
            Filter::eq("category", "sports"), // Fails
            Filter::gte("score", 0.5f64),
        ]);
        assert!(!FilterEvaluator::evaluate(&filter, Some(&meta)));
    }

    #[test]
    fn test_or_filter() {
        let meta = create_test_metadata();

        let filter = Filter::any(vec![
            Filter::eq("category", "sports"), // Fails
            Filter::eq("category", "tech"),   // Passes
        ]);
        assert!(FilterEvaluator::evaluate(&filter, Some(&meta)));

        let filter = Filter::any(vec![
            Filter::eq("category", "sports"),
            Filter::eq("category", "news"),
        ]);
        assert!(!FilterEvaluator::evaluate(&filter, Some(&meta)));
    }

    #[test]
    fn test_not_filter() {
        let meta = create_test_metadata();

        let filter = Filter::not(Filter::eq("category", "sports"));
        assert!(FilterEvaluator::evaluate(&filter, Some(&meta)));

        let filter = Filter::not(Filter::eq("category", "tech"));
        assert!(!FilterEvaluator::evaluate(&filter, Some(&meta)));
    }

    #[test]
    fn test_nested_dot_notation() {
        let meta = create_nested_metadata();

        // Access nested object
        let filter = Filter::eq("author.name", "Juan");
        assert!(FilterEvaluator::evaluate(&filter, Some(&meta)));

        let filter = Filter::gt("author.age", 25i64);
        assert!(FilterEvaluator::evaluate(&filter, Some(&meta)));
    }

    #[test]
    fn test_array_index_access() {
        let meta = create_nested_metadata();

        // Access array element by index
        let filter = Filter::eq("tags.0", "rust");
        assert!(FilterEvaluator::evaluate(&filter, Some(&meta)));

        let filter = Filter::eq("tags.1", "programming");
        assert!(FilterEvaluator::evaluate(&filter, Some(&meta)));
    }

    #[test]
    fn test_in_list() {
        let meta = create_test_metadata();

        let filter = Filter::in_list("category", vec!["tech", "science", "news"]);
        assert!(FilterEvaluator::evaluate(&filter, Some(&meta)));

        let filter = Filter::in_list("category", vec!["sports", "entertainment"]);
        assert!(!FilterEvaluator::evaluate(&filter, Some(&meta)));
    }

    #[test]
    fn test_contains() {
        let meta = create_test_metadata();

        let filter = Filter::contains("title", "Document");
        assert!(FilterEvaluator::evaluate(&filter, Some(&meta)));

        let filter = Filter::contains("title", "missing");
        assert!(!FilterEvaluator::evaluate(&filter, Some(&meta)));
    }

    #[test]
    fn test_exists() {
        let meta = create_test_metadata();

        let filter = Filter::exists("title");
        assert!(FilterEvaluator::evaluate(&filter, Some(&meta)));

        let filter = Filter::exists("nonexistent");
        assert!(!FilterEvaluator::evaluate(&filter, Some(&meta)));

        let filter = Filter::not_exists("nonexistent");
        assert!(FilterEvaluator::evaluate(&filter, Some(&meta)));
    }

    #[test]
    fn test_range() {
        let meta = create_test_metadata();

        let filter = Filter::range("score", Some(0.5f64), Some(1.0f64));
        assert!(FilterEvaluator::evaluate(&filter, Some(&meta)));

        let filter = Filter::range("score", Some(0.9f64), Some(1.0f64));
        assert!(!FilterEvaluator::evaluate(&filter, Some(&meta)));
    }

    #[test]
    fn test_none_metadata() {
        let filter = Filter::eq("field", "value");
        assert!(!FilterEvaluator::evaluate(&filter, None));

        let filter = Filter::not_exists("field");
        assert!(FilterEvaluator::evaluate(&filter, None));
    }
}
