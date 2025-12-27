//! Índice BM25 para búsqueda full-text.
//!
//! Implementa el algoritmo BM25 (Best Matching 25) para búsqueda por keywords
//! en campos de texto de los documentos.
//!
//! # Ejemplo
//!
//! ```rust
//! use minimemory::index::BM25Index;
//! use minimemory::Metadata;
//!
//! let index = BM25Index::new(vec!["title".into(), "content".into()]);
//!
//! let mut meta = Metadata::new();
//! meta.insert("title", "Rust Programming");
//! meta.insert("content", "Learn Rust programming language");
//!
//! index.add("doc-1", Some(&meta)).unwrap();
//!
//! let results = index.search("rust programming", 10);
//! ```

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::error::Result;
use crate::types::{Metadata, MetadataValue, VectorId};

/// Parámetros BM25
const K1: f32 = 1.2;  // Saturación de frecuencia de término
const B: f32 = 0.75;   // Factor de normalización por longitud

/// Documento tokenizado para indexación
#[derive(Clone, Serialize, Deserialize)]
struct TokenizedDoc {
    id: VectorId,
    /// Frecuencia de cada término en el documento
    term_frequencies: HashMap<String, u32>,
    /// Longitud total del documento (número de tokens)
    length: u32,
}

/// Datos internos del índice BM25
#[derive(Default, Serialize, Deserialize)]
struct BM25Inner {
    /// Documentos indexados
    documents: HashMap<VectorId, TokenizedDoc>,
    /// Índice invertido: término -> set de doc IDs
    inverted_index: HashMap<String, HashSet<VectorId>>,
    /// Frecuencia de documento: término -> número de docs que lo contienen
    doc_frequencies: HashMap<String, u32>,
    /// Suma total de longitudes de documentos
    total_doc_length: u64,
}

/// Resultado de búsqueda BM25
#[derive(Debug, Clone)]
pub struct BM25SearchResult {
    /// ID del documento
    pub id: VectorId,
    /// Score BM25 (mayor = más relevante)
    pub score: f32,
}

/// Índice BM25 para búsqueda full-text.
///
/// Indexa campos de texto de metadata para búsqueda por keywords.
pub struct BM25Index {
    inner: RwLock<BM25Inner>,
    /// Campos de metadata a indexar para full-text
    indexed_fields: Vec<String>,
}

impl BM25Index {
    /// Crea un nuevo índice BM25.
    ///
    /// # Arguments
    /// * `indexed_fields` - Nombres de campos de metadata a indexar (e.g., ["title", "content"])
    ///
    /// # Ejemplo
    /// ```rust
    /// use minimemory::index::BM25Index;
    /// let index = BM25Index::new(vec!["title".into(), "content".into()]);
    /// ```
    pub fn new(indexed_fields: Vec<String>) -> Self {
        Self {
            inner: RwLock::new(BM25Inner::default()),
            indexed_fields,
        }
    }

    /// Retorna los campos indexados.
    pub fn indexed_fields(&self) -> &[String] {
        &self.indexed_fields
    }

    /// Indexa un documento.
    ///
    /// # Arguments
    /// * `id` - ID del documento
    /// * `metadata` - Metadata del documento (de donde se extraen los campos indexados)
    pub fn add(&self, id: &str, metadata: Option<&Metadata>) -> Result<()> {
        let text = self.extract_text(metadata);
        let tokens = self.tokenize(&text);

        if tokens.is_empty() {
            // No hay texto indexable, pero registramos el documento
            let mut inner = self.inner.write();
            inner.documents.insert(id.to_string(), TokenizedDoc {
                id: id.to_string(),
                term_frequencies: HashMap::new(),
                length: 0,
            });
            return Ok(());
        }

        let mut term_frequencies: HashMap<String, u32> = HashMap::new();
        for token in &tokens {
            *term_frequencies.entry(token.clone()).or_insert(0) += 1;
        }

        let doc = TokenizedDoc {
            id: id.to_string(),
            term_frequencies: term_frequencies.clone(),
            length: tokens.len() as u32,
        };

        let mut inner = self.inner.write();

        // Verificar si el documento ya existía
        let existed = inner.documents.contains_key(id);

        // Actualizar índice invertido y frecuencias
        for term in term_frequencies.keys() {
            inner.inverted_index
                .entry(term.clone())
                .or_insert_with(HashSet::new)
                .insert(id.to_string());

            // Solo incrementar doc_freq si es nuevo documento
            if !existed {
                *inner.doc_frequencies.entry(term.clone()).or_insert(0) += 1;
            }
        }

        inner.total_doc_length += doc.length as u64;
        inner.documents.insert(id.to_string(), doc);

        Ok(())
    }

    /// Elimina un documento del índice.
    pub fn remove(&self, id: &str) -> Result<bool> {
        let mut inner = self.inner.write();

        if let Some(doc) = inner.documents.remove(id) {
            inner.total_doc_length = inner.total_doc_length.saturating_sub(doc.length as u64);

            // Actualizar índice invertido
            for term in doc.term_frequencies.keys() {
                if let Some(docs) = inner.inverted_index.get_mut(term) {
                    docs.remove(id);
                    if docs.is_empty() {
                        inner.inverted_index.remove(term);
                    }
                }

                if let Some(count) = inner.doc_frequencies.get_mut(term) {
                    *count = count.saturating_sub(1);
                    if *count == 0 {
                        inner.doc_frequencies.remove(term);
                    }
                }
            }

            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Busca documentos por query.
    ///
    /// # Arguments
    /// * `query` - Query de búsqueda (se tokeniza automáticamente)
    /// * `k` - Número máximo de resultados
    ///
    /// # Returns
    /// Lista de resultados ordenados por score BM25 descendente.
    pub fn search(&self, query: &str, k: usize) -> Vec<BM25SearchResult> {
        let query_tokens = self.tokenize(query);
        let inner = self.inner.read();

        let n = inner.documents.len() as f32;
        if n == 0.0 || query_tokens.is_empty() {
            return vec![];
        }

        let avgdl = inner.total_doc_length as f32 / n;

        // Calcular scores BM25
        let mut scores: HashMap<VectorId, f32> = HashMap::new();

        for token in &query_tokens {
            // IDF component
            let df = *inner.doc_frequencies.get(token).unwrap_or(&0) as f32;
            if df == 0.0 {
                continue;
            }

            let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();

            // Score para cada documento que contiene el término
            if let Some(doc_ids) = inner.inverted_index.get(token) {
                for doc_id in doc_ids {
                    if let Some(doc) = inner.documents.get(doc_id) {
                        let tf = *doc.term_frequencies.get(token).unwrap_or(&0) as f32;
                        let dl = doc.length as f32;

                        // BM25 formula
                        let score = idf * (tf * (K1 + 1.0)) /
                            (tf + K1 * (1.0 - B + B * dl / avgdl));

                        *scores.entry(doc_id.clone()).or_insert(0.0) += score;
                    }
                }
            }
        }

        // Ordenar por score descendente
        let mut results: Vec<_> = scores.into_iter()
            .map(|(id, score)| BM25SearchResult { id, score })
            .collect();
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(k);

        results
    }

    /// Extrae texto de los campos indexados.
    fn extract_text(&self, metadata: Option<&Metadata>) -> String {
        let Some(meta) = metadata else {
            return String::new();
        };

        self.indexed_fields
            .iter()
            .filter_map(|field| {
                match meta.get(field) {
                    Some(MetadataValue::String(s)) => Some(s.clone()),
                    _ => None,
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Tokeniza texto (lowercase + split por espacios/puntuación).
    fn tokenize(&self, text: &str) -> Vec<String> {
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty() && s.len() > 1)
            .map(|s| s.to_string())
            .collect()
    }

    /// Retorna el número de documentos indexados.
    pub fn len(&self) -> usize {
        self.inner.read().documents.len()
    }

    /// Verifica si el índice está vacío.
    pub fn is_empty(&self) -> bool {
        self.inner.read().documents.is_empty()
    }

    /// Limpia el índice.
    pub fn clear(&self) {
        let mut inner = self.inner.write();
        inner.documents.clear();
        inner.inverted_index.clear();
        inner.doc_frequencies.clear();
        inner.total_doc_length = 0;
    }

    /// Serializa el índice para persistencia.
    pub fn serialize(&self) -> Result<Vec<u8>> {
        let inner = self.inner.read();
        let data = bincode::serialize(&*inner)?;
        Ok(data)
    }

    /// Deserializa el índice desde bytes.
    pub fn deserialize(indexed_fields: Vec<String>, data: &[u8]) -> Result<Self> {
        let inner: BM25Inner = bincode::deserialize(data)?;
        Ok(Self {
            inner: RwLock::new(inner),
            indexed_fields,
        })
    }

    /// Retorna estadísticas del índice.
    pub fn stats(&self) -> BM25Stats {
        let inner = self.inner.read();
        BM25Stats {
            document_count: inner.documents.len(),
            unique_terms: inner.doc_frequencies.len(),
            total_tokens: inner.total_doc_length as usize,
            avg_doc_length: if inner.documents.is_empty() {
                0.0
            } else {
                inner.total_doc_length as f32 / inner.documents.len() as f32
            },
        }
    }
}

/// Estadísticas del índice BM25.
#[derive(Debug, Clone)]
pub struct BM25Stats {
    /// Número de documentos indexados
    pub document_count: usize,
    /// Número de términos únicos
    pub unique_terms: usize,
    /// Total de tokens en todos los documentos
    pub total_tokens: usize,
    /// Longitud promedio de documento
    pub avg_doc_length: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_index() -> BM25Index {
        BM25Index::new(vec!["title".into(), "content".into()])
    }

    fn create_doc(title: &str, content: &str) -> Metadata {
        let mut meta = Metadata::new();
        meta.insert("title", title);
        meta.insert("content", content);
        meta
    }

    #[test]
    fn test_add_and_search() {
        let index = create_index();

        let meta1 = create_doc("Rust Programming", "Learn Rust programming language");
        let meta2 = create_doc("Python Guide", "Python is great for beginners");
        let meta3 = create_doc("JavaScript Tutorial", "JavaScript for web development");

        index.add("doc-1", Some(&meta1)).unwrap();
        index.add("doc-2", Some(&meta2)).unwrap();
        index.add("doc-3", Some(&meta3)).unwrap();

        assert_eq!(index.len(), 3);

        // Search for Rust
        let results = index.search("rust", 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "doc-1");

        // Search for programming
        let results = index.search("programming", 10);
        assert_eq!(results.len(), 2); // doc-1 and doc-2 (has "is")
    }

    #[test]
    fn test_empty_query() {
        let index = create_index();
        let meta = create_doc("Test", "Content");
        index.add("doc-1", Some(&meta)).unwrap();

        let results = index.search("", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_no_match() {
        let index = create_index();
        let meta = create_doc("Rust", "Programming");
        index.add("doc-1", Some(&meta)).unwrap();

        let results = index.search("python", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_remove() {
        let index = create_index();
        let meta = create_doc("Rust", "Programming");
        index.add("doc-1", Some(&meta)).unwrap();

        assert_eq!(index.len(), 1);

        index.remove("doc-1").unwrap();
        assert_eq!(index.len(), 0);

        let results = index.search("rust", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_bm25_ranking() {
        let index = create_index();

        // doc-1: mentions "rust" twice
        let meta1 = create_doc("Rust", "Rust is a systems programming language. Rust is fast.");
        // doc-2: mentions "rust" once
        let meta2 = create_doc("Python", "Learn Rust programming");

        index.add("doc-1", Some(&meta1)).unwrap();
        index.add("doc-2", Some(&meta2)).unwrap();

        let results = index.search("rust", 10);
        assert_eq!(results.len(), 2);
        // doc-1 should rank higher (more occurrences)
        assert_eq!(results[0].id, "doc-1");
        assert!(results[0].score > results[1].score);
    }

    #[test]
    fn test_clear() {
        let index = create_index();
        let meta = create_doc("Test", "Content");
        index.add("doc-1", Some(&meta)).unwrap();

        index.clear();
        assert!(index.is_empty());
    }

    #[test]
    fn test_stats() {
        let index = create_index();
        let meta = create_doc("Rust Programming", "Learn Rust language");
        index.add("doc-1", Some(&meta)).unwrap();

        let stats = index.stats();
        assert_eq!(stats.document_count, 1);
        assert!(stats.unique_terms > 0);
        assert!(stats.total_tokens > 0);
    }

    #[test]
    fn test_serialization() {
        let index = create_index();
        let meta = create_doc("Test", "Content for serialization");
        index.add("doc-1", Some(&meta)).unwrap();

        let serialized = index.serialize().unwrap();
        let restored = BM25Index::deserialize(
            vec!["title".into(), "content".into()],
            &serialized
        ).unwrap();

        assert_eq!(restored.len(), 1);
        let results = restored.search("serialization", 10);
        assert_eq!(results.len(), 1);
    }
}
