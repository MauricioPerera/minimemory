//! Embedder basado en BERT/Sentence-Transformers usando Candle.
//!
//! Soporta modelos como all-MiniLM-L6-v2, BGE-small-en-v1.5,
//! y cualquier modelo compatible con sentence-transformers.

use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config as BertConfig};
use tokenizers::{PaddingParams, PaddingStrategy, Tokenizer, TruncationParams};

use crate::error::{Error, Result};

use super::{download_model_files, l2_normalize, EmbedderConfig};

/// Embedder basado en arquitectura BERT.
///
/// Usa mean pooling sobre los token embeddings para generar
/// una representación de oración/párrafo.
pub struct BertEmbedder {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
    normalize: bool,
    #[allow(dead_code)]
    max_length: usize,
}

impl BertEmbedder {
    /// Carga el modelo desde HuggingFace Hub.
    pub fn load(config: &EmbedderConfig) -> Result<Self> {
        let model_id = config.model.model_id();
        let device = Device::Cpu;
        let max_length = config.max_length.unwrap_or(512);

        // Descargar archivos del modelo
        let files = download_model_files(
            model_id,
            &["config.json", "tokenizer.json", "model.safetensors"],
            config.cache_dir.as_ref(),
        )?;

        let config_path = &files[0];
        let tokenizer_path = &files[1];
        let weights_path = &files[2];

        // Cargar configuración BERT
        let bert_config: BertConfig = {
            let config_str = std::fs::read_to_string(config_path)
                .map_err(|e| Error::InvalidConfig(format!("Failed to read config.json: {}", e)))?;
            serde_json::from_str(&config_str).map_err(|e| {
                Error::InvalidConfig(format!("Failed to parse BERT config: {}", e))
            })?
        };

        // Cargar tokenizer
        let mut tokenizer = Tokenizer::from_file(tokenizer_path).map_err(|e| {
            Error::InvalidConfig(format!("Failed to load tokenizer: {}", e))
        })?;

        // Configurar truncation y padding
        let _ = tokenizer.with_truncation(Some(TruncationParams {
            max_length,
            ..Default::default()
        }));
        tokenizer.with_padding(Some(PaddingParams {
            strategy: PaddingStrategy::BatchLongest,
            ..Default::default()
        }));

        // Cargar pesos del modelo
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[weights_path.clone()], DType::F32, &device)
                .map_err(|e| {
                    Error::InvalidConfig(format!("Failed to load model weights: {}", e))
                })?
        };

        let model = BertModel::load(vb, &bert_config)
            .map_err(|e| Error::InvalidConfig(format!("Failed to build BERT model: {}", e)))?;

        Ok(Self {
            model,
            tokenizer,
            device,
            normalize: config.normalize,
            max_length,
        })
    }

    /// Genera embedding para un texto.
    pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let results = self.embed_batch(&[text])?;
        Ok(results.into_iter().next().unwrap())
    }

    /// Genera embeddings para un batch de textos.
    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        // Tokenizar
        let encodings = self.tokenizer.encode_batch(texts.to_vec(), true).map_err(|e| {
            Error::InvalidConfig(format!("Tokenization failed: {}", e))
        })?;

        let batch_size = encodings.len();
        let max_len = encodings.iter().map(|e| e.get_ids().len()).max().unwrap_or(0);

        // Construir tensores de input
        let mut all_ids = Vec::with_capacity(batch_size * max_len);
        let mut all_type_ids = Vec::with_capacity(batch_size * max_len);
        let mut all_attention_mask = Vec::with_capacity(batch_size * max_len);

        for encoding in &encodings {
            let ids = encoding.get_ids();
            let type_ids = encoding.get_type_ids();
            let attention = encoding.get_attention_mask();

            let len = ids.len();
            all_ids.extend_from_slice(ids);
            all_type_ids.extend_from_slice(type_ids);
            all_attention_mask.extend_from_slice(attention);

            // Padding (ya debería estar hecho por el tokenizer, pero por seguridad)
            for _ in len..max_len {
                all_ids.push(0);
                all_type_ids.push(0);
                all_attention_mask.push(0);
            }
        }

        let input_ids = Tensor::new(all_ids.as_slice(), &self.device)
            .and_then(|t| t.reshape((batch_size, max_len)))
            .map_err(|e| Error::InvalidConfig(format!("Failed to create input tensor: {}", e)))?;

        let token_type_ids = Tensor::new(all_type_ids.as_slice(), &self.device)
            .and_then(|t| t.reshape((batch_size, max_len)))
            .map_err(|e| Error::InvalidConfig(format!("Failed to create type_ids tensor: {}", e)))?;

        let attention_mask_tensor = Tensor::new(all_attention_mask.as_slice(), &self.device)
            .and_then(|t| t.reshape((batch_size, max_len)))
            .map_err(|e| {
                Error::InvalidConfig(format!("Failed to create attention_mask tensor: {}", e))
            })?;

        // Forward pass
        let output = self
            .model
            .forward(&input_ids, &token_type_ids, Some(&attention_mask_tensor))
            .map_err(|e| Error::InvalidConfig(format!("BERT forward pass failed: {}", e)))?;

        // Mean pooling: promedio de token embeddings, enmascarado por attention
        let embeddings = mean_pooling(&output, &attention_mask_tensor)
            .map_err(|e| Error::InvalidConfig(format!("Mean pooling failed: {}", e)))?;

        // Convertir a Vec<Vec<f32>>
        let mut results = Vec::with_capacity(batch_size);
        for i in 0..batch_size {
            let emb = embeddings
                .get(i)
                .map_err(|e| Error::InvalidConfig(format!("Failed to get embedding {}: {}", i, e)))?;
            let mut vec: Vec<f32> = emb
                .to_vec1()
                .map_err(|e| Error::InvalidConfig(format!("Failed to convert to vec: {}", e)))?;

            if self.normalize {
                l2_normalize(&mut vec);
            }

            results.push(vec);
        }

        Ok(results)
    }
}

/// Mean pooling: promedio ponderado por attention mask.
///
/// Para cada secuencia, promedia los token embeddings ignorando los tokens de padding.
fn mean_pooling(output: &Tensor, attention_mask: &Tensor) -> candle_core::Result<Tensor> {
    let (_batch, _seq_len, _hidden) = output.dims3()?;

    // Expandir attention_mask de [batch, seq] a [batch, seq, hidden]
    let mask = attention_mask
        .to_dtype(DType::F32)?
        .unsqueeze(2)?
        .broadcast_as(output.shape())?;

    // Multiplicar output por mask y sumar sobre la dimensión de secuencia
    let masked = output.mul(&mask)?;
    let sum = masked.sum(1)?; // [batch, hidden]

    // Contar tokens válidos por secuencia
    let count = mask.sum(1)?; // [batch, hidden]
    let count = count.clamp(1e-9, f64::MAX)?; // Evitar división por cero

    sum.div(&count)
}
