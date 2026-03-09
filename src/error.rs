use thiserror::Error;

/// Result type for minimemory operations
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur in minimemory
#[derive(Error, Debug)]
pub enum Error {
    #[error("Vector dimension mismatch: expected {expected}, got {got}")]
    DimensionMismatch { expected: usize, got: usize },

    #[error("Vector with id '{0}' not found")]
    NotFound(String),

    #[error("Vector with id '{0}' already exists")]
    AlreadyExists(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Bincode error: {0}")]
    Bincode(#[from] bincode::Error),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Database is empty")]
    EmptyDatabase,

    #[error("Full-text search not enabled. Use VectorDB::with_fulltext() to enable.")]
    FulltextNotEnabled,

    #[error("Invalid filter: {0}")]
    InvalidFilter(String),

    #[error("Embedding error: {0}")]
    Embedding(String),
}
