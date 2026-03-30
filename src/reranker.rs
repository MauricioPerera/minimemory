//! # Reranker Module
//!
//! Cross-encoder reranking for search results.
//! Users provide their own ranking function (HTTP client, local model, etc.)
//!
//! ## Example
//!
//! ```rust,ignore
//! use minimemory::reranker::{Reranker, RerankResult};
//!
//! let reranker = Reranker::new(|query, documents| {
//!     // Call your reranking API here
//!     Ok(documents.iter().enumerate().map(|(i, _)| {
//!         RerankResult { index: i, score: 1.0 / (i as f32 + 1.0) }
//!     }).collect())
//! });
//!
//! let results = reranker.rank("what is rust?", &["Rust is a language".into(), "Iron oxide".into()]).unwrap();
//! ```

use crate::error::Result;
use crate::types::SearchResult;

/// Result from a reranking operation.
#[derive(Debug, Clone)]
pub struct RerankResult {
    /// Index into the original documents array
    pub index: usize,
    /// Relevance score (higher = more relevant)
    pub score: f32,
}

/// Cross-encoder reranker that delegates to a user-provided function.
///
/// This design avoids adding HTTP client dependencies — the user provides
/// a closure that calls their preferred reranking API or local model.
pub struct Reranker {
    rerank_fn: Box<dyn Fn(&str, &[String]) -> Result<Vec<RerankResult>> + Send + Sync>,
}

impl Reranker {
    /// Create a new reranker with a custom ranking function.
    ///
    /// The function receives (query, documents) and returns scored results.
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(&str, &[String]) -> Result<Vec<RerankResult>> + Send + Sync + 'static,
    {
        Self {
            rerank_fn: Box::new(f),
        }
    }

    /// Rank documents against a query using the configured reranking function.
    ///
    /// Returns results sorted by score (highest first).
    pub fn rank(&self, query: &str, documents: &[String]) -> Result<Vec<RerankResult>> {
        let mut results = (self.rerank_fn)(query, documents)?;
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        Ok(results)
    }

    /// Rerank search results using text from a metadata field.
    ///
    /// Extracts text from the specified metadata field of each result,
    /// reranks using the cross-encoder, and returns results in new order.
    ///
    /// Results without the specified text field are appended at the end
    /// with score 0.
    pub fn rerank_search(
        &self,
        query: &str,
        results: Vec<SearchResult>,
        text_field: &str,
    ) -> Result<Vec<SearchResult>> {
        if results.is_empty() {
            return Ok(results);
        }

        // Extract text documents from metadata
        let mut texts: Vec<String> = Vec::new();
        let mut text_indices: Vec<usize> = Vec::new(); // maps text index to result index
        let mut no_text_results: Vec<SearchResult> = Vec::new();

        for (i, result) in results.iter().enumerate() {
            if let Some(ref meta) = result.metadata {
                if let Some(crate::types::MetadataValue::String(text)) = meta.get(text_field) {
                    texts.push(text.clone());
                    text_indices.push(i);
                    continue;
                }
            }
            no_text_results.push(result.clone());
        }

        if texts.is_empty() {
            return Ok(results);
        }

        // Rerank the texts
        let ranked = self.rank(query, &texts)?;

        // Rebuild results in reranked order
        let mut reranked: Vec<SearchResult> = Vec::with_capacity(results.len());
        for rr in &ranked {
            if rr.index < text_indices.len() {
                let orig_idx = text_indices[rr.index];
                let mut result = results[orig_idx].clone();
                result.distance = 1.0 - rr.score; // Convert score to distance
                reranked.push(result);
            }
        }

        // Append results that had no text
        reranked.extend(no_text_results);

        Ok(reranked)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reranker_basic() {
        let reranker = Reranker::new(|_query, documents| {
            // Simple mock: reverse order (last doc is most relevant)
            Ok(documents
                .iter()
                .enumerate()
                .map(|(i, _)| RerankResult {
                    index: i,
                    score: i as f32,
                })
                .collect())
        });

        let docs = vec![
            "first document".to_string(),
            "second document".to_string(),
            "third document".to_string(),
        ];

        let results = reranker.rank("query", &docs).unwrap();
        assert_eq!(results.len(), 3);
        // Highest score first
        assert_eq!(results[0].index, 2);
        assert_eq!(results[1].index, 1);
        assert_eq!(results[2].index, 0);
    }

    #[test]
    fn test_reranker_empty() {
        let reranker = Reranker::new(|_q, _d| Ok(vec![]));
        let results = reranker.rank("query", &[]).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_rerank_search_results() {
        use crate::types::Metadata;

        let reranker = Reranker::new(|_query, documents| {
            // Mock: score based on document length (longer = more relevant)
            Ok(documents
                .iter()
                .enumerate()
                .map(|(i, doc)| RerankResult {
                    index: i,
                    score: doc.len() as f32,
                })
                .collect())
        });

        let mut meta1 = Metadata::new();
        meta1.insert("text", "short");
        let mut meta2 = Metadata::new();
        meta2.insert("text", "this is a much longer document");

        let results = vec![
            SearchResult {
                id: "a".to_string(),
                distance: 0.1,
                metadata: Some(meta1),
            },
            SearchResult {
                id: "b".to_string(),
                distance: 0.2,
                metadata: Some(meta2),
            },
        ];

        let reranked = reranker.rerank_search("query", results, "text").unwrap();
        // "b" should be first (longer text = higher score)
        assert_eq!(reranked[0].id, "b");
        assert_eq!(reranked[1].id, "a");
    }
}
