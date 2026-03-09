//! # Embeddings locales para minimemory
//!
//! Genera embeddings directamente en Rust usando modelos de HuggingFace,
//! sin necesidad de APIs externas.
//!
//! ## Modelos soportados
//!
//! - **MiniLM-L6-v2**: Modelo ligero (22.7M params, 384 dims). Rápido y eficiente.
//! - **BGE-small-en-v1.5**: Modelo de BAAI (33.4M params, 384 dims). Buena calidad para inglés.
//! - **EmbeddingGemma**: Modelo de Google (308M params, 768 dims). Multilingüe, Matryoshka.
//!
//! ## Ejemplo
//!
//! ```rust,ignore
//! use minimemory::embeddings::{Embedder, EmbeddingModel};
//!
//! // Crear embedder con modelo ligero
//! let embedder = Embedder::new(EmbeddingModel::MiniLM)?;
//!
//! // Generar embedding
//! let vector = embedder.embed("Hello, world!")?;
//! assert_eq!(vector.len(), 384);
//!
//! // Generar embeddings en batch
//! let vectors = embedder.embed_batch(&["text 1", "text 2", "text 3"])?;
//!
//! // Modelo multilingüe con dimensiones Matryoshka
//! let embedder = Embedder::new(EmbeddingModel::Gemma { dimensions: 256 })?;
//! let vector = embedder.embed("Texto en español")?;
//! assert_eq!(vector.len(), 256);
//! ```

mod bert;
mod gemma;

use crate::error::{Error, Result};
use std::path::PathBuf;

// Re-exports
pub use bert::BertEmbedder;
pub use gemma::GemmaEmbedder;

/// Modelos de embedding disponibles.
#[derive(Debug, Clone)]
pub enum EmbeddingModel {
    /// all-MiniLM-L6-v2 — 384 dims, 22.7M params, rápido
    MiniLM,
    /// BAAI/bge-small-en-v1.5 — 384 dims, 33.4M params, alta calidad (inglés)
    BgeSmall,
    /// google/embeddinggemma-300m — 768 dims (truncable a 512/256/128), multilingüe
    Gemma {
        /// Dimensiones de salida (768, 512, 256, 128). Usa Matryoshka truncation.
        dimensions: usize,
    },
    /// Modelo BERT/sentence-transformer custom desde HuggingFace Hub
    Custom {
        /// ID del modelo en HuggingFace (e.g., "sentence-transformers/paraphrase-MiniLM-L3-v2")
        model_id: String,
        /// Dimensiones de salida del modelo
        dimensions: usize,
    },
}

impl EmbeddingModel {
    /// Retorna las dimensiones de salida del modelo.
    pub fn dimensions(&self) -> usize {
        match self {
            EmbeddingModel::MiniLM => 384,
            EmbeddingModel::BgeSmall => 384,
            EmbeddingModel::Gemma { dimensions } => *dimensions,
            EmbeddingModel::Custom { dimensions, .. } => *dimensions,
        }
    }

    /// Retorna el ID del modelo en HuggingFace Hub.
    pub fn model_id(&self) -> &str {
        match self {
            EmbeddingModel::MiniLM => "sentence-transformers/all-MiniLM-L6-v2",
            EmbeddingModel::BgeSmall => "BAAI/bge-small-en-v1.5",
            EmbeddingModel::Gemma { .. } => "google/embeddinggemma-300m",
            EmbeddingModel::Custom { model_id, .. } => model_id,
        }
    }

    /// Retorna las dimensiones nativas (antes de truncar por Matryoshka).
    pub fn native_dimensions(&self) -> usize {
        match self {
            EmbeddingModel::MiniLM => 384,
            EmbeddingModel::BgeSmall => 384,
            EmbeddingModel::Gemma { .. } => 768,
            EmbeddingModel::Custom { dimensions, .. } => *dimensions,
        }
    }

    /// Indica si el modelo soporta Matryoshka (truncación dimensional).
    pub fn supports_matryoshka(&self) -> bool {
        matches!(self, EmbeddingModel::Gemma { .. })
    }

    /// Indica si el modelo es tipo Gemma (requiere implementación custom).
    fn is_gemma(&self) -> bool {
        matches!(self, EmbeddingModel::Gemma { .. })
    }
}

/// Generador de embeddings local.
///
/// Carga un modelo de HuggingFace Hub y genera embeddings en CPU.
/// Los modelos se descargan y cachean automáticamente en `~/.cache/huggingface/`.
pub struct Embedder {
    inner: EmbedderInner,
    model: EmbeddingModel,
}

enum EmbedderInner {
    Bert(BertEmbedder),
    Gemma(GemmaEmbedder),
}

/// Opciones de configuración para el Embedder.
#[derive(Debug, Clone)]
pub struct EmbedderConfig {
    /// Modelo a usar
    pub model: EmbeddingModel,
    /// Directorio de cache para modelos descargados (default: ~/.cache/huggingface/)
    pub cache_dir: Option<PathBuf>,
    /// Normalizar embeddings con L2 (default: true)
    pub normalize: bool,
    /// Longitud máxima de tokens (default: 512 para BERT, 2048 para Gemma)
    pub max_length: Option<usize>,
}

impl EmbedderConfig {
    pub fn new(model: EmbeddingModel) -> Self {
        Self {
            model,
            cache_dir: None,
            normalize: true,
            max_length: None,
        }
    }

    pub fn with_cache_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.cache_dir = Some(path.into());
        self
    }

    pub fn with_normalize(mut self, normalize: bool) -> Self {
        self.normalize = normalize;
        self
    }

    pub fn with_max_length(mut self, max_length: usize) -> Self {
        self.max_length = Some(max_length);
        self
    }
}

impl Embedder {
    /// Crea un nuevo Embedder con el modelo especificado.
    ///
    /// Descarga el modelo desde HuggingFace Hub si no está en cache.
    pub fn new(model: EmbeddingModel) -> Result<Self> {
        Self::with_config(EmbedderConfig::new(model))
    }

    /// Crea un Embedder con configuración personalizada.
    pub fn with_config(config: EmbedderConfig) -> Result<Self> {
        let model = config.model.clone();

        let inner = if model.is_gemma() {
            let gemma = GemmaEmbedder::load(&config)?;
            EmbedderInner::Gemma(gemma)
        } else {
            let bert = BertEmbedder::load(&config)?;
            EmbedderInner::Bert(bert)
        };

        Ok(Self { inner, model })
    }

    /// Genera embedding para un texto.
    pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let mut embedding = match &self.inner {
            EmbedderInner::Bert(bert) => bert.embed(text)?,
            EmbedderInner::Gemma(gemma) => gemma.embed(text)?,
        };

        // Matryoshka truncation si el modelo lo soporta y las dims son menores que las nativas
        let target_dims = self.model.dimensions();
        if embedding.len() > target_dims {
            embedding.truncate(target_dims);
            // Re-normalizar después de truncar (requerido para Matryoshka)
            l2_normalize(&mut embedding);
        }

        Ok(embedding)
    }

    /// Genera embeddings para un batch de textos.
    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let mut results = match &self.inner {
            EmbedderInner::Bert(bert) => bert.embed_batch(texts)?,
            EmbedderInner::Gemma(gemma) => gemma.embed_batch(texts)?,
        };

        // Matryoshka truncation
        let target_dims = self.model.dimensions();
        for embedding in &mut results {
            if embedding.len() > target_dims {
                embedding.truncate(target_dims);
                l2_normalize(embedding);
            }
        }

        Ok(results)
    }

    /// Retorna el modelo configurado.
    pub fn model(&self) -> &EmbeddingModel {
        &self.model
    }

    /// Retorna las dimensiones de salida.
    pub fn dimensions(&self) -> usize {
        self.model.dimensions()
    }

    /// Convierte el Embedder en una función compatible con `AgentMemory::set_embed_fn`.
    ///
    /// # Ejemplo
    ///
    /// ```rust,ignore
    /// use minimemory::embeddings::{Embedder, EmbeddingModel};
    /// use minimemory::agent_memory::{AgentMemory, MemoryConfig};
    ///
    /// let embedder = Embedder::new(EmbeddingModel::MiniLM)?;
    /// let mut memory = AgentMemory::new(MemoryConfig::small())?;
    /// memory.set_embed_fn(embedder.into_embed_fn());
    /// ```
    pub fn into_embed_fn(self) -> impl Fn(&str) -> Vec<f32> + Send + Sync + 'static {
        use std::sync::Arc;
        let embedder = Arc::new(self);
        move |text: &str| -> Vec<f32> {
            embedder
                .embed(text)
                .unwrap_or_else(|_| vec![0.0; embedder.dimensions()])
        }
    }
}

/// Descarga archivos del modelo desde HuggingFace Hub.
pub(crate) fn download_model_files(
    model_id: &str,
    filenames: &[&str],
    cache_dir: Option<&PathBuf>,
) -> Result<Vec<PathBuf>> {
    use hf_hub::api::tokio::{Api, ApiBuilder};
    use hf_hub::{Cache, Repo, RepoType};

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| Error::InvalidConfig(format!("Failed to create tokio runtime: {}", e)))?;

    rt.block_on(async {
        let api: Api = if let Some(cache) = cache_dir {
            let cache = Cache::new(cache.clone());
            ApiBuilder::from_cache(cache).build().map_err(|e| {
                Error::InvalidConfig(format!("Failed to create HF Hub API: {}", e))
            })?
        } else {
            Api::new()
                .map_err(|e| Error::InvalidConfig(format!("Failed to create HF Hub API: {}", e)))?
        };

        let repo = api.repo(Repo::new(model_id.to_string(), RepoType::Model));

        let mut paths = Vec::new();
        for filename in filenames {
            let path = repo.get(filename).await.map_err(|e| {
                Error::Embedding(format!(
                    "Failed to download '{}' from '{}': {}",
                    filename, model_id, e
                ))
            })?;
            paths.push(path);
        }

        Ok(paths)
    })
}

/// Normaliza un vector con L2.
pub(crate) fn l2_normalize(v: &mut [f32]) {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 1e-12 {
        v.iter_mut().for_each(|x| *x /= norm);
    }
}
