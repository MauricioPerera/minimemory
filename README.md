# minimemory

Base de datos vectorial embebida para Rust. Como SQLite, pero para búsqueda por similaridad de vectores.

## Características

- **Sin servidor**: Librería embebida, solo importar y usar
- **Ligera**: Sin dependencias pesadas
- **Rápida**: Optimizada para alto rendimiento
- **Flexible**: Múltiples métricas de distancia
- **Type-safe**: API idiomática de Rust

## Instalación

Agrega a tu `Cargo.toml`:

```toml
[dependencies]
minimemory = { path = "." }  # o desde crates.io cuando se publique
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

## API

### Configuración

```rust
use minimemory::{Config, Distance, IndexType};

// Configuración básica
let config = Config::new(dimensions);

// Con opciones
let config = Config::new(384)
    .with_distance(Distance::Cosine)      // Métrica de distancia
    .with_index(IndexType::Flat);          // Tipo de índice
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
// Insertar
db.insert("id", &vector, None)?;
db.insert("id", &vector, Some(metadata))?;

// Insertar en lote
db.insert_batch(&[
    ("id1", &vec1, None),
    ("id2", &vec2, Some(meta)),
])?;

// Obtener
if let Some((vector, metadata)) = db.get("id")? {
    println!("Vector: {:?}", vector);
}

// Actualizar
db.update("id", &new_vector, new_metadata)?;

// Eliminar
db.delete("id")?;

// Verificar existencia
if db.contains("id") {
    println!("Existe");
}
```

### Búsqueda

```rust
// Buscar k vecinos más cercanos
let results = db.search(&query_vector, k)?;

for result in results {
    println!("ID: {}", result.id);
    println!("Distancia: {}", result.distance);
    if let Some(meta) = result.metadata {
        println!("Metadata: {:?}", meta);
    }
}
```

### Metadata

```rust
use minimemory::Metadata;

let mut meta = Metadata::new();
meta.insert("title", "Mi documento");
meta.insert("score", 42i64);
meta.insert("rating", 4.5f64);
meta.insert("active", true);

db.insert("doc-1", &vector, Some(meta))?;

// Recuperar metadata
if let Some((_, Some(meta))) = db.get("doc-1")? {
    if let Some(title) = meta.get("title") {
        println!("Título: {:?}", title);
    }
}
```

### Utilidades

```rust
// Número de vectores
let count = db.len();

// Verificar si está vacía
if db.is_empty() {
    println!("Base de datos vacía");
}

// Limpiar todo
db.clear();

// Obtener configuración
let dims = db.dimensions();
let dist = db.distance();
```

## Ejemplos

### Búsqueda Semántica Simple

```rust
use minimemory::{VectorDB, Config, Distance, IndexType, Metadata};

fn main() -> minimemory::Result<()> {
    let db = VectorDB::new(
        Config::new(4)
            .with_distance(Distance::Cosine)
    )?;

    // Simular embeddings de documentos
    let docs = [
        ("doc1", [0.9, 0.1, 0.0, 0.0], "Guía de programación en Rust"),
        ("doc2", [0.8, 0.2, 0.1, 0.0], "Tutorial de Cargo"),
        ("doc3", [0.1, 0.1, 0.9, 0.1], "Recetas de cocina"),
        ("doc4", [0.0, 0.1, 0.8, 0.2], "Restaurantes en Madrid"),
    ];

    for (id, embedding, title) in &docs {
        let mut meta = Metadata::new();
        meta.insert("title", *title);
        db.insert(*id, embedding, Some(meta))?;
    }

    // Buscar documentos similares a "programación"
    let query = [0.85, 0.15, 0.05, 0.0];  // Embedding de la consulta
    let results = db.search(&query, 2)?;

    println!("Resultados para 'programación':");
    for r in results {
        if let Some(meta) = r.metadata {
            println!("  - {:?} (dist: {:.3})", meta.get("title"), r.distance);
        }
    }

    Ok(())
}
```

### Manejo de Errores

```rust
use minimemory::{VectorDB, Config, Error};

fn main() {
    let db = VectorDB::new(Config::new(3)).unwrap();

    // Error: dimensiones incorrectas
    match db.insert("a", &[1.0, 2.0], None) {
        Err(Error::DimensionMismatch { expected, got }) => {
            println!("Error: esperaba {} dimensiones, recibió {}", expected, got);
        }
        _ => {}
    }

    // Error: ID duplicado
    db.insert("a", &[1.0, 2.0, 3.0], None).unwrap();
    match db.insert("a", &[4.0, 5.0, 6.0], None) {
        Err(Error::AlreadyExists(id)) => {
            println!("Error: '{}' ya existe", id);
        }
        _ => {}
    }
}
```

## Arquitectura

```
minimemory
├── VectorDB          # Interfaz principal
├── Storage           # Capa de almacenamiento
│   └── MemoryStorage # Almacenamiento en memoria (HashMap)
├── Index             # Capa de indexación
│   └── FlatIndex     # Índice de búsqueda exacta
└── Distance          # Métricas de distancia
    ├── Cosine
    ├── Euclidean
    └── DotProduct
```

## Rendimiento

| Operación | Complejidad (Flat) | Notas |
|-----------|-------------------|-------|
| Insertar | O(1) | Constante |
| Buscar | O(n × d) | n=vectores, d=dimensiones |
| Eliminar | O(1) | Constante |
| Obtener | O(1) | Constante |

## Persistencia

```rust
use minimemory::VectorDB;

// Guardar a disco
db.save("my_vectors.mmdb")?;

// Cargar desde disco
let db = VectorDB::open("my_vectors.mmdb")?;
```

## Bindings para Otros Lenguajes

minimemory soporta Python, Node.js/TypeScript y PHP:

### Python

```bash
# Instalar
pip install maturin
cd bindings/python
maturin develop --features python
```

```python
from minimemory import VectorDB

db = VectorDB(dimensions=384, distance="cosine", index_type="hnsw")
db.insert("doc-1", [0.1] * 384)
results = db.search([0.1] * 384, k=10)
```

### Node.js / TypeScript

```bash
# Instalar
npm install -g @napi-rs/cli
cd bindings/nodejs
npm install && npm run build
```

```javascript
const { VectorDB } = require('minimemory');

const db = new VectorDB({ dimensions: 384, distance: 'cosine' });
db.insert('doc-1', new Array(384).fill(0.1));
const results = db.search(new Array(384).fill(0.1), 10);
```

### PHP (FFI)

```bash
# Compilar librería
cargo build --release --features ffi
```

```php
<?php
require_once 'MiniMemory.php';
use MiniMemory\VectorDB;

$db = new VectorDB(384, 'cosine', 'hnsw');
$db->insert('doc-1', array_fill(0, 384, 0.1));
$results = $db->search(array_fill(0, 384, 0.1), 10);
?>
```

Ver documentación detallada en `bindings/*/README.md`.

## Roadmap

- [x] Storage en memoria
- [x] Índice Flat (búsqueda exacta)
- [x] Métricas: Cosine, Euclidean, DotProduct
- [x] Metadata en vectores
- [x] Índice HNSW (búsqueda aproximada)
- [x] Persistencia en disco (.mmdb)
- [x] Optimizaciones SIMD (AVX2/SSE)
- [x] Bindings Python (PyO3)
- [x] Bindings Node.js (napi-rs)
- [x] Bindings PHP (FFI)
- [ ] Filtrado por metadata
- [ ] Quantización de vectores

## Licencia

MIT
