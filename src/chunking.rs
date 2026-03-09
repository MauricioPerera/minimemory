//! Módulo de chunking para procesamiento de Markdown.
//!
//! Integra `mq-markdown` para parsing inteligente de documentos Markdown,
//! extrayendo chunks estructurados listos para vectorización.
//!
//! # Ejemplo
//!
//! ```rust,ignore
//! use minimemory::chunking::{ChunkConfig, ChunkStrategy, chunk_markdown};
//!
//! let markdown = r#"
//! # Introducción
//! Este es el contenido de la introducción.
//!
//! ## Sección 1
//! Contenido de la sección 1.
//!
//! ```rust
//! fn main() {
//!     println!("Hello!");
//! }
//! ```
//!
//! ## Sección 2
//! Más contenido aquí.
//! "#;
//!
//! let config = ChunkConfig::default()
//!     .with_strategy(ChunkStrategy::ByHeading { max_level: 2 })
//!     .with_max_size(1000)
//!     .with_overlap(100);
//!
//! let chunks = chunk_markdown(markdown, &config).unwrap();
//!
//! for chunk in chunks {
//!     println!("Chunk: {} ({} chars)", chunk.id, chunk.content.len());
//!     println!("  Heading: {:?}", chunk.metadata.heading);
//!     println!("  Type: {:?}", chunk.metadata.chunk_type);
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::LazyLock;

use crate::error::{Error, Result};
use crate::types::Metadata;

/// Compiled heading regex (cached for reuse across calls)
static HEADING_PATTERN: LazyLock<regex_lite::Regex> =
    LazyLock::new(|| regex_lite::Regex::new(r"^(#{1,6})\s+(.+)$").unwrap());

/// Estrategia de chunking para dividir documentos.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChunkStrategy {
    /// Dividir por headings (H1, H2, etc.)
    ByHeading {
        /// Nivel máximo de heading a considerar (1-6)
        max_level: u8,
    },
    /// Dividir por tamaño fijo con overlap
    BySize {
        /// Tamaño objetivo de cada chunk en caracteres
        target_size: usize,
        /// Caracteres de overlap entre chunks
        overlap: usize,
    },
    /// Dividir por párrafos
    ByParagraph {
        /// Mínimo de párrafos por chunk
        min_paragraphs: usize,
        /// Máximo de párrafos por chunk
        max_paragraphs: usize,
    },
    /// Dividir por bloques de código (cada bloque es un chunk separado)
    ByCodeBlocks,
    /// Estrategia híbrida: headings + tamaño máximo
    Hybrid {
        /// Nivel máximo de heading
        max_heading_level: u8,
        /// Tamaño máximo antes de subdividir
        max_chunk_size: usize,
    },
}

impl Default for ChunkStrategy {
    fn default() -> Self {
        ChunkStrategy::ByHeading { max_level: 2 }
    }
}

/// Configuración para el proceso de chunking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkConfig {
    /// Estrategia de chunking a usar
    pub strategy: ChunkStrategy,
    /// Tamaño máximo de chunk (se subdivide si excede)
    pub max_chunk_size: usize,
    /// Tamaño mínimo de chunk (se combina con siguiente si es menor)
    pub min_chunk_size: usize,
    /// Caracteres de overlap entre chunks consecutivos
    pub overlap: usize,
    /// Incluir metadata del documento (frontmatter)
    pub include_frontmatter: bool,
    /// Preservar bloques de código como chunks separados
    pub separate_code_blocks: bool,
    /// Prefijo para IDs de chunks
    pub id_prefix: String,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            strategy: ChunkStrategy::default(),
            max_chunk_size: 2000,
            min_chunk_size: 100,
            overlap: 50,
            include_frontmatter: true,
            separate_code_blocks: true,
            id_prefix: "chunk".to_string(),
        }
    }
}

impl ChunkConfig {
    /// Crea nueva configuración con estrategia especificada.
    pub fn new(strategy: ChunkStrategy) -> Self {
        Self {
            strategy,
            ..Default::default()
        }
    }

    /// Establece la estrategia de chunking.
    pub fn with_strategy(mut self, strategy: ChunkStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Establece el tamaño máximo de chunk.
    pub fn with_max_size(mut self, size: usize) -> Self {
        self.max_chunk_size = size;
        self
    }

    /// Establece el tamaño mínimo de chunk.
    pub fn with_min_size(mut self, size: usize) -> Self {
        self.min_chunk_size = size;
        self
    }

    /// Establece el overlap entre chunks.
    pub fn with_overlap(mut self, overlap: usize) -> Self {
        self.overlap = overlap;
        self
    }

    /// Establece el prefijo para IDs.
    pub fn with_id_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.id_prefix = prefix.into();
        self
    }

    /// Habilita/deshabilita separación de bloques de código.
    pub fn with_separate_code_blocks(mut self, separate: bool) -> Self {
        self.separate_code_blocks = separate;
        self
    }
}

/// Tipo de contenido del chunk.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChunkType {
    /// Texto regular (párrafos, listas, etc.)
    Text,
    /// Bloque de código
    Code {
        /// Lenguaje del código (si se especifica)
        language: Option<String>,
    },
    /// Heading/título
    Heading {
        /// Nivel del heading (1-6)
        level: u8,
    },
    /// Tabla
    Table,
    /// Blockquote
    Quote,
    /// Frontmatter (YAML/TOML)
    Frontmatter,
    /// Mixto (contiene varios tipos)
    Mixed,
}

/// Metadata asociada a un chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    /// Heading padre (si existe)
    pub heading: Option<String>,
    /// Nivel del heading padre
    pub heading_level: Option<u8>,
    /// Tipo de contenido
    pub chunk_type: ChunkType,
    /// Posición en el documento original (índice de inicio)
    pub start_position: usize,
    /// Posición final en el documento original
    pub end_position: usize,
    /// Índice del chunk en la secuencia
    pub chunk_index: usize,
    /// Total de chunks del documento
    pub total_chunks: usize,
    /// Nombre del archivo fuente (si se conoce)
    pub source_file: Option<String>,
    /// Metadata adicional del frontmatter
    pub frontmatter: Option<HashMap<String, String>>,
}

impl ChunkMetadata {
    /// Convierte a Metadata de minimemory para almacenamiento.
    pub fn to_metadata(&self) -> Metadata {
        let mut meta = Metadata::new();

        if let Some(ref heading) = self.heading {
            meta.insert("heading", heading.as_str());
        }
        if let Some(level) = self.heading_level {
            meta.insert("heading_level", level as i64);
        }

        let type_str = match &self.chunk_type {
            ChunkType::Text => "text",
            ChunkType::Code { .. } => "code",
            ChunkType::Heading { .. } => "heading",
            ChunkType::Table => "table",
            ChunkType::Quote => "quote",
            ChunkType::Frontmatter => "frontmatter",
            ChunkType::Mixed => "mixed",
        };
        meta.insert("chunk_type", type_str);

        if let ChunkType::Code {
            language: Some(ref lang),
        } = self.chunk_type
        {
            meta.insert("language", lang.as_str());
        }

        meta.insert("start_position", self.start_position as i64);
        meta.insert("end_position", self.end_position as i64);
        meta.insert("chunk_index", self.chunk_index as i64);
        meta.insert("total_chunks", self.total_chunks as i64);

        if let Some(ref source) = self.source_file {
            meta.insert("source_file", source.as_str());
        }

        meta
    }
}

/// Un chunk de contenido listo para vectorización.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    /// ID único del chunk
    pub id: String,
    /// Contenido textual del chunk
    pub content: String,
    /// Metadata estructurada
    pub metadata: ChunkMetadata,
}

impl Chunk {
    /// Crea un nuevo chunk.
    pub fn new(id: String, content: String, metadata: ChunkMetadata) -> Self {
        Self {
            id,
            content,
            metadata,
        }
    }

    /// Retorna el número de caracteres.
    pub fn len(&self) -> usize {
        self.content.len()
    }

    /// Verifica si está vacío.
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// Retorna el número aproximado de tokens (palabras).
    pub fn word_count(&self) -> usize {
        self.content.split_whitespace().count()
    }
}

/// Resultado del chunking con estadísticas.
#[derive(Debug, Clone)]
pub struct ChunkingResult {
    /// Chunks generados
    pub chunks: Vec<Chunk>,
    /// Total de caracteres procesados
    pub total_chars: usize,
    /// Total de chunks generados
    pub total_chunks: usize,
    /// Tamaño promedio de chunk
    pub avg_chunk_size: usize,
}

impl ChunkingResult {
    /// Crea nuevo resultado.
    pub fn new(chunks: Vec<Chunk>) -> Self {
        let total_chars: usize = chunks.iter().map(|c| c.len()).sum();
        let total_chunks = chunks.len();
        let avg_chunk_size = if total_chunks > 0 {
            total_chars / total_chunks
        } else {
            0
        };

        Self {
            chunks,
            total_chars,
            total_chunks,
            avg_chunk_size,
        }
    }
}

// ============================================================================
// Implementación sin mq-markdown (fallback básico)
// ============================================================================

/// Parser de markdown básico (sin dependencia de mq).
/// Usado cuando el feature "chunking" no está habilitado o como fallback.
pub struct BasicMarkdownParser;

impl BasicMarkdownParser {
    /// Extrae chunks usando parsing básico de regex/splits.
    pub fn chunk(content: &str, config: &ChunkConfig) -> Result<ChunkingResult> {
        let chunks = match &config.strategy {
            ChunkStrategy::ByHeading { max_level } => {
                Self::chunk_by_heading(content, *max_level, config)
            }
            ChunkStrategy::BySize {
                target_size,
                overlap,
            } => Self::chunk_by_size(content, *target_size, *overlap, config),
            ChunkStrategy::ByParagraph {
                min_paragraphs,
                max_paragraphs,
            } => Self::chunk_by_paragraph(content, *min_paragraphs, *max_paragraphs, config),
            ChunkStrategy::ByCodeBlocks => Self::chunk_by_code_blocks(content, config),
            ChunkStrategy::Hybrid {
                max_heading_level,
                max_chunk_size,
            } => Self::chunk_hybrid(content, *max_heading_level, *max_chunk_size, config),
        }?;

        Ok(ChunkingResult::new(chunks))
    }

    fn chunk_by_heading(content: &str, max_level: u8, config: &ChunkConfig) -> Result<Vec<Chunk>> {
        let mut chunks = Vec::new();
        let mut current_content = String::new();
        let mut current_heading: Option<String> = None;
        let mut current_level: Option<u8> = None;
        let mut start_pos = 0;
        let mut chunk_index = 0;

        for line in content.lines() {
            if let Some(caps) = HEADING_PATTERN.captures(line) {
                let level = caps.get(1).unwrap().as_str().len() as u8;
                let heading_text = caps.get(2).unwrap().as_str().to_string();

                // Si encontramos un heading del nivel apropiado, crear chunk anterior
                if level <= max_level && !current_content.trim().is_empty() {
                    let chunk = Self::create_chunk(
                        &config.id_prefix,
                        chunk_index,
                        current_content.trim().to_string(),
                        current_heading.clone(),
                        current_level,
                        ChunkType::Text,
                        start_pos,
                        start_pos + current_content.len(),
                    );
                    chunks.push(chunk);
                    chunk_index += 1;
                    start_pos += current_content.len();
                    current_content.clear();
                }

                if level <= max_level {
                    current_heading = Some(heading_text);
                    current_level = Some(level);
                }
            }

            current_content.push_str(line);
            current_content.push('\n');
        }

        // Último chunk
        if !current_content.trim().is_empty() {
            let chunk = Self::create_chunk(
                &config.id_prefix,
                chunk_index,
                current_content.trim().to_string(),
                current_heading,
                current_level,
                ChunkType::Text,
                start_pos,
                content.len(),
            );
            chunks.push(chunk);
        }

        // Actualizar total_chunks en metadata
        let total = chunks.len();
        for chunk in &mut chunks {
            chunk.metadata.total_chunks = total;
        }

        Ok(chunks)
    }

    fn chunk_by_size(
        content: &str,
        target_size: usize,
        overlap: usize,
        config: &ChunkConfig,
    ) -> Result<Vec<Chunk>> {
        let mut chunks = Vec::new();
        let mut start = 0;
        let mut chunk_index = 0;

        while start < content.len() {
            let end = (start + target_size).min(content.len());

            // Buscar un buen punto de corte (fin de párrafo o oración)
            let actual_end = if end < content.len() {
                let slice = &content[start..end];
                if let Some(pos) = slice.rfind("\n\n") {
                    start + pos + 2
                } else if let Some(pos) = slice.rfind(". ") {
                    start + pos + 2
                } else {
                    end
                }
            } else {
                end
            };

            let chunk_content = content[start..actual_end].trim().to_string();

            if !chunk_content.is_empty() {
                let chunk = Self::create_chunk(
                    &config.id_prefix,
                    chunk_index,
                    chunk_content,
                    None,
                    None,
                    ChunkType::Text,
                    start,
                    actual_end,
                );
                chunks.push(chunk);
                chunk_index += 1;
            }

            // Mover con overlap, pero asegurar que siempre avanzamos
            // Si llegamos al final, salir del loop
            if actual_end >= content.len() {
                break;
            }

            // Calcular nuevo start con overlap
            let new_start = if actual_end > overlap {
                actual_end - overlap
            } else {
                actual_end
            };

            // Asegurar que siempre avanzamos al menos 1 posición
            start = new_start.max(start + 1);
        }

        let total = chunks.len();
        for chunk in &mut chunks {
            chunk.metadata.total_chunks = total;
        }

        Ok(chunks)
    }

    fn chunk_by_paragraph(
        content: &str,
        min_paragraphs: usize,
        max_paragraphs: usize,
        config: &ChunkConfig,
    ) -> Result<Vec<Chunk>> {
        let paragraphs: Vec<&str> = content
            .split("\n\n")
            .map(|p| p.trim())
            .filter(|p| !p.is_empty())
            .collect();

        let mut chunks = Vec::new();
        let mut current_paragraphs = Vec::new();
        let mut chunk_index = 0;
        let mut start_pos = 0;

        for para in paragraphs {
            current_paragraphs.push(para);

            if current_paragraphs.len() >= max_paragraphs {
                let chunk_content = current_paragraphs.join("\n\n");
                let end_pos = start_pos + chunk_content.len();

                let chunk = Self::create_chunk(
                    &config.id_prefix,
                    chunk_index,
                    chunk_content,
                    None,
                    None,
                    ChunkType::Text,
                    start_pos,
                    end_pos,
                );
                chunks.push(chunk);
                chunk_index += 1;
                start_pos = end_pos;
                current_paragraphs.clear();
            }
        }

        // Remaining paragraphs
        if current_paragraphs.len() >= min_paragraphs || chunks.is_empty() {
            if !current_paragraphs.is_empty() {
                let chunk_content = current_paragraphs.join("\n\n");
                let end_pos = content.len();

                let chunk = Self::create_chunk(
                    &config.id_prefix,
                    chunk_index,
                    chunk_content,
                    None,
                    None,
                    ChunkType::Text,
                    start_pos,
                    end_pos,
                );
                chunks.push(chunk);
            }
        } else if !current_paragraphs.is_empty() && !chunks.is_empty() {
            // Merge with last chunk
            let last = chunks.last_mut().unwrap();
            last.content.push_str("\n\n");
            last.content.push_str(&current_paragraphs.join("\n\n"));
            last.metadata.end_position = content.len();
        }

        let total = chunks.len();
        for chunk in &mut chunks {
            chunk.metadata.total_chunks = total;
        }

        Ok(chunks)
    }

    fn chunk_by_code_blocks(content: &str, config: &ChunkConfig) -> Result<Vec<Chunk>> {
        let mut chunks = Vec::new();
        let mut chunk_index = 0;
        let mut current_text = String::new();
        let mut in_code_block = false;
        let mut code_content = String::new();
        let mut code_language: Option<String> = None;
        let mut pos = 0;
        let mut text_start = 0;
        let mut code_start = 0;

        for line in content.lines() {
            let line_len = line.len() + 1; // +1 for newline

            if line.starts_with("```") {
                if in_code_block {
                    // End of code block
                    in_code_block = false;
                    code_content.push_str(line);

                    let chunk = Self::create_chunk(
                        &config.id_prefix,
                        chunk_index,
                        code_content.clone(),
                        None,
                        None,
                        ChunkType::Code {
                            language: code_language.clone(),
                        },
                        code_start,
                        pos + line_len,
                    );
                    chunks.push(chunk);
                    chunk_index += 1;
                    code_content.clear();
                    code_language = None;
                    text_start = pos + line_len;
                } else {
                    // Start of code block
                    // First, save accumulated text
                    if !current_text.trim().is_empty() {
                        let chunk = Self::create_chunk(
                            &config.id_prefix,
                            chunk_index,
                            current_text.trim().to_string(),
                            None,
                            None,
                            ChunkType::Text,
                            text_start,
                            pos,
                        );
                        chunks.push(chunk);
                        chunk_index += 1;
                        current_text.clear();
                    }

                    in_code_block = true;
                    code_start = pos;
                    code_language = if line.len() > 3 {
                        Some(line[3..].trim().to_string())
                    } else {
                        None
                    };
                    code_content.push_str(line);
                    code_content.push('\n');
                }
            } else if in_code_block {
                code_content.push_str(line);
                code_content.push('\n');
            } else {
                current_text.push_str(line);
                current_text.push('\n');
            }

            pos += line_len;
        }

        // Remaining text
        if !current_text.trim().is_empty() {
            let chunk = Self::create_chunk(
                &config.id_prefix,
                chunk_index,
                current_text.trim().to_string(),
                None,
                None,
                ChunkType::Text,
                text_start,
                content.len(),
            );
            chunks.push(chunk);
        }

        let total = chunks.len();
        for chunk in &mut chunks {
            chunk.metadata.total_chunks = total;
        }

        Ok(chunks)
    }

    fn chunk_hybrid(
        content: &str,
        max_heading_level: u8,
        max_chunk_size: usize,
        config: &ChunkConfig,
    ) -> Result<Vec<Chunk>> {
        // First, chunk by headings
        let heading_chunks = Self::chunk_by_heading(content, max_heading_level, config)?;

        let mut final_chunks = Vec::new();
        let mut new_index = 0;

        for chunk in heading_chunks {
            if chunk.len() > max_chunk_size {
                // Subdivide large chunks
                let sub_config = ChunkConfig {
                    strategy: ChunkStrategy::BySize {
                        target_size: max_chunk_size,
                        overlap: config.overlap,
                    },
                    ..config.clone()
                };
                let sub_result = Self::chunk_by_size(
                    &chunk.content,
                    max_chunk_size,
                    config.overlap,
                    &sub_config,
                )?;

                for mut sub_chunk in sub_result {
                    sub_chunk.id = format!("{}-{}", config.id_prefix, new_index);
                    sub_chunk.metadata.chunk_index = new_index;
                    sub_chunk.metadata.heading = chunk.metadata.heading.clone();
                    sub_chunk.metadata.heading_level = chunk.metadata.heading_level;
                    final_chunks.push(sub_chunk);
                    new_index += 1;
                }
            } else {
                let mut new_chunk = chunk;
                new_chunk.id = format!("{}-{}", config.id_prefix, new_index);
                new_chunk.metadata.chunk_index = new_index;
                final_chunks.push(new_chunk);
                new_index += 1;
            }
        }

        let total = final_chunks.len();
        for chunk in &mut final_chunks {
            chunk.metadata.total_chunks = total;
        }

        Ok(final_chunks)
    }

    fn create_chunk(
        prefix: &str,
        index: usize,
        content: String,
        heading: Option<String>,
        heading_level: Option<u8>,
        chunk_type: ChunkType,
        start_pos: usize,
        end_pos: usize,
    ) -> Chunk {
        Chunk {
            id: format!("{}-{}", prefix, index),
            content,
            metadata: ChunkMetadata {
                heading,
                heading_level,
                chunk_type,
                start_position: start_pos,
                end_position: end_pos,
                chunk_index: index,
                total_chunks: 0, // Se actualiza después
                source_file: None,
                frontmatter: None,
            },
        }
    }
}

// ============================================================================
// API pública
// ============================================================================

/// Procesa un documento Markdown y retorna chunks listos para vectorización.
///
/// # Argumentos
///
/// * `content` - Contenido Markdown a procesar
/// * `config` - Configuración de chunking
///
/// # Ejemplo
///
/// ```rust,ignore
/// use minimemory::chunking::{chunk_markdown, ChunkConfig, ChunkStrategy};
///
/// let md = "# Title\nSome content\n\n## Section\nMore content";
/// let config = ChunkConfig::new(ChunkStrategy::ByHeading { max_level: 2 });
/// let result = chunk_markdown(md, &config).unwrap();
///
/// println!("Generated {} chunks", result.total_chunks);
/// ```
pub fn chunk_markdown(content: &str, config: &ChunkConfig) -> Result<ChunkingResult> {
    BasicMarkdownParser::chunk(content, config)
}

/// Procesa un archivo Markdown y retorna chunks.
///
/// Similar a `chunk_markdown` pero lee desde archivo y agrega
/// el nombre del archivo a la metadata.
pub fn chunk_markdown_file(path: &std::path::Path, config: &ChunkConfig) -> Result<ChunkingResult> {
    let content = std::fs::read_to_string(path).map_err(Error::Io)?;

    let mut result = chunk_markdown(&content, config)?;

    // Agregar nombre de archivo a metadata
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string());

    for chunk in &mut result.chunks {
        chunk.metadata.source_file = filename.clone();
    }

    Ok(result)
}

/// Procesa múltiples archivos Markdown en paralelo.
pub fn chunk_markdown_files(
    paths: &[std::path::PathBuf],
    config: &ChunkConfig,
) -> Result<Vec<ChunkingResult>> {
    use rayon::prelude::*;

    paths
        .par_iter()
        .map(|path| chunk_markdown_file(path, config))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_by_heading() {
        let content = r#"# Main Title
Introduction paragraph.

## Section 1
Content of section 1.
More content here.

## Section 2
Content of section 2.

### Subsection 2.1
Subsection content.
"#;

        let config = ChunkConfig::new(ChunkStrategy::ByHeading { max_level: 2 });
        let result = chunk_markdown(content, &config).unwrap();

        assert!(result.total_chunks >= 2);
        assert!(
            result.chunks[0].content.contains("Main Title")
                || result.chunks[0].content.contains("Introduction")
        );
    }

    #[test]
    fn test_chunk_by_size() {
        let content = "A".repeat(1000) + " " + &"B".repeat(1000);

        let config = ChunkConfig::new(ChunkStrategy::BySize {
            target_size: 500,
            overlap: 50,
        });
        let result = chunk_markdown(&content, &config).unwrap();

        assert!(result.total_chunks >= 2);
    }

    #[test]
    fn test_chunk_by_code_blocks() {
        let content = r#"
Some text here.

```rust
fn main() {
    println!("Hello");
}
```

More text.

```python
print("Hello")
```

Final text.
"#;

        let config = ChunkConfig::new(ChunkStrategy::ByCodeBlocks);
        let result = chunk_markdown(content, &config).unwrap();

        // Should have text chunks and code chunks
        let code_chunks: Vec<_> = result
            .chunks
            .iter()
            .filter(|c| matches!(c.metadata.chunk_type, ChunkType::Code { .. }))
            .collect();

        assert_eq!(code_chunks.len(), 2);

        // Verify language detection
        if let ChunkType::Code { language } = &code_chunks[0].metadata.chunk_type {
            assert_eq!(language.as_deref(), Some("rust"));
        }
    }

    #[test]
    fn test_chunk_metadata_conversion() {
        let metadata = ChunkMetadata {
            heading: Some("Test Section".to_string()),
            heading_level: Some(2),
            chunk_type: ChunkType::Code {
                language: Some("rust".to_string()),
            },
            start_position: 0,
            end_position: 100,
            chunk_index: 0,
            total_chunks: 5,
            source_file: Some("test.md".to_string()),
            frontmatter: None,
        };

        let meta = metadata.to_metadata();

        assert!(meta.get("heading").is_some());
        assert!(meta.get("chunk_type").is_some());
        assert!(meta.get("language").is_some());
    }

    #[test]
    fn test_hybrid_strategy() {
        let content = r#"# Title

Short intro.

## Long Section

"#
        .to_owned()
            + &"Long content. ".repeat(500)
            + r#"

## Short Section

Brief content.
"#;

        let config = ChunkConfig::new(ChunkStrategy::Hybrid {
            max_heading_level: 2,
            max_chunk_size: 500,
        });
        let result = chunk_markdown(&content, &config).unwrap();

        // Long section should be split into multiple chunks
        assert!(result.total_chunks >= 3);
    }
}
