//! EmbeddingGemma: modelo de embedding multilingüe de Google.
//!
//! Basado en Gemma 3 pero con atención bidireccional (encoder),
//! mean pooling, y capas de proyección para generar embeddings de 768 dims.
//!
//! ## Arquitectura
//!
//! - **Atención bidireccional** con Grouped Query Attention (GQA)
//! - **RoPE** (Rotary Position Embeddings) para codificación posicional relativa
//! - **RMSNorm** para normalización eficiente
//! - **GeGLU** feed-forward con gate projection
//! - **Mean pooling** sobre tokens válidos (excluye padding)
//! - **Proyección MLP** (hidden → 768 dims con ReLU)
//!
//! ## RoPE (Rotary Position Embeddings)
//!
//! RoPE inyecta información posicional relativa rotando los vectores Q y K
//! en el espacio complejo. Para dos posiciones `m` y `n`, el producto punto
//! entre Q_m y K_n depende únicamente de la distancia relativa `m - n`,
//! no de las posiciones absolutas. Esto permite:
//!
//! - Generalización a secuencias más largas que las vistas en entrenamiento
//! - Decaimiento natural de la atención con la distancia
//! - Compatibilidad con atención bidireccional (no requiere causal mask)
//!
//! La implementación precomputa `cos(m·θ_i)` y `sin(m·θ_i)` para todas las
//! posiciones hasta `max_position_embeddings`, donde `θ_i = rope_theta^(-2i/d)`.
//!
//! ## Matryoshka Representation Learning
//!
//! Soporta truncación dimensional: las dimensiones se pueden truncar
//! a 512, 256, o 128 con mínima pérdida de calidad. Los primeros N
//! componentes del embedding capturan la información más importante.
//!
//! Referencia: <https://huggingface.co/google/embeddinggemma-300m>

use candle_core::{DType, Device, IndexOp, Module, Tensor};
use candle_nn::{linear_no_bias, Linear, VarBuilder};
use serde::Deserialize;
use tokenizers::{PaddingParams, PaddingStrategy, Tokenizer, TruncationParams};

use crate::error::{Error, Result};

use super::{download_model_files, l2_normalize, EmbedderConfig};

/// Configuración del modelo EmbeddingGemma (parseada de config.json).
#[derive(Debug, Clone, Deserialize)]
struct GemmaConfig {
    vocab_size: usize,
    hidden_size: usize,
    intermediate_size: usize,
    num_hidden_layers: usize,
    num_attention_heads: usize,
    num_key_value_heads: usize,
    head_dim: usize,
    max_position_embeddings: usize,
    rms_norm_eps: f64,
    #[serde(default = "default_rope_theta")]
    rope_theta: f64,
    /// Dimensiones de la proyección final (768 para EmbeddingGemma)
    #[serde(default = "default_embedding_dim")]
    embedding_dim: usize,
}

fn default_rope_theta() -> f64 {
    10000.0
}

fn default_embedding_dim() -> usize {
    768
}

/// EmbeddingGemma: encoder bidireccional basado en Gemma 3.
pub struct GemmaEmbedder {
    embeddings: candle_nn::Embedding,
    layers: Vec<GemmaLayer>,
    norm: RmsNorm,
    projection1: Linear,
    projection2: Linear,
    tokenizer: Tokenizer,
    device: Device,
    config: GemmaConfig,
    normalize: bool,
}

/// Capa transformer del modelo Gemma con atención bidireccional.
struct GemmaLayer {
    self_attn: GemmaAttention,
    mlp: GemmaMlp,
    input_layernorm: RmsNorm,
    post_attention_layernorm: RmsNorm,
}

/// Atención multi-head con Grouped Query Attention (GQA) y RoPE.
struct GemmaAttention {
    q_proj: Linear,
    k_proj: Linear,
    v_proj: Linear,
    o_proj: Linear,
    rotary_emb: RotaryEmbedding,
    num_heads: usize,
    num_kv_heads: usize,
    head_dim: usize,
}

/// Feed-forward network con GeGLU activation.
struct GemmaMlp {
    gate_proj: Linear,
    up_proj: Linear,
    down_proj: Linear,
}

/// RMS Layer Normalization.
struct RmsNorm {
    weight: Tensor,
    eps: f64,
}

impl RmsNorm {
    fn load(vb: &VarBuilder, size: usize, eps: f64) -> candle_core::Result<Self> {
        let weight = vb.get(size, "weight")?;
        Ok(Self { weight, eps })
    }

    fn forward(&self, x: &Tensor) -> candle_core::Result<Tensor> {
        let dtype = x.dtype();
        let x = x.to_dtype(DType::F32)?;
        let variance = x.sqr()?.mean_keepdim(candle_core::D::Minus1)?;
        let x = x.broadcast_div(&(variance + self.eps)?.sqrt()?)?;
        let x = x.to_dtype(dtype)?;
        x.broadcast_mul(&(&self.weight + 1.0)?)
    }
}

/// Rotary Position Embeddings (RoPE).
///
/// Aplica rotaciones basadas en la posición a los tensores Q y K,
/// permitiendo que el modelo capture información posicional relativa
/// incluso en atención bidireccional.
struct RotaryEmbedding {
    cos: Tensor,
    sin: Tensor,
}

impl RotaryEmbedding {
    fn new(head_dim: usize, max_seq_len: usize, theta: f64, device: &Device) -> candle_core::Result<Self> {
        let half_dim = head_dim / 2;
        // Frecuencias inversas: theta^(-2i/d) para i en [0, d/2)
        let inv_freq: Vec<f32> = (0..half_dim)
            .map(|i| 1.0 / theta.powf(i as f64 * 2.0 / head_dim as f64) as f32)
            .collect();
        let inv_freq = Tensor::new(inv_freq.as_slice(), device)?; // [half_dim]

        // Posiciones: [0, 1, 2, ..., max_seq_len-1]
        let positions: Vec<f32> = (0..max_seq_len).map(|p| p as f32).collect();
        let positions = Tensor::new(positions.as_slice(), device)?; // [max_seq_len]

        // Outer product: positions * inv_freq -> [max_seq_len, half_dim]
        let freqs = positions
            .unsqueeze(1)?
            .matmul(&inv_freq.unsqueeze(0)?)?;

        // Duplicar frecuencias para cubrir head_dim completo: [max_seq_len, head_dim]
        let emb = Tensor::cat(&[&freqs, &freqs], 1)?;

        let cos = emb.cos()?;
        let sin = emb.sin()?;

        Ok(Self { cos, sin })
    }

    /// Aplica RoPE a un tensor de shape [batch, heads, seq_len, head_dim].
    fn apply(&self, x: &Tensor, seq_len: usize) -> candle_core::Result<Tensor> {
        let cos = self.cos.i(..seq_len)?; // [seq_len, head_dim]
        let sin = self.sin.i(..seq_len)?;

        // Reshape para broadcast: [1, 1, seq_len, head_dim]
        let cos = cos.unsqueeze(0)?.unsqueeze(0)?;
        let sin = sin.unsqueeze(0)?.unsqueeze(0)?;

        // rotate_half: [-x2, x1] donde x = [x1, x2] dividido a la mitad
        let half_dim = x.dim(3)? / 2;
        let x1 = x.narrow(3, 0, half_dim)?;
        let x2 = x.narrow(3, half_dim, half_dim)?;
        let rotated = Tensor::cat(&[&x2.neg()?, &x1], 3)?;

        // x * cos + rotate_half(x) * sin
        x.broadcast_mul(&cos)?.add(&rotated.broadcast_mul(&sin)?)
    }
}

impl GemmaMlp {
    fn load(vb: &VarBuilder, config: &GemmaConfig) -> candle_core::Result<Self> {
        let gate_proj = linear_no_bias(config.hidden_size, config.intermediate_size, vb.pp("gate_proj"))?;
        let up_proj = linear_no_bias(config.hidden_size, config.intermediate_size, vb.pp("up_proj"))?;
        let down_proj = linear_no_bias(config.intermediate_size, config.hidden_size, vb.pp("down_proj"))?;
        Ok(Self {
            gate_proj,
            up_proj,
            down_proj,
        })
    }

    fn forward(&self, x: &Tensor) -> candle_core::Result<Tensor> {
        let gate = self.gate_proj.forward(x)?.gelu_erf()?;
        let up = self.up_proj.forward(x)?;
        self.down_proj.forward(&(gate * up)?)
    }
}

impl GemmaAttention {
    fn load(vb: &VarBuilder, config: &GemmaConfig, device: &Device) -> candle_core::Result<Self> {
        let hidden = config.hidden_size;
        let head_dim = config.head_dim;
        let num_heads = config.num_attention_heads;
        let num_kv_heads = config.num_key_value_heads;

        let q_proj = linear_no_bias(hidden, num_heads * head_dim, vb.pp("q_proj"))?;
        let k_proj = linear_no_bias(hidden, num_kv_heads * head_dim, vb.pp("k_proj"))?;
        let v_proj = linear_no_bias(hidden, num_kv_heads * head_dim, vb.pp("v_proj"))?;
        let o_proj = linear_no_bias(num_heads * head_dim, hidden, vb.pp("o_proj"))?;

        let rotary_emb = RotaryEmbedding::new(
            head_dim,
            config.max_position_embeddings,
            config.rope_theta,
            device,
        )?;

        Ok(Self {
            q_proj,
            k_proj,
            v_proj,
            o_proj,
            rotary_emb,
            num_heads,
            num_kv_heads,
            head_dim,
        })
    }

    /// Atención bidireccional con RoPE (sin máscara causal).
    fn forward(&self, x: &Tensor, attention_mask: Option<&Tensor>) -> candle_core::Result<Tensor> {
        let (batch, seq_len, _) = x.dims3()?;

        // Proyecciones Q, K, V
        let q = self.q_proj.forward(x)?;
        let k = self.k_proj.forward(x)?;
        let v = self.v_proj.forward(x)?;

        // Reshape para multi-head: [batch, seq, heads, head_dim] -> [batch, heads, seq, head_dim]
        let q = q
            .reshape((batch, seq_len, self.num_heads, self.head_dim))?
            .transpose(1, 2)?;
        let k = k
            .reshape((batch, seq_len, self.num_kv_heads, self.head_dim))?
            .transpose(1, 2)?;
        let v = v
            .reshape((batch, seq_len, self.num_kv_heads, self.head_dim))?
            .transpose(1, 2)?;

        // Aplicar RoPE a Q y K (inyecta información posicional relativa)
        let q = self.rotary_emb.apply(&q, seq_len)?;
        let k = self.rotary_emb.apply(&k, seq_len)?;

        // GQA: expandir K, V si num_kv_heads < num_heads
        let (k, v) = if self.num_kv_heads < self.num_heads {
            let repeat = self.num_heads / self.num_kv_heads;
            let k = k
                .unsqueeze(2)?
                .expand((batch, self.num_kv_heads, repeat, seq_len, self.head_dim))?
                .reshape((batch, self.num_heads, seq_len, self.head_dim))?;
            let v = v
                .unsqueeze(2)?
                .expand((batch, self.num_kv_heads, repeat, seq_len, self.head_dim))?
                .reshape((batch, self.num_heads, seq_len, self.head_dim))?;
            (k, v)
        } else {
            (k, v)
        };

        // Scaling
        let scale = (self.head_dim as f64).sqrt();

        // QK^T / sqrt(d)
        let attn_weights = q.matmul(&k.transpose(2, 3)?)?.affine(1.0 / scale, 0.0)?;

        // Aplicar attention mask (para padding, NO causal)
        let attn_weights = if let Some(mask) = attention_mask {
            attn_weights.broadcast_add(mask)?
        } else {
            attn_weights
        };

        // Softmax
        let attn_weights = candle_nn::ops::softmax_last_dim(&attn_weights)?;

        // Atención * V
        let output = attn_weights.matmul(&v)?;

        // Reshape: [batch, heads, seq, head_dim] -> [batch, seq, hidden]
        let output = output
            .transpose(1, 2)?
            .reshape((batch, seq_len, self.num_heads * self.head_dim))?;

        self.o_proj.forward(&output)
    }
}

impl GemmaLayer {
    fn load(vb: &VarBuilder, config: &GemmaConfig, device: &Device) -> candle_core::Result<Self> {
        let self_attn = GemmaAttention::load(&vb.pp("self_attn"), config, device)?;
        let mlp = GemmaMlp::load(&vb.pp("mlp"), config)?;
        let input_layernorm =
            RmsNorm::load(&vb.pp("input_layernorm"), config.hidden_size, config.rms_norm_eps)?;
        let post_attention_layernorm = RmsNorm::load(
            &vb.pp("post_attention_layernorm"),
            config.hidden_size,
            config.rms_norm_eps,
        )?;
        Ok(Self {
            self_attn,
            mlp,
            input_layernorm,
            post_attention_layernorm,
        })
    }

    fn forward(&self, x: &Tensor, attention_mask: Option<&Tensor>) -> candle_core::Result<Tensor> {
        // Pre-norm + attention + residual
        let residual = x;
        let x = self.input_layernorm.forward(x)?;
        let x = self.self_attn.forward(&x, attention_mask)?;
        let x = (residual + x)?;

        // Pre-norm + MLP + residual
        let residual = &x;
        let x = self.post_attention_layernorm.forward(&x)?;
        let x = self.mlp.forward(&x)?;
        residual + x
    }
}

impl GemmaEmbedder {
    /// Carga el modelo EmbeddingGemma desde HuggingFace Hub.
    pub fn load(config: &EmbedderConfig) -> Result<Self> {
        let model_id = config.model.model_id();
        let device = Device::Cpu;

        // Descargar archivos del modelo
        let files = download_model_files(
            model_id,
            &["config.json", "tokenizer.json", "model.safetensors"],
            config.cache_dir.as_ref(),
        )?;

        let config_path = &files[0];
        let tokenizer_path = &files[1];
        let weights_path = &files[2];

        // Cargar configuración
        let gemma_config: GemmaConfig = {
            let config_str = std::fs::read_to_string(config_path)
                .map_err(|e| Error::InvalidConfig(format!("Failed to read config.json: {}", e)))?;
            serde_json::from_str(&config_str).map_err(|e| {
                Error::InvalidConfig(format!("Failed to parse Gemma config: {}", e))
            })?
        };

        let max_length = config.max_length.unwrap_or(2048);

        // Cargar tokenizer
        let mut tokenizer = Tokenizer::from_file(tokenizer_path).map_err(|e| {
            Error::InvalidConfig(format!("Failed to load tokenizer: {}", e))
        })?;

        let _ = tokenizer.with_truncation(Some(TruncationParams {
            max_length,
            ..Default::default()
        }));
        tokenizer.with_padding(Some(PaddingParams {
            strategy: PaddingStrategy::BatchLongest,
            ..Default::default()
        }));

        // Cargar pesos
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[weights_path.clone()], DType::F32, &device)
                .map_err(|e| {
                    Error::InvalidConfig(format!("Failed to load model weights: {}", e))
                })?
        };

        // Construir modelo
        let model_vb = vb.pp("model");

        // Token embeddings
        let embeddings = candle_nn::embedding(
            gemma_config.vocab_size,
            gemma_config.hidden_size,
            model_vb.pp("embed_tokens"),
        )
        .map_err(|e| Error::InvalidConfig(format!("Failed to load embeddings: {}", e)))?;

        // Transformer layers
        let mut layers = Vec::with_capacity(gemma_config.num_hidden_layers);
        for i in 0..gemma_config.num_hidden_layers {
            let layer = GemmaLayer::load(&model_vb.pp(format!("layers.{}", i)), &gemma_config, &device)
                .map_err(|e| {
                    Error::InvalidConfig(format!("Failed to load layer {}: {}", i, e))
                })?;
            layers.push(layer);
        }

        // Final norm
        let norm = RmsNorm::load(
            &model_vb.pp("norm"),
            gemma_config.hidden_size,
            gemma_config.rms_norm_eps,
        )
        .map_err(|e| Error::InvalidConfig(format!("Failed to load final norm: {}", e)))?;

        // Projection layers (EmbeddingGemma specific)
        // These project from hidden_size to embedding_dim (768)
        let proj_vb = vb.pp("projector");
        let projection1 = candle_nn::linear(
            gemma_config.hidden_size,
            gemma_config.embedding_dim,
            proj_vb.pp("linear1"),
        )
        .or_else(|_| {
            // Fallback: try loading as projection.0
            candle_nn::linear(
                gemma_config.hidden_size,
                gemma_config.embedding_dim,
                vb.pp("projection.0"),
            )
        })
        .map_err(|e| {
            Error::InvalidConfig(format!("Failed to load projection layer 1: {}", e))
        })?;

        let projection2 = candle_nn::linear(
            gemma_config.embedding_dim,
            gemma_config.embedding_dim,
            proj_vb.pp("linear2"),
        )
        .or_else(|_| {
            candle_nn::linear(
                gemma_config.embedding_dim,
                gemma_config.embedding_dim,
                vb.pp("projection.2"),
            )
        })
        .map_err(|e| {
            Error::InvalidConfig(format!("Failed to load projection layer 2: {}", e))
        })?;

        Ok(Self {
            embeddings,
            layers,
            norm,
            projection1,
            projection2,
            tokenizer,
            device,
            config: gemma_config,
            normalize: config.normalize,
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
        let mut all_attention_mask = Vec::with_capacity(batch_size * max_len);

        for encoding in &encodings {
            let ids = encoding.get_ids();
            let attention = encoding.get_attention_mask();

            all_ids.extend_from_slice(ids);
            all_attention_mask.extend_from_slice(attention);

            let len = ids.len();
            for _ in len..max_len {
                all_ids.push(0);
                all_attention_mask.push(0);
            }
        }

        let input_ids = Tensor::new(all_ids.as_slice(), &self.device)
            .and_then(|t| t.reshape((batch_size, max_len)))
            .map_err(|e| Error::InvalidConfig(format!("Failed to create input tensor: {}", e)))?;

        let attention_mask_raw = Tensor::new(all_attention_mask.as_slice(), &self.device)
            .and_then(|t| t.reshape((batch_size, max_len)))
            .map_err(|e| {
                Error::InvalidConfig(format!("Failed to create attention_mask tensor: {}", e))
            })?;

        // Crear attention mask para transformer (0 = attend, -inf = mask)
        let attention_mask_4d = self
            .make_attention_mask(&attention_mask_raw, batch_size, max_len)
            .map_err(|e| Error::InvalidConfig(format!("Failed to create 4D mask: {}", e)))?;

        // Forward pass
        // 1. Token embeddings + scaling
        let hidden_size_sqrt = (self.config.hidden_size as f64).sqrt();
        let mut hidden = self
            .embeddings
            .forward(&input_ids)
            .and_then(|t| t.affine(hidden_size_sqrt, 0.0))
            .map_err(|e| Error::InvalidConfig(format!("Embedding lookup failed: {}", e)))?;

        // 2. Transformer layers (bidireccional - sin máscara causal)
        for (i, layer) in self.layers.iter().enumerate() {
            hidden = layer
                .forward(&hidden, Some(&attention_mask_4d))
                .map_err(|e| {
                    Error::InvalidConfig(format!("Layer {} forward failed: {}", i, e))
                })?;
        }

        // 3. Final norm
        hidden = self
            .norm
            .forward(&hidden)
            .map_err(|e| Error::InvalidConfig(format!("Final norm failed: {}", e)))?;

        // 4. Mean pooling
        let pooled = mean_pooling(&hidden, &attention_mask_raw)
            .map_err(|e| Error::InvalidConfig(format!("Mean pooling failed: {}", e)))?;

        // 5. Projection layers (linear1 -> ReLU -> linear2)
        let projected = self
            .projection1
            .forward(&pooled)
            .and_then(|t| t.relu())
            .and_then(|t| self.projection2.forward(&t))
            .map_err(|e| Error::InvalidConfig(format!("Projection failed: {}", e)))?;

        // Convertir a Vec<Vec<f32>>
        let mut results = Vec::with_capacity(batch_size);
        for i in 0..batch_size {
            let emb = projected.get(i).map_err(|e| {
                Error::InvalidConfig(format!("Failed to get embedding {}: {}", i, e))
            })?;
            let mut vec: Vec<f32> = emb.to_vec1().map_err(|e| {
                Error::InvalidConfig(format!("Failed to convert to vec: {}", e))
            })?;

            if self.normalize {
                l2_normalize(&mut vec);
            }

            results.push(vec);
        }

        Ok(results)
    }

    /// Crea attention mask 4D para padding (bidireccional, no causal).
    ///
    /// Shape: [batch, 1, 1, seq_len] donde 0.0 = attend, -inf = mask
    fn make_attention_mask(
        &self,
        mask: &Tensor,
        batch_size: usize,
        seq_len: usize,
    ) -> candle_core::Result<Tensor> {
        let mask = mask.to_dtype(DType::F32)?;
        // Invertir: 1->0 (attend), 0->-inf (mask)
        let inverted = (1.0 - &mask)?;
        let large_neg = inverted.affine(-1e9, 0.0)?;
        // Reshape a [batch, 1, 1, seq_len] para broadcast en attention
        large_neg.reshape((batch_size, 1, 1, seq_len))
    }
}

/// Mean pooling: promedio ponderado por attention mask.
fn mean_pooling(output: &Tensor, attention_mask: &Tensor) -> candle_core::Result<Tensor> {
    let mask = attention_mask
        .to_dtype(DType::F32)?
        .unsqueeze(2)?
        .broadcast_as(output.shape())?;

    let masked = output.mul(&mask)?;
    let sum = masked.sum(1)?;
    let count = mask.sum(1)?;
    let count = count.clamp(1e-9, f64::MAX)?;

    sum.div(&count)
}
