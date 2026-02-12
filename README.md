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
// Guardar a disco (escritura atómica con CRC32)
db.save("my_database.mmdb")?;

// Cargar desde disco (verifica CRC32)
let db = VectorDB::open("my_database.mmdb")?;

// Con full-text search
let db = VectorDB::open_with_fulltext(
    "my_database.mmdb",
    vec!["title".into(), "content".into()]
)?;
```

El formato `.mmdb` v2 incluye:
- **Vectores + metadata** serializados con bincode
- **HNSW index** persistido (tagged block "HNSW") - no necesita reconstruir el grafo al cargar
- **BM25 index** persistido (tagged block "BM25") - keywords listos inmediatamente
- **CRC32 checksum** para detectar corrupción
- **Escritura atómica** (escribe a `.tmp`, luego renombra) para crash safety
- Compatible con archivos v1 (lectura backward-compatible)

## Arquitectura

```
minimemory
├── VectorDB              # Interfaz principal
├── Storage               # Capa de almacenamiento
│   ├── MemoryStorage     # HashMap thread-safe
│   └── DiskStorage       # .mmdb con CRC32 + escritura atómica
├── Index                 # Indexación vectorial
│   ├── FlatIndex         # Búsqueda exacta O(n)
│   └── HNSWIndex         # Búsqueda aproximada O(log n) (persistido)
├── BM25Index             # Full-text search (persistido)
├── Query                 # Sistema de filtros
│   └── Filter            # Operadores de filtrado
├── Search                # Búsqueda híbrida
│   └── HybridSearch      # RRF fusion de resultados
├── Chunking              # Procesamiento de Markdown
│   ├── ChunkStrategy     # Estrategias de división
│   └── Chunk             # Unidad de contenido
├── Quantization          # Compresión de vectores
│   ├── Quantizer         # Motor de quantización
│   ├── Int8              # Scalar 4x compression
│   └── Binary            # 32x compression
├── PartialIndex          # Índices sobre subconjuntos
│   ├── PartialIndexConfig # Configuración de filtros
│   └── PartialIndexManager # Gestor de índices
├── Replication           # Sincronización entre instancias
│   ├── ChangeLog         # Registro de operaciones
│   └── ReplicationManager # Gestor de sync
├── MemoryTraits          # Sistema domain-agnostic (core)
│   ├── GenericMemory<P>  # Memoria genérica parametrizada
│   ├── DomainPreset      # Configuración de dominio
│   ├── TransferLevel     # Niveles de transferibilidad (unificado)
│   ├── Priority          # Sistema de prioridad híbrida
│   ├── DecayConfig       # Decay temporal exponencial
│   ├── UsageStats        # Estadísticas persistentes de uso
│   └── Presets           # Presets predefinidos
│       ├── SoftwareDevelopment
│       ├── Conversational
│       └── CustomerService
├── AgentMemory           # Facade sobre GenericMemory<SoftwareDev>
│   ├── WorkingContext    # Contexto actual (persistido)
│   ├── TaskEpisode       # Experiencias de tareas
│   └── CodeSnippet       # Código aprendido
├── Transfer              # Transferencia de conocimiento
│   ├── TransferableMemory # Wrapper sobre AgentMemory
│   ├── KnowledgeDomain   # Dominios de conocimiento
│   └── ProjectContext    # Contexto de proyecto
└── Distance              # Métricas
    ├── Cosine
    ├── Euclidean
    └── DotProduct
```

## Quantización de Vectores

minimemory soporta quantización para reducir el uso de memoria manteniendo calidad de búsqueda.

### Tipos de Quantización

| Tipo | Compresión | Precisión | Uso |
|------|------------|-----------|-----|
| `None` | 1x (4 bytes/dim) | 100% | Default, máxima precisión |
| `Int8` | 4x (1 byte/dim) | ~99% | Balance memoria/precisión |
| `Binary` | 32x (1 bit/dim) | ~90-95% | Máxima compresión |

### Configuración

```rust
use minimemory::{Config, Distance, QuantizationType};

// Sin quantización (default)
let config = Config::new(384);

// Quantización Int8 (recomendada para embeddings)
let config = Config::new(1536)
    .with_quantization(QuantizationType::Int8);

// Quantización binaria (para embeddings muy grandes)
let config = Config::new(4096)
    .with_quantization(QuantizationType::Binary);
```

### Uso Manual del Quantizador

```rust
use minimemory::quantization::{Quantizer, QuantizationType};

// Crear quantizador entrenado con samples
let samples: Vec<Vec<f32>> = vec![/* tus vectores */];
let sample_refs: Vec<&[f32]> = samples.iter().map(|v| v.as_slice()).collect();

let quantizer = Quantizer::int8_trained(384, &sample_refs);

// Quantizar un vector
let vector = vec![0.1; 384];
let quantized = quantizer.quantize(&vector).unwrap();

// Ver ahorro de memoria
println!("Original: {} bytes", 384 * 4);
println!("Quantizado: {} bytes", quantized.memory_bytes());

// Dequantizar si es necesario
let restored = quantized.to_f32();
```

### Distancias con Vectores Quantizados

```rust
use minimemory::{Distance, quantization::quantized_distance};

let dist = quantized_distance(&quant_a, &quant_b, Distance::Cosine).unwrap();
```

## Índices Parciales

Los índices parciales permiten crear índices sobre subconjuntos de documentos, mejorando el rendimiento de consultas frecuentes sobre categorías específicas.

### Beneficios

- **Menor uso de memoria**: Solo indexa documentos relevantes
- **Búsquedas más rápidas**: Índices más pequeños = menos comparaciones
- **Especialización**: Índices optimizados para patrones de consulta específicos

### Creación de Índices Parciales

```rust
use minimemory::{VectorDB, Config, Filter};
use minimemory::partial_index::PartialIndexConfig;

let db = VectorDB::new(Config::new(384)).unwrap();

// Crear índice parcial para documentos de categoría "tech"
db.create_partial_index(
    "tech_docs",
    PartialIndexConfig::new(Filter::eq("category", "tech"))
).unwrap();

// Crear índice HNSW para documentos activos con score alto
db.create_partial_index(
    "premium_docs",
    PartialIndexConfig::new(
        Filter::eq("status", "active")
            .and(Filter::gt("score", 0.8f64))
    ).with_hnsw(16, 200)
).unwrap();
```

### Búsqueda en Índices Parciales

```rust
// Buscar solo en documentos de tecnología (más rápido que buscar en todo)
let results = db.search_partial("tech_docs", &query_vector, 10).unwrap();

for result in results {
    println!("{}: {:.4}", result.id, result.distance);
}
```

### Gestión de Índices

```rust
// Listar todos los índices parciales
let indexes = db.list_partial_indexes();
for idx in indexes {
    println!("{}: {} documentos", idx.name, idx.document_count);
}

// Reconstruir índice después de cambios masivos
let count = db.rebuild_partial_index("tech_docs").unwrap();
println!("Índice reconstruido con {} documentos", count);

// Eliminar índice
db.drop_partial_index("tech_docs").unwrap();
```

### Sincronización Automática

Los índices parciales se actualizan automáticamente cuando:
- Se insertan nuevos documentos que cumplen el filtro
- Se eliminan documentos del índice principal

## Replicación

minimemory soporta replicación para sincronizar datos entre múltiples instancias.

### Change Log

El Change Log registra todas las operaciones para permitir replicación incremental.

```rust
use minimemory::replication::{ChangeLog, ReplicationManager};

// Crear log de cambios
let log = ChangeLog::new();

// Registrar operaciones
log.track_insert("doc-1", &vec![0.1; 384], None);
log.track_update("doc-1", &vec![0.2; 384], None);
log.track_delete("doc-2");

// Exportar cambios desde checkpoint
let checkpoint = log.checkpoint();
// ... más operaciones ...
let changes = log.export_since_checkpoint();
```

### Snapshot y Restauración

```rust
use minimemory::{VectorDB, Config};
use minimemory::replication::ReplicationManager;

// Crear snapshot de la DB primaria
let primary = VectorDB::new(Config::new(384)).unwrap();
primary.insert("doc-1", &vec![0.1; 384], None).unwrap();

let snapshot = ReplicationManager::create_snapshot(&primary).unwrap();

// Restaurar en réplica
let replica = VectorDB::new(Config::new(384)).unwrap();
let count = ReplicationManager::apply_snapshot(&replica, &snapshot).unwrap();
println!("Restaurados {} documentos", count);
```

### Sincronización Incremental

```rust
use minimemory::replication::{ChangeLog, ReplicationManager};

// En instancia primaria
let log = ChangeLog::with_instance_id("primary");

// Registrar cambios
log.track_insert("doc-1", &vec![0.1; 384], None);
primary.insert("doc-1", &vec![0.1; 384], None).unwrap();

// Exportar cambios
let changes = log.export_since(0);

// En réplica: aplicar cambios
let result = ReplicationManager::apply_changes(&replica, &changes).unwrap();
println!("Aplicados: {}, Omitidos: {}", result.applied, result.skipped);
```

### Resolución de Conflictos

```rust
use minimemory::replication::{ReplicationManager, ConflictResolution};

// Estrategias disponibles:
// - KeepLocal: mantener versión local
// - ApplyRemote: aplicar versión remota
// - LastWriteWins: gana el timestamp más reciente (default)

let manager = ReplicationManager::new()
    .with_conflict_strategy(ConflictResolution::LastWriteWins);
```

## Memoria Agéntica

Sistema de memoria diseñado para agentes de IA que desarrollan código.

### Tipos de Memoria

| Tipo | Uso |
|------|-----|
| **Episódica** | Experiencias de tareas (éxitos/fallos) |
| **Semántica** | Conocimiento de APIs, patrones, código |
| **Working** | Contexto actual (proyecto, goals) |

### Inicialización

```rust
use minimemory::agent_memory::{AgentMemory, MemoryConfig, TaskOutcome};

// Crear memoria (1536 dims para OpenAI embeddings)
let mut memory = AgentMemory::new(MemoryConfig::openai()).unwrap();

// Para modelos pequeños (384 dims)
let memory = AgentMemory::new(MemoryConfig::small()).unwrap();

// Establecer función de embedding
memory.set_embed_fn(|text| {
    // Tu implementación de embedding aquí
    openai_embed(text)
});
```

### Aprender (Learning)

```rust
use minimemory::agent_memory::{TaskEpisode, CodeSnippet, ErrorSolution, Language, TaskOutcome};

// Aprender de una tarea completada
memory.learn_task(
    "Implementar autenticación JWT",
    "fn verify_token(token: &str) -> Result<Claims> { ... }",
    TaskOutcome::Success,
    vec!["Usar jsonwebtoken crate", "Validar expiration"]
).unwrap();

// Aprender snippet de código
memory.learn_code(CodeSnippet {
    code: "async fn fetch_data() { ... }".to_string(),
    description: "Fetch async con retry".to_string(),
    language: Language::Rust,
    dependencies: vec!["reqwest".into()],
    use_case: "HTTP requests con reintentos".to_string(),
    quality_score: 0.95,
    tags: vec!["async".into(), "http".into()],
}).unwrap();

// Aprender solución a un error
memory.learn_error_solution(ErrorSolution {
    error_message: "cannot borrow as mutable".to_string(),
    error_type: "E0596".to_string(),
    root_cause: "Missing mut keyword".to_string(),
    solution: "Add mut to variable declaration".to_string(),
    fixed_code: Some("let mut x = 5;".to_string()),
    language: Language::Rust,
}).unwrap();
```

### Recordar (Recall)

```rust
// Buscar experiencias similares
let experiences = memory.recall_similar("autenticación de usuarios", 5).unwrap();

// Buscar código similar
let snippets = memory.recall_code("HTTP client async", 5).unwrap();

// Buscar soluciones a errores
let solutions = memory.recall_error_solutions("borrow checker error", 3).unwrap();

// Buscar solo experiencias exitosas
let successes = memory.recall_successful("database migration", 5).unwrap();

// Buscar en el proyecto actual (usa índice parcial)
memory.focus_project("my-project").unwrap();
let project_results = memory.recall_in_project("authentication", 10).unwrap();
```

### Working Memory (Contexto Actual)

```rust
// Establecer contexto de trabajo
memory.with_working_context(|ctx| {
    ctx.set_project("my-app");
    ctx.set_task("Implementar login");
    ctx.add_goal("Escribir tests");
    ctx.add_goal("Documentar API");
    ctx.add_open_file("src/auth.rs");
    ctx.add_conversation("user", "Necesito login con OAuth");
});

// Leer contexto
let ctx = memory.working_context();
println!("Proyecto: {:?}", ctx.current_project);
println!("Goals: {:?}", ctx.active_goals);
```

### Persistencia

```rust
// Guardar memoria
memory.save("agent_memory.mmdb").unwrap();

// Cargar memoria
let memory = AgentMemory::load("agent_memory.mmdb", MemoryConfig::openai()).unwrap();
```

### Estadísticas

```rust
let stats = memory.stats().unwrap();
println!("Total: {} entradas", stats.total_entries);
println!("Episodios: {}", stats.episodes);
println!("Snippets: {}", stats.code_snippets);
println!("Soluciones: {}", stats.error_solutions);
```

## Memoria Genérica (Domain-Agnostic)

Sistema de memoria extensible que funciona para cualquier dominio, no solo desarrollo de software.

### Conceptos Clave

| Concepto | Descripción |
|----------|-------------|
| **DomainPreset** | Configuración completa para un dominio (traits + decay + weights) |
| **TransferLevel** | Nivel de transferibilidad: Instance → Context → Domain → Universal |
| **Priority** | Prioridad híbrida: Low, Normal, High, Critical |
| **DecayConfig** | Configuración de decay temporal exponencial |

### Presets Incluidos

| Preset | Uso | Decay | Prioridad |
|--------|-----|-------|-----------|
| `SoftwareDevelopment` | Agentes de código | Lento (90 días) | Por utilidad |
| `Conversational` | Chatbots | Rápido (7 días) | Por recencia |
| `CustomerService` | Atención al cliente | Normal (30 días) | Manual |

### Uso Básico

```rust
use minimemory::memory_traits::{GenericMemory, InstanceContext};
use minimemory::memory_traits::presets::SoftwareDevelopment;

// Crear memoria para desarrollo de software
let memory = GenericMemory::<SoftwareDevelopment>::new(384)?;

// Establecer contexto
memory.set_instance("my-project", "rust", "backend");

// Aprender (prioridad automática basada en contenido)
memory.learn(
    "fix-auth-bug",
    &embedding,
    "Fixed JWT validation bug",
    "Security fix for token expiration",
    "success"
)?;

// Recall con scoring híbrido
let results = memory.recall(&query_embedding, 5)?;
for r in results {
    println!("{}: relevance={:.2}, priority={:?}, transfer={:?}",
        r.id, r.relevance, r.priority, r.transfer_level);
}
```

### Sistema de Prioridad Híbrida

La prioridad se calcula combinando múltiples factores:

```
priority_score = (base × 0.4) + (frequency × 0.2) + (usefulness × 0.25) + (recency × 0.15) × decay
```

| Factor | Descripción |
|--------|-------------|
| **Base** | Prioridad manual o detectada por keywords (0.25 - 1.0) |
| **Frequency** | log2(accesos + 1) / 10, capped at 1.0 |
| **Usefulness** | útiles / accesos (ratio de feedback positivo) |
| **Recency** | e^(-hours/168), decae con el tiempo |
| **Decay** | 0.5^(age/half_life), exponencial configurable |

```rust
use minimemory::memory_traits::{Priority, DecayConfig, PriorityWeights};

// Prioridad manual
memory.learn_with_priority(
    "critical-fix",
    &embedding,
    "Security patch",
    "CVE-2024-1234 fix",
    "deployed",
    Priority::Critical
)?;

// Feedback positivo (aumenta usefulness)
memory.mark_useful("fix-auth-bug");

// Actualizar prioridad
memory.update_priority("old-task", Priority::Low)?;

// Configurar decay
memory.set_decay_config(DecayConfig::fast()); // 7 días half-life

// Configurar pesos de prioridad
memory.set_priority_weights(PriorityWeights::usage_focused());
```

### Niveles de Transferencia

```
┌─────────────────────────────────────────────────────────────┐
│                     UNIVERSAL (1.0)                         │
│        Patrones, algoritmos, principios generales           │
├─────────────────────────────────────────────────────────────┤
│                      DOMAIN (0.75)                          │
│         Conocimiento específico del dominio                 │
├─────────────────────────────────────────────────────────────┤
│                     CONTEXT (0.50)                          │
│            Conocimiento del contexto actual                 │
├─────────────────────────────────────────────────────────────┤
│                    INSTANCE (0.25)                          │
│          Conocimiento específico de la instancia            │
└─────────────────────────────────────────────────────────────┘
```

```rust
// Recall solo conocimiento universal
let universal = memory.recall_universal(&query, 10)?;

// Recall en el mismo dominio
let domain = memory.recall_same_domain(&query, "backend", 10)?;

// Recall en el mismo contexto
let context = memory.recall_same_context(&query, "rust", 10)?;

// Recall solo prioridad crítica
let critical = memory.recall_critical(&query, 5)?;

// Recall alta prioridad (High + Critical)
let important = memory.recall_high_priority(&query, 10)?;
```

### Score Combinado Final

```
combined_score = (relevance × 0.4) + (transfer × 0.3) + (priority × 0.3)
```

```rust
// Ajustar pesos del score final
memory.set_score_weights(0.5, 0.25, 0.25); // 50% relevancia, 25% transfer, 25% priority

// Ajustar umbral de transferibilidad
memory.set_transfer_threshold(0.5); // Solo incluir si transfer_score >= 0.5
```

### Crear un Preset Personalizado

```rust
use minimemory::memory_traits::{
    DomainClassifier, ConceptExtractor, ContextMatcher,
    PriorityCalculator, DomainPreset, DecayConfig, PriorityWeights, Priority
};

// 1. Implementar los traits
#[derive(Debug, Default)]
struct MyDomainClassifier;

impl DomainClassifier for MyDomainClassifier {
    fn domains(&self) -> Vec<&'static str> {
        vec!["finance", "trading", "risk"]
    }

    fn classify(&self, content: &str) -> String {
        if content.contains("trade") { "trading".into() }
        else if content.contains("risk") { "risk".into() }
        else { "finance".into() }
    }

    fn is_related(&self, d1: &str, d2: &str) -> bool {
        true // Todos relacionados en finanzas
    }
}

// 2. Definir el preset
struct FinancePreset;

impl DomainPreset for FinancePreset {
    type Domain = MyDomainClassifier;
    type Concepts = MyConceptExtractor;
    type Context = MyContextMatcher;
    type Priority = MyPriorityCalculator;

    fn name() -> &'static str { "Finance" }
    fn description() -> &'static str { "Memory for financial applications" }

    fn default_decay() -> DecayConfig {
        DecayConfig::slow() // Conocimiento financiero persiste
    }

    fn default_weights() -> PriorityWeights {
        PriorityWeights::manual_focused() // Prioridad manual importante
    }
}

// 3. Usar
let memory = GenericMemory::<FinancePreset>::new(768)?;
```

## Rendimiento

| Operación | Flat | HNSW |
|-----------|------|------|
| Insertar | O(1) | O(log n) |
| Buscar | O(n × d) | O(log n × d) |
| Keyword Search | O(n) | O(n) |
| Filtrar | O(n) | O(n) |

*n = número de documentos, d = dimensiones*

## Benchmarks

Ejecutar benchmarks completos:

```bash
cargo bench
```

### Resultados de Referencia

Medidos en CPU moderno (benchmark con Criterion):

#### Búsqueda Vectorial

| Dataset | Flat | HNSW |
|---------|------|------|
| 100×128d | ~15µs | ~18µs |
| 1000×128d | ~85µs | ~35µs |
| 5000×128d | ~420µs | ~45µs |
| 1000×384d | ~180µs | ~50µs |

#### Búsqueda Híbrida (500 documentos)

| Operación | Tiempo |
|-----------|--------|
| Vector only | ~65µs |
| Keyword only (BM25) | ~45µs |
| Hybrid (vector + keyword) | ~120µs |
| Hybrid + filter | ~135µs |

#### Memory Traits (GenericMemory)

| Operación | Tiempo |
|-----------|--------|
| learn (100 items) | ~2.8ms |
| recall k=10 | ~890µs |
| recall_critical | ~420µs |
| recall_high_priority | ~520µs |
| recall_by_keywords | ~139µs |
| mark_useful | ~2µs |

#### Filtros de Metadata (1000 documentos)

| Filtro | Tiempo |
|--------|--------|
| Equality (eq) | ~95µs |
| Comparison (gt) | ~110µs |
| Combined (AND) | ~125µs |
| Filter-only search | ~85µs |

#### Cálculo de Distancias

| Dimensiones | Cosine | Euclidean | DotProduct |
|-------------|--------|-----------|------------|
| 64d | ~45ns | ~35ns | ~25ns |
| 128d | ~85ns | ~65ns | ~45ns |
| 384d | ~220ns | ~170ns | ~120ns |
| 768d | ~430ns | ~340ns | ~240ns |
| 1536d | ~850ns | ~680ns | ~480ns |

#### Persistencia

| Tamaño | Save | Load |
|--------|------|------|
| 100 docs | ~180µs | ~150µs |
| 1000 docs | ~1.8ms | ~1.5ms |
| 5000 docs | ~9ms | ~7.5ms |

### Ejecutar Benchmarks Específicos

```bash
# Solo búsqueda vectorial
cargo bench -- search

# Solo memory traits
cargo bench -- memory_traits

# Solo híbrido
cargo bench -- hybrid

# Solo filtros
cargo bench -- filters
```

## Ejemplos

### Ejemplo con Ollama (Embeddings Reales)

El proyecto incluye un ejemplo completo que demuestra GenericMemory con embeddings reales usando Ollama.

#### Requisitos

1. [Ollama](https://ollama.ai) instalado y corriendo
2. Modelo de embedding instalado:

```bash
# Instalar modelo de embedding (elige uno)
ollama pull embeddinggemma      # Google (768 dims)
ollama pull nomic-embed-text    # Nomic (768 dims)
ollama pull mxbai-embed-large   # MixedBread (1024 dims)
```

#### Ejecutar el Ejemplo

```bash
cargo run --example ollama_memory
```

#### Qué Demuestra

```rust
// 1. Conexión con Ollama para embeddings
let ollama = OllamaClient::new("embeddinggemma");
let embedding = ollama.embed("texto a vectorizar")?;

// 2. GenericMemory con preset SoftwareDevelopment
let memory = GenericMemory::<SoftwareDevelopment>::new(dims)?;
memory.set_context(
    InstanceContext::new("demo-project")
        .with_context("rust")
        .with_domain("backend")
);

// 3. Aprendizaje con diferentes prioridades
memory.learn_with_priority(
    "security-fix-1",
    &embedding,
    "Sanitized user input using parameterized queries",
    "Fixed SQL injection vulnerability in user login",
    "success",
    Priority::Critical
)?;

// 4. Recall con diferentes estrategias
let results = memory.recall(&query, 5)?;           // General
let critical = memory.recall_critical(&query, 5)?; // Solo Critical
let high = memory.recall_high_priority(&query, 10)?; // High + Critical

// 5. Sistema de feedback
memory.mark_useful("security-fix-1");  // Aumenta usefulness score

// 6. Búsqueda por keywords (BM25)
let keyword_results = memory.recall_by_keywords("JWT token", 5)?;

// 7. Estadísticas
let stats = memory.stats();
println!("Total: {}, Accesos: {}, Utilidad: {:.0}%",
    stats.total_memories,
    stats.total_accesses,
    stats.avg_usefulness * 100.0
);
```

#### Salida Esperada

```
=== minimemory + Ollama Demo ===

Conectando con Ollama... OK! (modelo: embeddinggemma, dims: 768)
Memoria inicializada con preset: SoftwareDevelopment

--- Fase 1: Aprendiendo experiencias ---
  Aprendiendo: security-fix-1... OK (priority: Critical)
  Aprendiendo: bug-fix-1... OK (priority: High)
  ...

--- Fase 2: Probando recall ---
Query: 'security vulnerability fix'
Top 3 resultados:
  1. security-fix-1 (relevance: 0.923, priority: Critical, transfer: Domain)
  2. bug-fix-1 (relevance: 0.756, priority: High, transfer: Context)
  ...

--- Estadísticas finales ---
Total memorias: 8
Preset: SoftwareDevelopment
Total accesos: 24
Utilidad promedio: 50.00%
```

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
- [x] **Quantización de vectores (Int8, Binary)**
- [x] **Índices parciales**
- [x] **Replicación (Change Log, Sync, Snapshots)**
- [x] **Memoria Agéntica (Agent Memory Framework)**
- [x] **Transferencia de conocimiento entre proyectos**
- [x] **Sistema domain-agnostic con traits**
- [x] **Prioridad híbrida (manual + automática + uso + recencia)**
- [x] **Decay temporal exponencial configurable**
- [x] **Presets: SoftwareDevelopment, Conversational, CustomerService**
- [x] **HNSW index persistence** (serialized to .mmdb)
- [x] **BM25 index persistence** (serialized to .mmdb)
- [x] **CRC32 checksum** verification on .mmdb files
- [x] **Atomic writes** (.tmp + rename) for crash safety
- [x] **AgentMemory unified as GenericMemory facade** (inherits priority, decay, usage stats)
- [x] **UsageStats persistence** across save/load cycles
- [x] **WorkingContext persistence** across save/load cycles
- [x] **Unified TransferLevel** (eliminated duplicate enum)
- [x] **HNSW entry point recovery** after node deletion
- [x] **HNSW index defragmentation** (free indices pool)
- [x] **272 total tests** (159 unit + 83 integration + 30 doc-tests)

## Licencia

MIT
