//! Benchmarks para minimemory.
//!
//! Ejecutar con: `cargo bench`
//!
//! Benchmarks incluidos:
//! - insert: Inserción de vectores (Flat vs HNSW)
//! - search: Búsqueda vectorial por tamaño
//! - distance: Cálculo de distancias
//! - persistence: Guardar/cargar bases de datos
//! - bm25: Búsqueda full-text
//! - hybrid: Búsqueda híbrida (vector + keyword)
//! - memory_traits: Sistema de memoria genérica con prioridades

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use minimemory::{Config, Distance, IndexType, VectorDB, Metadata, Filter};
use minimemory::memory_traits::{GenericMemory, Priority};
use minimemory::memory_traits::presets::SoftwareDevelopment;

fn generate_vector(dim: usize, seed: usize) -> Vec<f32> {
    (0..dim)
        .map(|i| ((seed * dim + i) % 1000) as f32 / 1000.0)
        .collect()
}

fn generate_normalized_vector(dim: usize, seed: usize) -> Vec<f32> {
    let mut v: Vec<f32> = (0..dim)
        .map(|i| ((seed * dim + i) % 1000) as f32 / 1000.0 - 0.5)
        .collect();

    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in &mut v {
            *x /= norm;
        }
    }
    v
}

// ============================================================================
// Benchmarks de inserción
// ============================================================================

fn bench_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert");

    for dim in [64, 128, 384, 768].iter() {
        group.bench_with_input(
            BenchmarkId::new("flat", dim),
            dim,
            |b, &dim| {
                b.iter_with_setup(
                    || VectorDB::new(Config::new(dim)).unwrap(),
                    |db| {
                        for i in 0..100 {
                            let v = generate_vector(dim, i);
                            db.insert(format!("v{}", i), &v, None).unwrap();
                        }
                    },
                )
            },
        );

        group.bench_with_input(
            BenchmarkId::new("hnsw", dim),
            dim,
            |b, &dim| {
                b.iter_with_setup(
                    || VectorDB::new(Config::new(dim).with_index(IndexType::hnsw())).unwrap(),
                    |db| {
                        for i in 0..100 {
                            let v = generate_vector(dim, i);
                            db.insert(format!("v{}", i), &v, None).unwrap();
                        }
                    },
                )
            },
        );
    }

    group.finish();
}

// ============================================================================
// Benchmarks de búsqueda
// ============================================================================

fn bench_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("search");

    // Preparar bases de datos con diferentes tamaños
    for (size, dim) in [(100, 128), (1000, 128), (5000, 128), (1000, 384)].iter() {
        let db_flat = VectorDB::new(
            Config::new(*dim)
                .with_distance(Distance::Cosine)
                .with_index(IndexType::Flat),
        )
        .unwrap();

        let db_hnsw = VectorDB::new(
            Config::new(*dim)
                .with_distance(Distance::Cosine)
                .with_index(IndexType::hnsw()),
        )
        .unwrap();

        for i in 0..*size {
            let v = generate_normalized_vector(*dim, i);
            db_flat.insert(format!("v{}", i), &v, None).unwrap();
            db_hnsw.insert(format!("v{}", i), &v, None).unwrap();
        }

        let query = generate_normalized_vector(*dim, 9999);

        group.bench_with_input(
            BenchmarkId::new("flat", format!("{}x{}", size, dim)),
            &(&db_flat, &query),
            |b, (db, q)| {
                b.iter(|| {
                    black_box(db.search(q, 10).unwrap())
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("hnsw", format!("{}x{}", size, dim)),
            &(&db_hnsw, &query),
            |b, (db, q)| {
                b.iter(|| {
                    black_box(db.search(q, 10).unwrap())
                })
            },
        );
    }

    group.finish();
}

// ============================================================================
// Benchmarks de distancia
// ============================================================================

fn bench_distance(c: &mut Criterion) {
    let mut group = c.benchmark_group("distance");

    for dim in [64, 128, 384, 768, 1536].iter() {
        let a = generate_normalized_vector(*dim, 1);
        let b = generate_normalized_vector(*dim, 2);

        group.bench_with_input(
            BenchmarkId::new("cosine", dim),
            &(&a, &b),
            |bench, (a, b)| {
                bench.iter(|| {
                    black_box(Distance::Cosine.calculate(a, b))
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("euclidean", dim),
            &(&a, &b),
            |bench, (a, b)| {
                bench.iter(|| {
                    black_box(Distance::Euclidean.calculate(a, b))
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("dot_product", dim),
            &(&a, &b),
            |bench, (a, b)| {
                bench.iter(|| {
                    black_box(Distance::DotProduct.calculate(a, b))
                })
            },
        );
    }

    group.finish();
}

// ============================================================================
// Benchmarks de persistencia
// ============================================================================

fn bench_persistence(c: &mut Criterion) {
    use std::fs;

    let mut group = c.benchmark_group("persistence");

    for size in [100, 1000, 5000].iter() {
        let dim = 128;
        let db = VectorDB::new(Config::new(dim)).unwrap();

        for i in 0..*size {
            let v = generate_vector(dim, i);
            db.insert(format!("v{}", i), &v, None).unwrap();
        }

        let path = format!("bench_{}_{}.mmdb", size, std::process::id());

        group.bench_with_input(
            BenchmarkId::new("save", size),
            &(&db, &path),
            |b, (db, path)| {
                b.iter(|| {
                    db.save(path).unwrap()
                })
            },
        );

        // Guardar para benchmark de load
        db.save(&path).unwrap();

        group.bench_with_input(
            BenchmarkId::new("open", size),
            &path,
            |b, path| {
                b.iter(|| {
                    black_box(VectorDB::open(path).unwrap())
                })
            },
        );

        fs::remove_file(&path).ok();
    }

    group.finish();
}

// ============================================================================
// Benchmarks de BM25 (full-text search)
// ============================================================================

fn bench_bm25(c: &mut Criterion) {
    let mut group = c.benchmark_group("bm25");

    // Sample documents for full-text search
    let documents = vec![
        ("doc1", "Rust programming language systems programming memory safety"),
        ("doc2", "Python machine learning data science artificial intelligence"),
        ("doc3", "JavaScript web development frontend React Angular Vue"),
        ("doc4", "Database SQL PostgreSQL MySQL performance optimization"),
        ("doc5", "Kubernetes Docker containers orchestration microservices"),
        ("doc6", "Security authentication authorization JWT tokens encryption"),
        ("doc7", "API REST GraphQL endpoints HTTP web services"),
        ("doc8", "Testing unit tests integration tests TDD BDD coverage"),
        ("doc9", "Git version control branching merging collaboration"),
        ("doc10", "Cloud AWS Azure GCP infrastructure serverless"),
    ];

    for size in [10, 100, 500].iter() {
        let db = VectorDB::with_fulltext(
            Config::new(128),
            vec!["content".into()],
        ).unwrap();

        // Insert documents
        for i in 0..*size {
            let (id, content) = &documents[i % documents.len()];
            let mut meta = Metadata::new();
            meta.insert("content", format!("{} variant {}", content, i));
            let v = generate_vector(128, i);
            db.insert_document(&format!("{}_{}", id, i), Some(&v), Some(meta)).unwrap();
        }

        group.bench_with_input(
            BenchmarkId::new("keyword_search", size),
            &db,
            |b, db| {
                b.iter(|| {
                    black_box(db.keyword_search("programming Rust", 10).unwrap())
                })
            },
        );
    }

    group.finish();
}

// ============================================================================
// Benchmarks de búsqueda híbrida
// ============================================================================

fn bench_hybrid(c: &mut Criterion) {
    use minimemory::HybridSearchParams;

    let mut group = c.benchmark_group("hybrid");

    let db = VectorDB::with_fulltext(
        Config::new(128),
        vec!["content".into()],
    ).unwrap();

    // Insert 500 documents with vectors and metadata
    for i in 0..500 {
        let mut meta = Metadata::new();
        meta.insert("content", format!("Document about {} topic {}",
            if i % 3 == 0 { "Rust programming" }
            else if i % 3 == 1 { "Python machine learning" }
            else { "JavaScript web development" },
            i
        ));
        meta.insert("category", if i % 2 == 0 { "tech" } else { "science" });
        meta.insert("score", (i % 100) as f64 / 100.0);

        let v = generate_normalized_vector(128, i);
        db.insert_document(&format!("doc_{}", i), Some(&v), Some(meta)).unwrap();
    }

    let query = generate_normalized_vector(128, 9999);

    // Vector-only search
    group.bench_function("vector_only_500", |b| {
        b.iter(|| {
            black_box(db.search(&query, 10).unwrap())
        })
    });

    // Keyword-only search
    group.bench_function("keyword_only_500", |b| {
        b.iter(|| {
            black_box(db.keyword_search("Rust programming", 10).unwrap())
        })
    });

    // Hybrid search (vector + keyword)
    group.bench_function("hybrid_500", |b| {
        let params = HybridSearchParams::hybrid(
            query.clone(),
            "Rust programming",
            10,
        );
        b.iter(|| {
            black_box(db.hybrid_search(params.clone()).unwrap())
        })
    });

    // Hybrid with filter
    group.bench_function("hybrid_filtered_500", |b| {
        let params = HybridSearchParams::hybrid(
            query.clone(),
            "Rust programming",
            10,
        ).with_filter(Filter::eq("category", "tech"));
        b.iter(|| {
            black_box(db.hybrid_search(params.clone()).unwrap())
        })
    });

    group.finish();
}

// ============================================================================
// Benchmarks de GenericMemory (memory_traits)
// ============================================================================

fn bench_memory_traits(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_traits");
    let dim = 128;

    // Crear memoria con preset de desarrollo de software
    let memory = GenericMemory::<SoftwareDevelopment>::new(dim).unwrap();
    memory.set_instance("benchmark-project", "rust", "backend");

    // Preparar datos de prueba
    let embeddings: Vec<Vec<f32>> = (0..100)
        .map(|i| generate_normalized_vector(dim, i))
        .collect();

    // Benchmark: learn
    group.bench_function("learn_100", |b| {
        b.iter_with_setup(
            || GenericMemory::<SoftwareDevelopment>::new(dim).unwrap(),
            |mem| {
                for (i, emb) in embeddings.iter().enumerate() {
                    mem.learn(
                        &format!("task_{}", i),
                        emb,
                        &format!("Fixed bug in module {}", i),
                        "Bug fix for authentication issue",
                        "success",
                    ).unwrap();
                }
            },
        )
    });

    // Preparar memoria con datos para recall benchmarks
    let memory_filled = GenericMemory::<SoftwareDevelopment>::new(dim).unwrap();
    memory_filled.set_instance("benchmark-project", "rust", "backend");

    for (i, emb) in embeddings.iter().enumerate() {
        let priority = match i % 4 {
            0 => Priority::Low,
            1 => Priority::Normal,
            2 => Priority::High,
            _ => Priority::Critical,
        };
        memory_filled.learn_with_priority(
            &format!("task_{}", i),
            emb,
            &format!("Task content {}", i),
            &format!("Description for task {}", i),
            if i % 2 == 0 { "success" } else { "failure" },
            priority,
        ).unwrap();
    }

    let query = generate_normalized_vector(dim, 9999);

    // Benchmark: recall
    group.bench_function("recall_k10", |b| {
        b.iter(|| {
            black_box(memory_filled.recall(&query, 10).unwrap())
        })
    });

    // Benchmark: recall_critical
    group.bench_function("recall_critical", |b| {
        b.iter(|| {
            black_box(memory_filled.recall_critical(&query, 10).unwrap())
        })
    });

    // Benchmark: recall_high_priority
    group.bench_function("recall_high_priority", |b| {
        b.iter(|| {
            black_box(memory_filled.recall_high_priority(&query, 10).unwrap())
        })
    });

    // Benchmark: recall_by_keywords
    group.bench_function("recall_by_keywords", |b| {
        b.iter(|| {
            black_box(memory_filled.recall_by_keywords("task content", 10).unwrap())
        })
    });

    // Benchmark: mark_useful (feedback loop)
    group.bench_function("mark_useful", |b| {
        b.iter(|| {
            for i in 0..10 {
                memory_filled.mark_useful(&format!("task_{}", i));
            }
        })
    });

    group.finish();
}

// ============================================================================
// Benchmarks de filtros de metadata
// ============================================================================

fn bench_filters(c: &mut Criterion) {
    let mut group = c.benchmark_group("filters");

    let db = VectorDB::new(Config::new(128)).unwrap();

    // Insert documents with varied metadata
    for i in 0..1000 {
        let mut meta = Metadata::new();
        meta.insert("category", if i % 3 == 0 { "tech" } else if i % 3 == 1 { "science" } else { "art" });
        meta.insert("score", (i % 100) as f64 / 100.0);
        meta.insert("active", i % 2 == 0);
        meta.insert("priority", if i % 4 == 0 { "critical" } else { "normal" });

        let v = generate_normalized_vector(128, i);
        db.insert(&format!("doc_{}", i), &v, Some(meta)).unwrap();
    }

    let query = generate_normalized_vector(128, 9999);

    // Simple equality filter
    group.bench_function("filter_eq_1000", |b| {
        b.iter(|| {
            black_box(db.search_with_filter(&query, 10, Filter::eq("category", "tech")).unwrap())
        })
    });

    // Numeric comparison filter
    group.bench_function("filter_gt_1000", |b| {
        b.iter(|| {
            black_box(db.search_with_filter(&query, 10, Filter::gt("score", 0.5f64)).unwrap())
        })
    });

    // Combined AND filter
    group.bench_function("filter_and_1000", |b| {
        b.iter(|| {
            black_box(db.search_with_filter(
                &query,
                10,
                Filter::and(vec![
                    Filter::eq("category", "tech"),
                    Filter::eq("active", true)
                ])
            ).unwrap())
        })
    });

    // Filter-only search (no vector)
    group.bench_function("filter_search_1000", |b| {
        b.iter(|| {
            black_box(db.filter_search(
                Filter::eq("priority", "critical"),
                10
            ).unwrap())
        })
    });

    group.finish();
}

// ============================================================================
// Configuración de criterion
// ============================================================================

criterion_group!(
    benches,
    bench_insert,
    bench_search,
    bench_distance,
    bench_persistence,
    bench_bm25,
    bench_hybrid,
    bench_memory_traits,
    bench_filters,
);

criterion_main!(benches);
