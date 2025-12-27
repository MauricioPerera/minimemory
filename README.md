# minimemory

Base de datos híbrida embebida para Rust. Como SQLite para documentos + búsqueda vectorial + full-text search.

## Características

- **Sin servidor**: Librería embebida, solo importar y usar
- **Ligera**: Sin dependencias pesadas
- **Rápida**: Optimizada para alto rendimiento
- **Híbrida**: Combina vectores, BM25 y filtros de metadata
- **Flexible**: Múltiples métricas de distancia (Cosine, Euclidean, DotProduct)
- **RAG-ready**: Integración con mq para chunking inteligente de Markdown
- **Type-safe**: API idiomática de Rust

## Instalación

Agrega a tu `Cargo.toml`:

```toml
[dependencies]
minimemory = { git = "https://github.com/MauricioPerera/minimemory" }

# Con soporte para chunking de Markdown (integración mq)
minimemory = { git = "https://github.com/MauricioPerera/minimemory", features = ["chunking"] }
```

## Uso Rápido

```rust
use minimemory::{VectorDB, Config, Distance, IndexType};

fn main() -> minimemory::Result<()> {
    // Crear base de datos en memoria
    let config = Config::new(384)  // 384 dimensiones
        .with_distance(Distance::Cosine)
        .with_index(IndexType::Flat);

    let db = VectorDB::new(config)?;

    // Insertar vectores
    db.insert("doc-1", &vec![0.1; 384], None)?;
    db.insert("doc-2", &vec![0.2; 384], None)?;

    // Buscar los 5 más similares
    let query = vec![0.15; 384];
    let results = db.search(&query, 5)?;

    for result in results {
        println!("ID: {}, Distancia: {:.4}", result.id, result.distance);
    }

    Ok(())
}
```

## Búsqueda Híbrida

minimemory soporta tres tipos de búsqueda que pueden combinarse:

### 1. Búsqueda Vectorial (Semántica)

```rust
let results = db.search(&query_vector, 10)?;
```

### 2. Búsqueda por Keywords (BM25)

```rust
use minimemory::{VectorDB, Config, Metadata};

// Crear DB con full-text search habilitado
let db = VectorDB::with_fulltext(
    Config::new(384),
    vec!["title".into(), "content".into()]  // Campos a indexar
)?;

// Insertar documentos
let mut meta = Metadata::new();
meta.insert("title", "Guía de Rust");
meta.insert("content", "Aprende programación en Rust...");
db.insert_document("doc-1", Some(&embedding), Some(meta))?;

// Buscar por keywords
let results = db.keyword_search("rust programación", 10)?;
```

### 3. Búsqueda por Filtros de Metadata

```rust
use minimemory::Filter;

// Filtros básicos
let results = db.filter_search(
    Filter::eq("category", "tech"),
    100
)?;

// Filtros combinados
let results = db.filter_search(
    Filter::eq("category", "tech")
        .and(Filter::gt("score", 0.5f64))
        .or(Filter::eq("featured", true)),
    100
)?;
```

### 4. Búsqueda Híbrida Completa

```rust
use minimemory::HybridSearchParams;

// Combinar vector + keywords + filtros
let params = HybridSearchParams::hybrid(
    query_embedding,        // Vector de consulta
    "rust programming",     // Keywords
    10                      // Top K
)
.with_filter(Filter::eq("category", "tech"))
.with_vector_weight(0.7)   // 70% vector, 30% keywords
.with_fusion_k(60);        // Parámetro RRF

let results = db.hybrid_search(params)?;
```

## Filtros de Metadata

| Operador | Ejemplo | Descripción |
|----------|---------|-------------|
| `eq` | `Filter::eq("status", "active")` | Igual a |
| `ne` | `Filter::ne("status", "deleted")` | Diferente de |
| `gt` | `Filter::gt("score", 0.5f64)` | Mayor que |
| `gte` | `Filter::gte("count", 10i64)` | Mayor o igual |
| `lt` | `Filter::lt("price", 100.0f64)` | Menor que |
| `lte` | `Filter::lte("age", 30i64)` | Menor o igual |
| `contains` | `Filter::contains("tags", "rust")` | Contiene valor |
| `starts_with` | `Filter::starts_with("title", "How")` | Empieza con |
| `and` | `f1.and(f2)` | AND lógico |
| `or` | `f1.or(f2)` | OR lógico |

```rust
// Acceso a campos anidados (dot notation)
Filter::eq("author.name", "Juan");
Filter::gt("metadata.views", 1000i64);
```

## Chunking de Markdown (Integración mq)

minimemory integra [mq](https://github.com/harehare/mq) para procesamiento inteligente de Markdown, ideal para pipelines RAG.

### Habilitando el Feature

```toml
[dependencies]
minimemory = { git = "...", features = ["chunking"] }
```

### Estrategias de Chunking

```rust
use minimemory::chunking::{ChunkConfig, ChunkStrategy, chunk_markdown};

// Por headings (H1, H2)
let config = ChunkConfig::new(ChunkStrategy::ByHeading { max_level: 2 });

// Por tamaño con overlap
let config = ChunkConfig::new(ChunkStrategy::BySize {
    target_size: 1000,
    overlap: 100,
});

// Por párrafos
let config = ChunkConfig::new(ChunkStrategy::ByParagraph {
    min_paragraphs: 2,
    max_paragraphs: 5,
});

// Por bloques de código
let config = ChunkConfig::new(ChunkStrategy::ByCodeBlocks);

// Híbrido: headings + tamaño máximo
let config = ChunkConfig::new(ChunkStrategy::Hybrid {
    max_heading_level: 2,
    max_chunk_size: 1000,
});
```

### Procesando Markdown

```rust
use minimemory::chunking::{chunk_markdown, ChunkConfig};

let markdown = r#"
# Introducción
Contenido de la introducción...

## Sección 1
Más contenido aquí.

```rust
fn main() {
    println!("Hello!");
}
```

## Sección 2
Contenido final.
"#;

let config = ChunkConfig::default()
    .with_max_size(1000)
    .with_overlap(50);

let result = chunk_markdown(markdown, &config)?;

for chunk in result.chunks {
    println!("ID: {}", chunk.id);
    println!("Contenido: {} chars", chunk.content.len());
    println!("Heading: {:?}", chunk.metadata.heading);
    println!("Tipo: {:?}", chunk.metadata.chunk_type);
}
```

### Pipeline RAG Completo

```rust
use minimemory::{VectorDB, Config};
use minimemory::chunking::{ChunkConfig, ChunkStrategy};

// 1. Crear DB con full-text search
let db = VectorDB::with_fulltext(
    Config::new(384),
    vec!["content".into(), "heading".into()]
)?;

// 2. Configurar chunking
let chunk_config = ChunkConfig::new(ChunkStrategy::Hybrid {
    max_heading_level: 2,
    max_chunk_size: 1000,
}).with_overlap(100);

// Opción A: Solo keyword search (sin embeddings)
let count = db.ingest_markdown(markdown_content, &chunk_config)?;
println!("Ingested {} chunks", count);

// Opción B: Con embeddings (RAG semántico completo)
let result = chunk_markdown(markdown_content, &chunk_config)?;
for chunk in &result.chunks {
    let embedding = your_embedding_api(&chunk.content)?; // OpenAI, Ollama, etc.
    db.insert_chunk(chunk, Some(&embedding))?;
}

// 3. Buscar
let results = db.hybrid_search(
    HybridSearchParams::hybrid(query_embedding, "search terms", 10)
)?;
```

### Metadata de Chunks

Cada chunk incluye metadata estructurada:

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `heading` | `Option<String>` | Heading padre del chunk |
| `heading_level` | `Option<u8>` | Nivel del heading (1-6) |
| `chunk_type` | `ChunkType` | Text, Code, Heading, Table, Quote |
| `start_position` | `usize` | Posición inicial en documento |
| `end_position` | `usize` | Posición final |
| `chunk_index` | `usize` | Índice del chunk |
| `total_chunks` | `usize` | Total de chunks generados |
| `source_file` | `Option<String>` | Archivo fuente |
| `language` | `Option<String>` | Lenguaje (para bloques de código) |

## Documentos sin Vector

Puedes almacenar documentos solo con metadata (como MongoDB):

```rust
let mut meta = Metadata::new();
meta.insert("title", "Mi Artículo");
meta.insert("content", "Texto completo del artículo...");
meta.insert("author", "Juan");

// Insertar SIN vector
db.insert_document("article-1", None, Some(meta))?;

// Buscar por keywords (BM25)
let results = db.keyword_search("artículo", 10)?;

// O por filtros
let results = db.filter_search(Filter::eq("author", "Juan"), 10)?;
```

## API Completa

### Configuración

```rust
use minimemory::{Config, Distance, IndexType};

let config = Config::new(384)
    .with_distance(Distance::Cosine)      // Métrica de distancia
    .with_index(IndexType::HNSW {         // Índice aproximado
        m: 16,
        ef_construction: 200,
    });
```

### Métricas de Distancia

| Métrica | Descripción | Uso típico |
|---------|-------------|------------|
| `Distance::Cosine` | Similitud coseno (1 - cos_sim) | Embeddings de texto |
| `Distance::Euclidean` | Distancia L2 | Vectores normalizados |
| `Distance::DotProduct` | Producto punto negativo | Cuando la magnitud importa |

### Tipos de Índice

| Tipo | Descripción | Complejidad |
|------|-------------|-------------|
| `IndexType::Flat` | Búsqueda exacta (brute-force) | O(n) |
| `IndexType::HNSW { m, ef_construction }` | Búsqueda aproximada rápida | O(log n) |

### Operaciones CRUD

```rust
// Insertar con vector
db.insert("id", &vector, Some(metadata))?;

// Insertar documento (vector opcional)
db.insert_document("id", Some(&vector), Some(metadata))?;
db.insert_document("id", None, Some(metadata))?;  // Solo metadata

// Insertar chunk
db.insert_chunk(&chunk, Some(&embedding))?;

// Obtener
if let Some((vector, metadata)) = db.get("id")? {
    println!("Vector: {:?}, Meta: {:?}", vector, metadata);
}

// Actualizar
db.update("id", &new_vector, new_metadata)?;
db.update_document("id", Some(&vector), Some(metadata))?;

// Eliminar
db.delete("id")?;

// Verificar existencia
if db.contains("id") { ... }

// Estadísticas
println!("Count: {}", db.len());
println!("Has fulltext: {}", db.has_fulltext());
```

### Persistencia

```rust
// Guardar a disco
db.save("my_database.mmdb")?;

// Cargar desde disco
let db = VectorDB::open("my_database.mmdb")?;

// Con full-text search
let db = VectorDB::open_with_fulltext(
    "my_database.mmdb",
    vec!["title".into(), "content".into()]
)?;
```

## Arquitectura

```
minimemory
├── VectorDB              # Interfaz principal
├── Storage               # Capa de almacenamiento
│   └── MemoryStorage     # HashMap thread-safe
├── Index                 # Indexación vectorial
│   ├── FlatIndex         # Búsqueda exacta O(n)
│   └── HNSWIndex         # Búsqueda aproximada O(log n)
├── BM25Index             # Full-text search
├── Query                 # Sistema de filtros
│   └── Filter            # Operadores de filtrado
├── Search                # Búsqueda híbrida
│   └── HybridSearch      # RRF fusion de resultados
├── Chunking              # Procesamiento de Markdown
│   ├── ChunkStrategy     # Estrategias de división
│   └── Chunk             # Unidad de contenido
└── Distance              # Métricas
    ├── Cosine
    ├── Euclidean
    └── DotProduct
```

## Rendimiento

| Operación | Flat | HNSW |
|-----------|------|------|
| Insertar | O(1) | O(log n) |
| Buscar | O(n × d) | O(log n × d) |
| Keyword Search | O(n) | O(n) |
| Filtrar | O(n) | O(n) |

*n = número de documentos, d = dimensiones*

## Bindings

### Python

```bash
pip install maturin
maturin develop --features python
```

```python
from minimemory import VectorDB

db = VectorDB(dimensions=384, distance="cosine", index_type="hnsw")
db.insert("doc-1", [0.1] * 384, {"title": "Test"})
results = db.search([0.1] * 384, k=10)
```

### Node.js

```bash
npm install -g @napi-rs/cli
npm run build
```

```javascript
const { VectorDB } = require('minimemory');

const db = new VectorDB({ dimensions: 384, distance: 'cosine' });
db.insert('doc-1', new Array(384).fill(0.1), { title: 'Test' });
const results = db.search(new Array(384).fill(0.1), 10);
```

## Roadmap

- [x] Storage en memoria
- [x] Índice Flat (búsqueda exacta)
- [x] Índice HNSW (búsqueda aproximada)
- [x] Métricas: Cosine, Euclidean, DotProduct
- [x] Metadata en vectores
- [x] Persistencia en disco (.mmdb)
- [x] Bindings Python/Node.js/PHP
- [x] **Documentos sin vector (metadata-only)**
- [x] **BM25 full-text search**
- [x] **Filtros de metadata con operadores**
- [x] **Búsqueda híbrida (RRF fusion)**
- [x] **Integración mq para chunking**
- [ ] Quantización de vectores
- [ ] Índices parciales
- [ ] Replicación

## Licencia

MIT
