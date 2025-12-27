//! Benchmarks para minimemory.
//!
//! Ejecutar con: `cargo bench`

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use minimemory::{Config, Distance, IndexType, VectorDB};

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
// Configuración de criterion
// ============================================================================

criterion_group!(
    benches,
    bench_insert,
    bench_search,
    bench_distance,
    bench_persistence,
);

criterion_main!(benches);
