//! Operadores de comparación para filtros de metadata.

use crate::types::MetadataValue;
use serde::{Deserialize, Serialize};

/// Operadores de comparación para filtros.
///
/// Cada operador define una condición que se evalúa contra
/// un valor de metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FilterOp {
    /// Igual a (==)
    Eq(MetadataValue),
    /// No igual a (!=)
    Ne(MetadataValue),
    /// Mayor que (>)
    Gt(MetadataValue),
    /// Mayor o igual que (>=)
    Gte(MetadataValue),
    /// Menor que (<)
    Lt(MetadataValue),
    /// Menor o igual que (<=)
    Lte(MetadataValue),
    /// Valor está en lista (IN)
    In(Vec<MetadataValue>),
    /// Valor NO está en lista (NOT IN)
    Nin(Vec<MetadataValue>),
    /// Campo existe o no existe
    Exists(bool),
    /// String contiene substring
    Contains(String),
    /// String empieza con prefijo
    StartsWith(String),
    /// String termina con sufijo
    EndsWith(String),
    /// String matches regex pattern
    Regex(String),
}

impl FilterOp {
    /// Evalúa el operador contra un valor de metadata.
    ///
    /// # Arguments
    /// * `value` - El valor de metadata a evaluar (None si el campo no existe)
    ///
    /// # Returns
    /// `true` si la condición se cumple, `false` en caso contrario.
    pub fn evaluate(&self, value: Option<&MetadataValue>) -> bool {
        match self {
            FilterOp::Exists(should_exist) => value.is_some() == *should_exist,

            FilterOp::Eq(expected) => value.is_some_and(|v| values_equal(v, expected)),

            FilterOp::Ne(expected) => value.is_none_or(|v| !values_equal(v, expected)),

            FilterOp::Gt(threshold) => value
                .is_some_and(|v| compare_values(v, threshold) == Some(std::cmp::Ordering::Greater)),

            FilterOp::Gte(threshold) => value.is_some_and(|v| {
                matches!(
                    compare_values(v, threshold),
                    Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
                )
            }),

            FilterOp::Lt(threshold) => value
                .is_some_and(|v| compare_values(v, threshold) == Some(std::cmp::Ordering::Less)),

            FilterOp::Lte(threshold) => value.is_some_and(|v| {
                matches!(
                    compare_values(v, threshold),
                    Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
                )
            }),

            FilterOp::In(list) => {
                value.is_some_and(|v| list.iter().any(|item| values_equal(v, item)))
            }

            FilterOp::Nin(list) => {
                value.is_none_or(|v| !list.iter().any(|item| values_equal(v, item)))
            }

            FilterOp::Contains(substr) => match value {
                Some(MetadataValue::String(s)) => s.to_lowercase().contains(&substr.to_lowercase()),
                _ => false,
            },

            FilterOp::StartsWith(prefix) => match value {
                Some(MetadataValue::String(s)) => {
                    s.to_lowercase().starts_with(&prefix.to_lowercase())
                }
                _ => false,
            },

            FilterOp::EndsWith(suffix) => match value {
                Some(MetadataValue::String(s)) => {
                    s.to_lowercase().ends_with(&suffix.to_lowercase())
                }
                _ => false,
            },

            FilterOp::Regex(pattern) => match value {
                Some(MetadataValue::String(s)) => regex_lite::Regex::new(pattern)
                    .map(|re| re.is_match(s))
                    .unwrap_or(false),
                _ => false,
            },
        }
    }
}

/// Compara dos valores para igualdad.
fn values_equal(a: &MetadataValue, b: &MetadataValue) -> bool {
    match (a, b) {
        (MetadataValue::String(s1), MetadataValue::String(s2)) => s1 == s2,
        (MetadataValue::Int(i1), MetadataValue::Int(i2)) => i1 == i2,
        (MetadataValue::Float(f1), MetadataValue::Float(f2)) => (f1 - f2).abs() < f64::EPSILON,
        (MetadataValue::Bool(b1), MetadataValue::Bool(b2)) => b1 == b2,
        // Cross-type numeric comparison
        (MetadataValue::Int(i), MetadataValue::Float(f)) => (*i as f64 - f).abs() < f64::EPSILON,
        (MetadataValue::Float(f), MetadataValue::Int(i)) => (f - *i as f64).abs() < f64::EPSILON,
        _ => false,
    }
}

/// Compara dos valores numéricamente.
fn compare_values(a: &MetadataValue, b: &MetadataValue) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (MetadataValue::Int(i1), MetadataValue::Int(i2)) => Some(i1.cmp(i2)),
        (MetadataValue::Float(f1), MetadataValue::Float(f2)) => f1.partial_cmp(f2),
        (MetadataValue::Int(i), MetadataValue::Float(f)) => (*i as f64).partial_cmp(f),
        (MetadataValue::Float(f), MetadataValue::Int(i)) => f.partial_cmp(&(*i as f64)),
        (MetadataValue::String(s1), MetadataValue::String(s2)) => Some(s1.cmp(s2)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eq_string() {
        let op = FilterOp::Eq(MetadataValue::String("test".into()));
        assert!(op.evaluate(Some(&MetadataValue::String("test".into()))));
        assert!(!op.evaluate(Some(&MetadataValue::String("other".into()))));
        assert!(!op.evaluate(None));
    }

    #[test]
    fn test_eq_int() {
        let op = FilterOp::Eq(MetadataValue::Int(42));
        assert!(op.evaluate(Some(&MetadataValue::Int(42))));
        assert!(!op.evaluate(Some(&MetadataValue::Int(41))));
    }

    #[test]
    fn test_numeric_comparison() {
        let op = FilterOp::Gt(MetadataValue::Int(10));
        assert!(op.evaluate(Some(&MetadataValue::Int(15))));
        assert!(!op.evaluate(Some(&MetadataValue::Int(10))));
        assert!(!op.evaluate(Some(&MetadataValue::Int(5))));

        let op = FilterOp::Gte(MetadataValue::Float(0.5));
        assert!(op.evaluate(Some(&MetadataValue::Float(0.5))));
        assert!(op.evaluate(Some(&MetadataValue::Float(0.6))));
        assert!(!op.evaluate(Some(&MetadataValue::Float(0.4))));
    }

    #[test]
    fn test_cross_type_comparison() {
        // Int vs Float
        let op = FilterOp::Gt(MetadataValue::Int(10));
        assert!(op.evaluate(Some(&MetadataValue::Float(10.5))));
        assert!(!op.evaluate(Some(&MetadataValue::Float(9.5))));
    }

    #[test]
    fn test_in_operator() {
        let op = FilterOp::In(vec![
            MetadataValue::String("a".into()),
            MetadataValue::String("b".into()),
            MetadataValue::String("c".into()),
        ]);
        assert!(op.evaluate(Some(&MetadataValue::String("b".into()))));
        assert!(!op.evaluate(Some(&MetadataValue::String("d".into()))));
    }

    #[test]
    fn test_exists() {
        let op = FilterOp::Exists(true);
        assert!(op.evaluate(Some(&MetadataValue::String("any".into()))));
        assert!(!op.evaluate(None));

        let op = FilterOp::Exists(false);
        assert!(!op.evaluate(Some(&MetadataValue::String("any".into()))));
        assert!(op.evaluate(None));
    }

    #[test]
    fn test_contains() {
        let op = FilterOp::Contains("rust".into());
        assert!(op.evaluate(Some(&MetadataValue::String(
            "Learning Rust programming".into()
        ))));
        assert!(op.evaluate(Some(&MetadataValue::String("RUST is great".into())))); // case insensitive
        assert!(!op.evaluate(Some(&MetadataValue::String("Python is cool".into()))));
    }

    #[test]
    fn test_starts_with() {
        let op = FilterOp::StartsWith("hello".into());
        assert!(op.evaluate(Some(&MetadataValue::String("Hello World".into()))));
        assert!(!op.evaluate(Some(&MetadataValue::String("World Hello".into()))));
    }

    #[test]
    fn test_ends_with() {
        let op = FilterOp::EndsWith(".rs".into());
        assert!(op.evaluate(Some(&MetadataValue::String("main.rs".into()))));
        assert!(!op.evaluate(Some(&MetadataValue::String("main.py".into()))));
    }
}
