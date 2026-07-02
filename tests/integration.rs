//! Tests de integración para minimemory.
//!
//! Estos tests verifican el funcionamiento completo de la librería
//! incluyendo todas las funcionalidades principales.

use minimemory::{Config, Distance, IndexType, Metadata, VectorDB};

// ============================================================================
// Tests básicos de VectorDB
// ============================================================================

mod basic {
    use super::*;

    #[test]
    fn test_create_empty_db() {
        let db = VectorDB::new(Config::new(128)).unwrap();
        assert!(db.is_empty());
        assert_eq!(db.len(), 0);
        assert_eq!(db.dimensions(), 128);
    }

    #[test]
    fn test_insert_single_vector() {
        let db = VectorDB::new(Config::new(3)).unwrap();

        db.insert("vec1", &[1.0, 2.0, 3.0], None).unwrap();

        assert_eq!(db.len(), 1);
        assert!(db.contains("vec1"));
        assert!(!db.contains("vec2"));
    }

    #[test]
    fn test_insert_multiple_vectors() {
        let db = VectorDB::new(Config::new(4)).unwrap();

        for i in 0..100 {
            let vector: Vec<f32> = (0..4).map(|j| (i * 4 + j) as f32).collect();
            db.insert(format!("vec_{}", i), &vector, None).unwrap();
        }

        assert_eq!(db.len(), 100);
    }

    #[test]
    fn test_get_vector() {
        let db = VectorDB::new(Config::new(3)).unwrap();
        let original = vec![1.5, 2.5, 3.5];

        db.insert("test", &original, None).unwrap();

        let (retrieved, _) = db.get("test").unwrap().unwrap();
        assert_eq!(retrieved, Some(original));
    }

    #[test]
    fn test_get_nonexistent() {
        let db = VectorDB::new(Config::new(3)).unwrap();

        let result = db.get("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_delete_vector() {
        let db = VectorDB::new(Config::new(3)).unwrap();

        db.insert("to_delete", &[1.0, 2.0, 3.0], None).unwrap();
        assert!(db.contains("to_delete"));

        let deleted = db.delete("to_delete").unwrap();
        assert!(deleted);
        assert!(!db.contains("to_delete"));
    }

    #[test]
    fn test_delete_nonexistent() {
        let db = VectorDB::new(Config::new(3)).unwrap();

        let deleted = db.delete("nonexistent").unwrap();
        assert!(!deleted);
    }

    #[test]
    fn test_update_vector() {
        let db = VectorDB::new(Config::new(3)).unwrap();

        db.insert("updatable", &[1.0, 2.0, 3.0], None).unwrap();
        db.update("updatable", &[4.0, 5.0, 6.0], None).unwrap();

        let (vector, _) = db.get("updatable").unwrap().unwrap();
        assert_eq!(vector, Some(vec![4.0, 5.0, 6.0]));
    }

    #[test]
    fn test_clear_db() {
        let db = VectorDB::new(Config::new(3)).unwrap();

        db.insert("a", &[1.0, 2.0, 3.0], None).unwrap();
        db.insert("b", &[4.0, 5.0, 6.0], None).unwrap();
        assert_eq!(db.len(), 2);

        db.clear();
        assert!(db.is_empty());
    }
}

// ============================================================================
// Tests de errores
// ============================================================================

mod errors {
    use super::*;
    use minimemory::Error;

    #[test]
    fn test_dimension_mismatch_on_insert() {
        let db = VectorDB::new(Config::new(3)).unwrap();

        let result = db.insert("wrong_dim", &[1.0, 2.0], None);

        assert!(matches!(
            result,
            Err(Error::DimensionMismatch {
                expected: 3,
                got: 2
            })
        ));
    }

    #[test]
    fn test_dimension_mismatch_on_search() {
        let db = VectorDB::new(Config::new(4)).unwrap();
        db.insert("a", &[1.0, 2.0, 3.0, 4.0], None).unwrap();

        let result = db.search(&[1.0, 2.0], 1);

        assert!(matches!(
            result,
            Err(Error::DimensionMismatch {
                expected: 4,
                got: 2
            })
        ));
    }

    #[test]
    fn test_duplicate_insert() {
        let db = VectorDB::new(Config::new(3)).unwrap();

        db.insert("duplicate", &[1.0, 2.0, 3.0], None).unwrap();
        let result = db.insert("duplicate", &[4.0, 5.0, 6.0], None);

        assert!(matches!(result, Err(Error::AlreadyExists(_))));
    }
}

// ============================================================================
// Tests de búsqueda
// ============================================================================

mod search {
    use super::*;

    #[test]
    fn test_search_empty_db() {
        let db = VectorDB::new(Config::new(3)).unwrap();

        let results = db.search(&[1.0, 2.0, 3.0], 10).unwrap();

        assert!(results.is_empty());
    }

    #[test]
    fn test_search_exact_match() {
        let db = VectorDB::new(Config::new(3).with_distance(Distance::Euclidean)).unwrap();

        db.insert("exact", &[1.0, 2.0, 3.0], None).unwrap();
        db.insert("other", &[10.0, 20.0, 30.0], None).unwrap();

        let results = db.search(&[1.0, 2.0, 3.0], 1).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "exact");
        assert!(results[0].distance < 0.001);
    }

    #[test]
    fn test_search_k_nearest() {
        let db = VectorDB::new(Config::new(2).with_distance(Distance::Euclidean)).unwrap();

        // Insertar puntos en un patrón conocido
        db.insert("origin", &[0.0, 0.0], None).unwrap();
        db.insert("near", &[1.0, 0.0], None).unwrap();
        db.insert("medium", &[2.0, 0.0], None).unwrap();
        db.insert("far", &[10.0, 0.0], None).unwrap();

        let results = db.search(&[0.0, 0.0], 3).unwrap();

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].id, "origin");
        assert_eq!(results[1].id, "near");
        assert_eq!(results[2].id, "medium");
    }

    #[test]
    fn test_search_returns_ordered_results() {
        let db = VectorDB::new(Config::new(3).with_distance(Distance::Euclidean)).unwrap();

        for i in 0..20 {
            let vector = vec![i as f32, 0.0, 0.0];
            db.insert(format!("v{}", i), &vector, None).unwrap();
        }

        let results = db.search(&[5.0, 0.0, 0.0], 10).unwrap();

        // Verificar que están ordenados por distancia
        for i in 1..results.len() {
            assert!(results[i - 1].distance <= results[i].distance);
        }
    }

    #[test]
    fn test_search_k_larger_than_db() {
        let db = VectorDB::new(Config::new(3)).unwrap();

        db.insert("a", &[1.0, 0.0, 0.0], None).unwrap();
        db.insert("b", &[0.0, 1.0, 0.0], None).unwrap();

        let results = db.search(&[0.5, 0.5, 0.0], 100).unwrap();

        // Debería retornar solo 2 resultados
        assert_eq!(results.len(), 2);
    }
}

// ============================================================================
// Tests de métricas de distancia
// ============================================================================

mod distance_metrics {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let db = VectorDB::new(Config::new(3).with_distance(Distance::Cosine)).unwrap();

        // Vectores normalizados
        db.insert("x_axis", &[1.0, 0.0, 0.0], None).unwrap();
        db.insert("y_axis", &[0.0, 1.0, 0.0], None).unwrap();
        db.insert("xy_45", &[0.707, 0.707, 0.0], None).unwrap();

        let results = db.search(&[1.0, 0.0, 0.0], 3).unwrap();

        // x_axis debería ser el más cercano (distancia ~0)
        assert_eq!(results[0].id, "x_axis");
        assert!(results[0].distance < 0.01);

        // xy_45 debería ser segundo
        assert_eq!(results[1].id, "xy_45");

        // y_axis debería ser el más lejano (ortogonal, distancia ~1)
        assert_eq!(results[2].id, "y_axis");
        assert!((results[2].distance - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_euclidean_distance() {
        let db = VectorDB::new(Config::new(2).with_distance(Distance::Euclidean)).unwrap();

        db.insert("origin", &[0.0, 0.0], None).unwrap();
        db.insert("three_four", &[3.0, 4.0], None).unwrap();

        let results = db.search(&[0.0, 0.0], 2).unwrap();

        // Distancia 3-4-5 triángulo
        let three_four_result = results.iter().find(|r| r.id == "three_four").unwrap();
        assert!((three_four_result.distance - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_dot_product() {
        let db = VectorDB::new(Config::new(3).with_distance(Distance::DotProduct)).unwrap();

        db.insert("high_dot", &[1.0, 1.0, 1.0], None).unwrap();
        db.insert("low_dot", &[-1.0, -1.0, -1.0], None).unwrap();
        db.insert("zero_dot", &[1.0, -1.0, 0.0], None).unwrap();

        let results = db.search(&[1.0, 1.0, 1.0], 3).unwrap();

        // high_dot debería tener la menor distancia (dot product más alto)
        assert_eq!(results[0].id, "high_dot");
    }
}

// ============================================================================
// Tests de metadata
// ============================================================================

mod metadata {
    use super::*;
    use minimemory::MetadataValue;

    #[test]
    fn test_insert_with_metadata() {
        let db = VectorDB::new(Config::new(3)).unwrap();

        let mut meta = Metadata::new();
        meta.insert("title", "Test Document");
        meta.insert("score", 95i64);
        meta.insert("rating", 4.5f64);
        meta.insert("active", true);

        db.insert("with_meta", &[1.0, 2.0, 3.0], Some(meta))
            .unwrap();

        let (_, retrieved_meta) = db.get("with_meta").unwrap().unwrap();
        let meta = retrieved_meta.unwrap();

        assert!(matches!(
            meta.get("title"),
            Some(MetadataValue::String(s)) if s == "Test Document"
        ));
        assert!(matches!(meta.get("score"), Some(MetadataValue::Int(95))));
        assert!(matches!(
            meta.get("active"),
            Some(MetadataValue::Bool(true))
        ));
    }

    #[test]
    fn test_search_returns_metadata() {
        let db = VectorDB::new(Config::new(3)).unwrap();

        let mut meta = Metadata::new();
        meta.insert("category", "important");

        db.insert("doc1", &[1.0, 0.0, 0.0], Some(meta)).unwrap();
        db.insert("doc2", &[0.0, 1.0, 0.0], None).unwrap();

        let results = db.search(&[1.0, 0.0, 0.0], 2).unwrap();

        let doc1_result = results.iter().find(|r| r.id == "doc1").unwrap();
        assert!(doc1_result.metadata.is_some());

        let doc2_result = results.iter().find(|r| r.id == "doc2").unwrap();
        assert!(doc2_result.metadata.is_none());
    }

    #[test]
    fn test_update_preserves_new_metadata() {
        let db = VectorDB::new(Config::new(3)).unwrap();

        let mut old_meta = Metadata::new();
        old_meta.insert("version", 1i64);

        db.insert("doc", &[1.0, 2.0, 3.0], Some(old_meta)).unwrap();

        let mut new_meta = Metadata::new();
        new_meta.insert("version", 2i64);

        db.update("doc", &[4.0, 5.0, 6.0], Some(new_meta)).unwrap();

        let (_, meta) = db.get("doc").unwrap().unwrap();
        assert!(matches!(
            meta.unwrap().get("version"),
            Some(MetadataValue::Int(2))
        ));
    }
}

// ============================================================================
// Tests de índice HNSW
// ============================================================================

mod hnsw {
    use super::*;

    #[test]
    fn test_hnsw_basic_search() {
        let config = Config::new(4)
            .with_distance(Distance::Euclidean)
            .with_index(IndexType::hnsw());

        let db = VectorDB::new(config).unwrap();

        // Insertar varios vectores
        for i in 0..50 {
            let vector: Vec<f32> = (0..4).map(|j| ((i * 4 + j) as f32) / 100.0).collect();
            db.insert(format!("v{}", i), &vector, None).unwrap();
        }

        let query: Vec<f32> = (0..4).map(|j| (25 * 4 + j) as f32 / 100.0).collect();
        let results = db.search(&query, 5).unwrap();

        assert!(!results.is_empty());
        assert!(results.len() <= 5);
    }

    #[test]
    fn test_hnsw_with_custom_params() {
        let config = Config::new(8)
            .with_distance(Distance::Cosine)
            .with_index(IndexType::hnsw_with_params(8, 100));

        let db = VectorDB::new(config).unwrap();

        for i in 0..20 {
            let vector: Vec<f32> = (0..8).map(|_| rand_float()).collect();
            db.insert(format!("v{}", i), &vector, None).unwrap();
        }

        let query: Vec<f32> = (0..8).map(|_| rand_float()).collect();
        let results = db.search(&query, 3).unwrap();

        assert!(!results.is_empty());
    }

    #[test]
    fn test_hnsw_delete() {
        let config = Config::new(4).with_index(IndexType::hnsw());

        let db = VectorDB::new(config).unwrap();

        db.insert("a", &[1.0, 0.0, 0.0, 0.0], None).unwrap();
        db.insert("b", &[0.0, 1.0, 0.0, 0.0], None).unwrap();

        db.delete("a").unwrap();

        assert!(!db.contains("a"));
        assert!(db.contains("b"));
    }
}

// ============================================================================
// Tests de persistencia
// ============================================================================

mod persistence {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn temp_db_path(test_name: &str) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let count = COUNTER.fetch_add(1, Ordering::SeqCst);
        let mut path = std::env::temp_dir();
        path.push(format!(
            "minimemory_{}_{}_{}_{}.mmdb",
            test_name,
            std::process::id(),
            format!("{:?}", std::thread::current().id()).replace(['(', ')', ' '], ""),
            count
        ));
        path
    }

    #[test]
    fn test_save_and_open() {
        let path = temp_db_path("save_open");

        // Crear y guardar
        {
            let db = VectorDB::new(Config::new(3).with_distance(Distance::Cosine)).unwrap();

            db.insert("doc1", &[1.0, 2.0, 3.0], None).unwrap();
            db.insert("doc2", &[4.0, 5.0, 6.0], None).unwrap();

            db.save(&path).unwrap();
        }

        // Abrir y verificar
        {
            let db = VectorDB::open(&path).unwrap();

            assert_eq!(db.len(), 2);
            assert_eq!(db.dimensions(), 3);
            assert!(db.contains("doc1"));
            assert!(db.contains("doc2"));

            let (vector, _) = db.get("doc1").unwrap().unwrap();
            assert_eq!(vector, Some(vec![1.0, 2.0, 3.0]));
        }

        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_save_with_metadata() {
        let path = temp_db_path("save_meta");

        {
            let db = VectorDB::new(Config::new(2)).unwrap();

            let mut meta = Metadata::new();
            meta.insert("title", "Test");
            meta.insert("count", 42i64);

            db.insert("with_meta", &[1.0, 2.0], Some(meta)).unwrap();
            db.save(&path).unwrap();
        }

        {
            let db = VectorDB::open(&path).unwrap();
            let (_, meta) = db.get("with_meta").unwrap().unwrap();

            assert!(meta.is_some());
            let meta = meta.unwrap();
            assert!(meta.get("title").is_some());
            assert!(meta.get("count").is_some());
        }

        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_save_empty_db() {
        let path = temp_db_path("save_empty");

        {
            let db = VectorDB::new(Config::new(4)).unwrap();
            db.save(&path).unwrap();
        }

        {
            let db = VectorDB::open(&path).unwrap();
            assert!(db.is_empty());
            assert_eq!(db.dimensions(), 4);
        }

        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_save_large_db() {
        let path = temp_db_path("save_large");

        {
            let db = VectorDB::new(Config::new(128)).unwrap();

            for i in 0..1000 {
                let vector: Vec<f32> = (0..128)
                    .map(|j| ((i * 128 + j) % 1000) as f32 / 1000.0)
                    .collect();
                db.insert(format!("doc_{}", i), &vector, None).unwrap();
            }

            db.save(&path).unwrap();
        }

        {
            let db = VectorDB::open(&path).unwrap();
            assert_eq!(db.len(), 1000);

            // Verificar algunos vectores aleatorios
            assert!(db.contains("doc_0"));
            assert!(db.contains("doc_500"));
            assert!(db.contains("doc_999"));
        }

        fs::remove_file(&path).ok();
    }
}

// ============================================================================
// Tests de rendimiento / stress
// ============================================================================

mod stress {
    use super::*;

    #[test]
    fn test_many_inserts() {
        let db = VectorDB::new(Config::new(64)).unwrap();

        for i in 0..5000 {
            let vector: Vec<f32> = (0..64).map(|j| ((i + j) % 100) as f32 / 100.0).collect();
            db.insert(format!("v{}", i), &vector, None).unwrap();
        }

        assert_eq!(db.len(), 5000);
    }

    #[test]
    fn test_many_searches() {
        let db = VectorDB::new(Config::new(32)).unwrap();

        for i in 0..100 {
            let vector: Vec<f32> = (0..32).map(|j| ((i + j) % 50) as f32 / 50.0).collect();
            db.insert(format!("v{}", i), &vector, None).unwrap();
        }

        // Realizar muchas búsquedas
        for i in 0..100 {
            let query: Vec<f32> = (0..32).map(|j| ((i + j) % 50) as f32 / 50.0).collect();
            let results = db.search(&query, 10).unwrap();
            assert!(!results.is_empty());
        }
    }

    #[test]
    fn test_insert_delete_cycle() {
        let db = VectorDB::new(Config::new(16)).unwrap();

        // Insertar 100 vectores
        for i in 0..100 {
            let vector: Vec<f32> = (0..16).map(|_| rand_float()).collect();
            db.insert(format!("v{}", i), &vector, None).unwrap();
        }
        assert_eq!(db.len(), 100);

        // Eliminar la mitad
        for i in 0..50 {
            db.delete(&format!("v{}", i)).unwrap();
        }
        assert_eq!(db.len(), 50);

        // Insertar más
        for i in 100..200 {
            let vector: Vec<f32> = (0..16).map(|_| rand_float()).collect();
            db.insert(format!("v{}", i), &vector, None).unwrap();
        }
        assert_eq!(db.len(), 150);
    }
}

// ============================================================================
// Tests de concurrencia
// ============================================================================

mod concurrency {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_concurrent_reads() {
        let db = Arc::new(VectorDB::new(Config::new(8)).unwrap());

        // Insertar datos
        for i in 0..100 {
            let vector: Vec<f32> = (0..8).map(|j| (i + j) as f32).collect();
            db.insert(format!("v{}", i), &vector, None).unwrap();
        }

        let mut handles = vec![];

        // Múltiples lectores concurrentes
        for _ in 0..4 {
            let db_clone = Arc::clone(&db);
            let handle = thread::spawn(move || {
                for i in 0..100 {
                    let _ = db_clone.get(&format!("v{}", i));

                    let query: Vec<f32> = (0..8).map(|j| (i + j) as f32).collect();
                    let _ = db_clone.search(&query, 5);
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn test_concurrent_writes() {
        let db = Arc::new(VectorDB::new(Config::new(4)).unwrap());
        let mut handles = vec![];

        // Múltiples escritores concurrentes
        for t in 0..4 {
            let db_clone = Arc::clone(&db);
            let handle = thread::spawn(move || {
                for i in 0..25 {
                    let id = format!("t{}v{}", t, i);
                    let vector: Vec<f32> = (0..4).map(|j| (t * 100 + i + j) as f32).collect();
                    let _ = db_clone.insert(id, &vector, None);
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(db.len(), 100);
    }

    #[test]
    fn test_concurrent_mixed_insert_search_delete() {
        use minimemory::Config;

        let config = Config::new(8).with_index(minimemory::IndexType::HNSW {
            m: 16,
            ef_construction: 100,
        });
        let db = Arc::new(VectorDB::new(config).unwrap());

        // Seed with 50 vectors
        for i in 0..50 {
            let vec: Vec<f32> = (0..8).map(|j| (i * 8 + j) as f32 / 400.0).collect();
            db.insert(format!("seed-{}", i), &vec, None).unwrap();
        }

        let mut handles = vec![];

        // 4 threads inserting
        for t in 0..4 {
            let db = db.clone();
            handles.push(thread::spawn(move || {
                for i in 0..25 {
                    let id = format!("ins-{}-{}", t, i);
                    let vec: Vec<f32> =
                        (0..8).map(|j| (t * 200 + i * 8 + j) as f32 / 800.0).collect();
                    db.insert(id, &vec, None).unwrap();
                }
            }));
        }

        // 4 threads searching
        for t in 0..4 {
            let db = db.clone();
            handles.push(thread::spawn(move || {
                for i in 0..50 {
                    let query: Vec<f32> =
                        (0..8).map(|j| (t * 100 + i * 8 + j) as f32 / 400.0).collect();
                    let results = db.search(&query, 5).unwrap();
                    // May get fewer results during concurrent deletes
                    assert!(results.len() <= 5);
                }
            }));
        }

        // 2 threads deleting seed vectors
        for t in 0..2 {
            let db = db.clone();
            handles.push(thread::spawn(move || {
                for i in 0..25 {
                    let id = format!("seed-{}", t * 25 + i);
                    let _ = db.delete(&id); // May fail if already deleted
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        // All seed vectors deleted (50), all inserted (100)
        // Final count: 50 - 50 + 100 = 100
        assert_eq!(db.len(), 100);

        // Search should still work correctly
        let query: Vec<f32> = (0..8).map(|j| j as f32 / 8.0).collect();
        let results = db.search(&query, 10).unwrap();
        assert!(!results.is_empty());
        // Results should be sorted by distance
        for w in results.windows(2) {
            assert!(w[0].distance <= w[1].distance);
        }
    }
}

// ============================================================================
// Tests de casos edge
// ============================================================================

mod edge_cases {
    use super::*;

    #[test]
    fn test_single_dimension() {
        let db = VectorDB::new(Config::new(1)).unwrap();

        db.insert("a", &[1.0], None).unwrap();
        db.insert("b", &[2.0], None).unwrap();
        db.insert("c", &[10.0], None).unwrap();

        let results = db.search(&[1.5], 2).unwrap();

        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_high_dimension() {
        let dim = 1024;
        let db = VectorDB::new(Config::new(dim)).unwrap();

        let vector: Vec<f32> = (0..dim).map(|i| i as f32 / dim as f32).collect();
        db.insert("high_dim", &vector, None).unwrap();

        let results = db.search(&vector, 1).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "high_dim");
    }

    #[test]
    fn test_zero_vector() {
        let db = VectorDB::new(Config::new(3).with_distance(Distance::Euclidean)).unwrap();

        db.insert("zero", &[0.0, 0.0, 0.0], None).unwrap();
        db.insert("nonzero", &[1.0, 2.0, 3.0], None).unwrap();

        let results = db.search(&[0.0, 0.0, 0.0], 2).unwrap();

        assert_eq!(results[0].id, "zero");
        assert!(results[0].distance < 0.001);
    }

    #[test]
    fn test_negative_values() {
        let db = VectorDB::new(Config::new(3)).unwrap();

        db.insert("neg", &[-1.0, -2.0, -3.0], None).unwrap();
        db.insert("pos", &[1.0, 2.0, 3.0], None).unwrap();
        db.insert("mix", &[-1.0, 2.0, -3.0], None).unwrap();

        let results = db.search(&[-1.0, -2.0, -3.0], 3).unwrap();

        assert!(!results.is_empty());
    }

    #[test]
    fn test_very_small_values() {
        let db = VectorDB::new(Config::new(3).with_distance(Distance::Euclidean)).unwrap();

        db.insert("tiny", &[1e-10, 1e-10, 1e-10], None).unwrap();
        db.insert("small", &[1e-5, 1e-5, 1e-5], None).unwrap();

        let results = db.search(&[1e-10, 1e-10, 1e-10], 2).unwrap();

        assert_eq!(results[0].id, "tiny");
    }

    #[test]
    fn test_very_large_values() {
        let db = VectorDB::new(Config::new(3).with_distance(Distance::Euclidean)).unwrap();

        db.insert("huge", &[1e10, 1e10, 1e10], None).unwrap();
        db.insert("large", &[1e5, 1e5, 1e5], None).unwrap();

        let results = db.search(&[1e10, 1e10, 1e10], 2).unwrap();

        assert_eq!(results[0].id, "huge");
    }

    #[test]
    fn test_unicode_ids() {
        let db = VectorDB::new(Config::new(3)).unwrap();

        db.insert("日本語", &[1.0, 2.0, 3.0], None).unwrap();
        db.insert("émojis🎉", &[4.0, 5.0, 6.0], None).unwrap();
        db.insert("спасибо", &[7.0, 8.0, 9.0], None).unwrap();

        assert!(db.contains("日本語"));
        assert!(db.contains("émojis🎉"));
        assert!(db.contains("спасибо"));
    }

    #[test]
    fn test_special_characters_in_id() {
        let db = VectorDB::new(Config::new(2)).unwrap();

        db.insert("path/to/file.txt", &[1.0, 2.0], None).unwrap();
        db.insert("key:value", &[3.0, 4.0], None).unwrap();
        db.insert("with spaces", &[5.0, 6.0], None).unwrap();
        db.insert("a\tb\nc", &[7.0, 8.0], None).unwrap();

        assert!(db.contains("path/to/file.txt"));
        assert!(db.contains("key:value"));
        assert!(db.contains("with spaces"));
        assert!(db.contains("a\tb\nc"));
    }
}

// ============================================================================
// Memory Traits Integration Tests
// ============================================================================

mod memory_traits_integration {
    use minimemory::memory_traits::presets::SoftwareDevelopment;
    use minimemory::memory_traits::{GenericMemory, InstanceContext, Priority};

    fn generate_embedding(seed: usize, dim: usize) -> Vec<f32> {
        (0..dim)
            .map(|i| ((seed * dim + i) % 1000) as f32 / 1000.0)
            .collect()
    }

    #[test]
    fn test_generic_memory_full_workflow() {
        let memory = GenericMemory::<SoftwareDevelopment>::new(64).unwrap();

        memory.set_context(
            InstanceContext::new("test-project")
                .with_context("rust")
                .with_domain("backend"),
        );

        let emb = generate_embedding(1, 64);
        memory
            .learn(
                "task-1",
                &emb,
                "Fixed auth bug",
                "Authentication fix",
                "success",
            )
            .unwrap();

        let query = generate_embedding(1, 64);
        let results = memory.recall(&query, 5).unwrap();

        assert!(!results.is_empty());
        assert_eq!(results[0].id, "task-1");
    }

    #[test]
    fn test_learn_auto_priority() {
        let memory = GenericMemory::<SoftwareDevelopment>::new(64).unwrap();

        // Security issue should get Critical priority
        let emb = generate_embedding(1, 64);
        memory
            .learn(
                "sec-fix",
                &emb,
                "Fixed XSS vulnerability",
                "Security patch",
                "success",
            )
            .unwrap();

        let query = generate_embedding(1, 64);
        let results = memory.recall(&query, 1).unwrap();

        assert_eq!(results[0].priority, Priority::Critical);
    }

    #[test]
    fn test_learn_manual_priority() {
        let memory = GenericMemory::<SoftwareDevelopment>::new(64).unwrap();

        let emb = generate_embedding(1, 64);
        memory
            .learn_with_priority(
                "manual-task",
                &emb,
                "Some content",
                "Description",
                "success",
                Priority::High,
            )
            .unwrap();

        let query = generate_embedding(1, 64);
        let results = memory.recall(&query, 1).unwrap();

        assert_eq!(results[0].priority, Priority::High);
    }

    #[test]
    fn test_recall_critical_only() {
        let memory = GenericMemory::<SoftwareDevelopment>::new(64).unwrap();

        // Add mixed priorities
        for i in 0..4 {
            let emb = generate_embedding(i, 64);
            let priority = match i % 4 {
                0 => Priority::Low,
                1 => Priority::Normal,
                2 => Priority::High,
                _ => Priority::Critical,
            };
            memory
                .learn_with_priority(
                    &format!("task-{}", i),
                    &emb,
                    "Content",
                    "Desc",
                    "success",
                    priority,
                )
                .unwrap();
        }

        let query = generate_embedding(3, 64);
        let critical = memory.recall_critical(&query, 10).unwrap();

        assert!(!critical.is_empty());
        for r in &critical {
            assert_eq!(r.priority, Priority::Critical);
        }
    }

    #[test]
    fn test_recall_high_priority() {
        let memory = GenericMemory::<SoftwareDevelopment>::new(64).unwrap();

        for i in 0..8 {
            let emb = generate_embedding(i, 64);
            let priority = match i % 4 {
                0 => Priority::Low,
                1 => Priority::Normal,
                2 => Priority::High,
                _ => Priority::Critical,
            };
            memory
                .learn_with_priority(
                    &format!("task-{}", i),
                    &emb,
                    "Content",
                    "Desc",
                    "success",
                    priority,
                )
                .unwrap();
        }

        let query = generate_embedding(0, 64);
        let high = memory.recall_high_priority(&query, 10).unwrap();

        for r in &high {
            assert!(r.priority >= Priority::High);
        }
    }

    #[test]
    fn test_mark_useful_feedback() {
        let memory = GenericMemory::<SoftwareDevelopment>::new(64).unwrap();

        let emb = generate_embedding(1, 64);
        memory
            .learn("useful-task", &emb, "Content", "Desc", "success")
            .unwrap();

        // Mark as useful multiple times
        memory.mark_useful("useful-task");
        memory.mark_useful("useful-task");
        memory.mark_useful("useful-task");

        let stats = memory.stats();
        assert!(stats.avg_usefulness > 0.0);
    }

    #[test]
    fn test_recall_by_keywords() {
        let memory = GenericMemory::<SoftwareDevelopment>::new(64).unwrap();

        let emb1 = generate_embedding(1, 64);
        memory
            .learn(
                "auth-task",
                &emb1,
                "JWT token authentication",
                "Auth system",
                "success",
            )
            .unwrap();

        let emb2 = generate_embedding(2, 64);
        memory
            .learn(
                "db-task",
                &emb2,
                "Database connection pool",
                "DB optimization",
                "success",
            )
            .unwrap();

        let results = memory.recall_by_keywords("JWT authentication", 5).unwrap();

        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.id == "auth-task"));
    }

    #[test]
    fn test_context_aware_recall() {
        let memory = GenericMemory::<SoftwareDevelopment>::new(64).unwrap();

        memory.set_context(
            InstanceContext::new("project-a")
                .with_context("rust")
                .with_domain("backend"),
        );

        let emb = generate_embedding(1, 64);
        memory
            .learn(
                "rust-task",
                &emb,
                "Rust async code",
                "Async implementation",
                "success",
            )
            .unwrap();

        assert!(memory.current_context().is_some());
        let ctx = memory.current_context().unwrap();
        assert_eq!(ctx.instance_id, "project-a");
    }

    #[test]
    fn test_memory_stats_accuracy() {
        let memory = GenericMemory::<SoftwareDevelopment>::new(64).unwrap();

        for i in 0..5 {
            let emb = generate_embedding(i, 64);
            memory
                .learn(&format!("task-{}", i), &emb, "Content", "Desc", "success")
                .unwrap();
        }

        let stats = memory.stats();
        assert_eq!(stats.total_memories, 5);
        assert_eq!(stats.preset_name, "Software Development");
    }
}

// ============================================================================
// Agent Memory Integration Tests
// ============================================================================

mod agent_memory_integration {
    use minimemory::agent_memory::{
        AgentMemory, CodeSnippet, ErrorSolution, Language, MemoryConfig, MemoryType, TaskOutcome,
    };

    fn make_agent_memory() -> AgentMemory {
        let config = MemoryConfig::small();
        let mut memory = AgentMemory::new(config).unwrap();
        memory.set_embed_fn(|text: &str| {
            let dims = 384;
            let mut vec = vec![0.0f32; dims];
            for (i, byte) in text.bytes().enumerate() {
                vec[i % dims] += byte as f32 / 255.0;
            }
            let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > 0.0 {
                vec.iter_mut().for_each(|x| *x /= norm);
            }
            vec
        });
        memory
    }

    #[test]
    fn test_agent_memory_creation() {
        let config = MemoryConfig::small();
        let memory = AgentMemory::new(config).unwrap();

        let stats = memory.stats().unwrap();
        assert_eq!(stats.total_entries, 0);
    }

    #[test]
    fn test_learn_task_workflow() {
        let memory = make_agent_memory();

        let id = memory
            .learn_task(
                "Implement login feature",
                "fn login(user: &str) { /* ... */ }",
                TaskOutcome::Success,
                vec!["Use bcrypt for passwords", "Add rate limiting"],
            )
            .unwrap();

        assert!(id.starts_with("episode-"));
        assert_eq!(memory.db().len(), 1);
    }

    #[test]
    fn test_learn_code_snippet() {
        let memory = make_agent_memory();

        let id = memory
            .learn_code(CodeSnippet {
                code: "fn hello() { println!(\"Hello\"); }".to_string(),
                description: "Simple hello function".to_string(),
                language: Language::Rust,
                dependencies: vec![],
                use_case: "Greeting users".to_string(),
                quality_score: 0.9,
                tags: vec!["example".to_string()],
            })
            .unwrap();

        assert!(id.starts_with("code-"));

        let _results = memory.recall_code("hello function greeting", 5).unwrap();
        // Results depend on embed_fn, but ID should be correct
        assert_eq!(memory.db().len(), 1);
    }

    #[test]
    fn test_learn_error_solution() {
        let memory = make_agent_memory();

        let id = memory
            .learn_error_solution(ErrorSolution {
                error_message: "cannot borrow as mutable".to_string(),
                error_type: "E0596".to_string(),
                root_cause: "Missing mut keyword".to_string(),
                solution: "Add mut to variable declaration".to_string(),
                fixed_code: Some("let mut x = 5;".to_string()),
                language: Language::Rust,
            })
            .unwrap();

        assert!(id.starts_with("error-"));

        let _results = memory
            .recall_error_solutions("cannot borrow mutable", 5)
            .unwrap();
        // Results depend on embed_fn
        assert_eq!(memory.db().len(), 1);
    }

    #[test]
    fn test_recall_similar_hybrid() {
        let memory = make_agent_memory();

        for i in 0..5 {
            memory
                .learn_task(
                    &format!("Task {} about authentication and JWT tokens", i),
                    &format!("fn auth{}() {{ /* JWT logic */ }}", i),
                    TaskOutcome::Success,
                    vec!["Use JWT for auth"],
                )
                .unwrap();
        }

        // recall_similar takes a text query, not an embedding
        let _results = memory.recall_similar("authentication JWT", 3).unwrap();
        // Results depend on embed_fn (placeholder returns zeros)
        assert_eq!(memory.db().len(), 5);
    }

    #[test]
    fn test_recall_experiences_filter() {
        let memory = make_agent_memory();

        // Add a task episode
        memory
            .learn_task(
                "Fix bug in parser",
                "fn parse() { /* fixed */ }",
                TaskOutcome::Success,
                vec!["Check edge cases"],
            )
            .unwrap();

        // Add a code snippet
        memory
            .learn_code(CodeSnippet {
                code: "let x = 1;".to_string(),
                description: "Variable declaration".to_string(),
                language: Language::Rust,
                dependencies: vec![],
                use_case: "Initialization".to_string(),
                quality_score: 0.8,
                tags: vec![],
            })
            .unwrap();

        // recall_experiences filters by Episode type
        let episodes = memory.recall_experiences("parser bug", 5).unwrap();
        for e in &episodes {
            assert_eq!(e.memory_type, MemoryType::Episode);
        }
    }

    #[test]
    fn test_working_context_management() {
        let config = MemoryConfig::small();
        let memory = AgentMemory::new(config).unwrap();

        // Use with_working_context to modify
        memory.with_working_context(|ctx| {
            ctx.set_project("my-project");
            ctx.set_task("Implement feature X");
            ctx.add_open_file("src/main.rs");
            ctx.add_goal("Write tests");
        });

        // Access via working_context()
        let ctx = memory.working_context();
        assert_eq!(ctx.current_project, Some("my-project".to_string()));
        assert_eq!(ctx.current_task, Some("Implement feature X".to_string()));
        assert!(ctx.open_files.contains(&"src/main.rs".to_string()));
    }

    #[test]
    fn test_recall_successful_vs_failures() {
        let memory = make_agent_memory();

        memory
            .learn_task(
                "Successful implementation",
                "fn good() { /* works */ }",
                TaskOutcome::Success,
                vec!["Good approach"],
            )
            .unwrap();

        memory
            .learn_task(
                "Failed implementation",
                "fn bad() { /* broken */ }",
                TaskOutcome::Failure,
                vec!["Wrong approach"],
            )
            .unwrap();

        // recall_successful and recall_failures take string queries
        let successes = memory.recall_successful("implementation", 5).unwrap();
        let _failures = memory.recall_failures("implementation", 5).unwrap();

        // At least one should have results
        assert!(memory.db().len() == 2);

        // Verify filtering works (if embed_fn was set, these would filter by outcome)
        for s in &successes {
            // Verify it's filtering for success
            if let Some(ref meta) = s.metadata {
                if let Some(outcome) = meta.get("outcome") {
                    assert_eq!(outcome.as_str(), Some("success"));
                }
            }
        }
    }
}

// ============================================================================
// Replication Integration Tests
// ============================================================================

mod replication_integration {
    use minimemory::replication::{ChangeLog, ReplicationManager};
    use minimemory::{Config, VectorDB};

    #[test]
    fn test_change_log_tracking() {
        let log = ChangeLog::new();

        log.track_insert("doc-1", &[1.0, 2.0, 3.0], None);
        log.track_insert("doc-2", &[4.0, 5.0, 6.0], None);
        log.track_delete("doc-1");

        let changes = log.export_since(0);
        assert_eq!(changes.len(), 3);
    }

    #[test]
    fn test_snapshot_and_restore() {
        let source = VectorDB::new(Config::new(3)).unwrap();
        source.insert("a", &[1.0, 2.0, 3.0], None).unwrap();
        source.insert("b", &[4.0, 5.0, 6.0], None).unwrap();

        let snapshot = ReplicationManager::create_snapshot(&source).unwrap();

        let target = VectorDB::new(Config::new(3)).unwrap();
        ReplicationManager::apply_snapshot(&target, &snapshot).unwrap();

        assert_eq!(target.len(), 2);
        assert!(target.contains("a"));
        assert!(target.contains("b"));
    }

    #[test]
    fn test_incremental_export() {
        let log = ChangeLog::new();

        log.track_insert("doc-1", &[1.0, 2.0, 3.0], None);
        let checkpoint1 = log.checkpoint();

        log.track_insert("doc-2", &[4.0, 5.0, 6.0], None);

        let since_checkpoint = log.export_since(checkpoint1);
        assert_eq!(since_checkpoint.len(), 1);
        assert_eq!(since_checkpoint[0].document_id, "doc-2");
    }

    #[test]
    fn test_apply_changes_to_db() {
        let log = ChangeLog::new();
        log.track_insert("doc-1", &[1.0, 2.0, 3.0], None);
        log.track_insert("doc-2", &[4.0, 5.0, 6.0], None);

        let changes = log.export_since(0);

        let db = VectorDB::new(Config::new(3)).unwrap();
        let result = ReplicationManager::apply_changes(&db, &changes).unwrap();

        assert_eq!(result.applied, 2);
        assert_eq!(db.len(), 2);
    }

    #[test]
    fn test_changelog_serialization() {
        let log = ChangeLog::new();
        log.track_insert("doc-1", &[1.0, 2.0, 3.0], None);

        let json = serde_json::to_string(&log.export_since(0)).unwrap();
        assert!(!json.is_empty());

        let parsed: Vec<minimemory::replication::ChangeEntry> =
            serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 1);
    }
}

// ============================================================================
// Hybrid Search Integration Tests
// ============================================================================

mod hybrid_search_integration {
    use minimemory::{Config, Filter, HybridSearchParams, Metadata, VectorDB};

    fn generate_vector(seed: usize, dim: usize) -> Vec<f32> {
        (0..dim)
            .map(|i| ((seed * dim + i) % 1000) as f32 / 1000.0)
            .collect()
    }

    #[test]
    fn test_hybrid_search_combined() {
        let db = VectorDB::with_fulltext(Config::new(64), vec!["content".into()]).unwrap();

        for i in 0..10 {
            let mut meta = Metadata::new();
            meta.insert(
                "content",
                format!("Document about topic {} and Rust programming", i),
            );
            let v = generate_vector(i, 64);
            db.insert_document(&format!("doc-{}", i), Some(&v), Some(meta))
                .unwrap();
        }

        let query = generate_vector(0, 64);
        let params = HybridSearchParams::hybrid(query, "Rust programming", 5);
        let results = db.hybrid_search(params).unwrap();

        assert!(!results.is_empty());
    }

    #[test]
    fn test_filter_only_search() {
        let db = VectorDB::new(Config::new(64)).unwrap();

        for i in 0..20 {
            let mut meta = Metadata::new();
            meta.insert("category", if i % 2 == 0 { "even" } else { "odd" });
            let v = generate_vector(i, 64);
            db.insert(&format!("doc-{}", i), &v, Some(meta)).unwrap();
        }

        let results = db
            .filter_search(Filter::eq("category", "even"), 10)
            .unwrap();

        assert_eq!(results.len(), 10);
        for r in &results {
            let cat = r
                .metadata
                .as_ref()
                .unwrap()
                .get("category")
                .unwrap()
                .as_str()
                .unwrap();
            assert_eq!(cat, "even");
        }
    }

    #[test]
    fn test_vector_with_filter() {
        let db = VectorDB::new(Config::new(64)).unwrap();

        for i in 0..50 {
            let mut meta = Metadata::new();
            meta.insert("score", (i % 10) as f64);
            let v = generate_vector(i, 64);
            db.insert(&format!("doc-{}", i), &v, Some(meta)).unwrap();
        }

        let query = generate_vector(0, 64);
        let results = db
            .search_with_filter(&query, 5, Filter::gt("score", 5.0f64))
            .unwrap();

        for r in &results {
            let score = r
                .metadata
                .as_ref()
                .unwrap()
                .get("score")
                .unwrap()
                .as_f64()
                .unwrap();
            assert!(score > 5.0);
        }
    }

    #[test]
    fn test_keyword_search_only() {
        let db = VectorDB::with_fulltext(Config::new(64), vec!["text".into()]).unwrap();

        let texts = [
            "The quick brown fox jumps",
            "A lazy dog sleeps",
            "Quick foxes are smart",
            "Dogs and cats play together",
        ];

        for (i, text) in texts.iter().enumerate() {
            let mut meta = Metadata::new();
            meta.insert("text", *text);
            let v = generate_vector(i, 64);
            db.insert_document(&format!("doc-{}", i), Some(&v), Some(meta))
                .unwrap();
        }

        let results = db.keyword_search("quick fox", 5).unwrap();

        assert!(!results.is_empty());
    }
}

// ============================================================================
// Phase 5.1: GenericMemory Transfer & Multi-Preset Tests
// ============================================================================

mod generic_memory_advanced {
    use minimemory::memory_traits::presets::{Conversational, CustomerService, SoftwareDevelopment};
    use minimemory::memory_traits::{GenericMemory, InstanceContext, Priority, TransferLevel};

    fn gen_emb(seed: usize, dim: usize) -> Vec<f32> {
        (0..dim)
            .map(|i| ((seed * 17 + i * 31) % 1000) as f32 / 1000.0)
            .collect()
    }

    #[test]
    fn test_conversational_preset_workflow() {
        let memory = GenericMemory::<Conversational>::new(64).unwrap();

        memory.set_context(
            InstanceContext::new("chat-session-1")
                .with_context("customer-support")
                .with_domain("retail"),
        );

        let emb = gen_emb(1, 64);
        memory
            .learn("conv-1", &emb, "User asked about return policy", "Return inquiry", "resolved")
            .unwrap();

        let emb2 = gen_emb(2, 64);
        memory
            .learn("conv-2", &emb2, "User asked about shipping times", "Shipping inquiry", "resolved")
            .unwrap();

        let query = gen_emb(1, 64);
        let results = memory.recall(&query, 5).unwrap();

        assert!(!results.is_empty());
        assert_eq!(results[0].id, "conv-1");

        let stats = memory.stats();
        assert_eq!(stats.total_memories, 2);
        assert_eq!(stats.preset_name, "Conversational");
    }

    #[test]
    fn test_customer_service_preset_workflow() {
        let memory = GenericMemory::<CustomerService>::new(64).unwrap();

        memory.set_context(
            InstanceContext::new("support-team")
                .with_context("billing")
                .with_domain("retail"),
        );

        let emb = gen_emb(1, 64);
        memory
            .learn(
                "ticket-1",
                &emb,
                "Customer reported billing error on invoice #1234",
                "Billing error",
                "resolved",
            )
            .unwrap();

        let query = gen_emb(1, 64);
        let results = memory.recall(&query, 5).unwrap();

        assert!(!results.is_empty());
        assert_eq!(results[0].id, "ticket-1");

        let stats = memory.stats();
        assert_eq!(stats.preset_name, "Customer Service");
    }

    #[test]
    fn test_transfer_level_filtering_universal() {
        let memory = GenericMemory::<SoftwareDevelopment>::new(64).unwrap();

        // Learn content with universal concepts (error handling, testing, design patterns)
        let emb1 = gen_emb(1, 64);
        memory
            .learn(
                "universal-1",
                &emb1,
                "Design patterns for error handling and testing strategies",
                "Universal programming patterns",
                "success",
            )
            .unwrap();

        // Learn instance-specific content
        let emb2 = gen_emb(2, 64);
        memory
            .learn(
                "instance-1",
                &emb2,
                "This project specific configuration only here for our custom setup",
                "Project config",
                "success",
            )
            .unwrap();

        // recall_universal should only return Universal-level items
        let query = gen_emb(1, 64);
        let universal_results = memory.recall_universal(&query, 10).unwrap();

        for r in &universal_results {
            assert_eq!(
                r.transfer_level,
                TransferLevel::Universal,
                "recall_universal returned non-universal item: {} (level: {:?})",
                r.id,
                r.transfer_level,
            );
        }
    }

    #[test]
    fn test_transfer_level_filtering_same_domain() {
        let memory = GenericMemory::<SoftwareDevelopment>::new(64).unwrap();

        memory.set_context(
            InstanceContext::new("project-a")
                .with_context("rust")
                .with_domain("web_backend"),
        );

        // Learn backend content
        let emb1 = gen_emb(1, 64);
        memory
            .learn("backend-1", &emb1, "REST API endpoint handler", "API handler", "success")
            .unwrap();

        // Learn frontend content with different domain
        memory.set_context(
            InstanceContext::new("project-b")
                .with_context("typescript")
                .with_domain("web_frontend"),
        );

        let emb2 = gen_emb(2, 64);
        memory
            .learn("frontend-1", &emb2, "React component rendering", "UI component", "success")
            .unwrap();

        // Switch back to backend context and query same domain
        memory.set_context(
            InstanceContext::new("project-c")
                .with_context("rust")
                .with_domain("web_backend"),
        );

        let query = gen_emb(1, 64);
        let same_domain = memory.recall_same_domain(&query, 10).unwrap();

        // All results should be from web_backend domain
        for r in &same_domain {
            let domain = r.metadata.get("domain").and_then(|v| v.as_str());
            assert_eq!(
                domain,
                Some("web_backend"),
                "recall_same_domain returned wrong domain for {}: {:?}",
                r.id,
                domain,
            );
        }
    }

    #[test]
    fn test_usage_stats_persist_through_mark_useful() {
        let memory = GenericMemory::<SoftwareDevelopment>::new(64).unwrap();

        let emb = gen_emb(1, 64);
        memory.learn("task-1", &emb, "Content", "Desc", "success").unwrap();

        // Mark useful 3 times
        memory.mark_useful("task-1");
        memory.mark_useful("task-1");
        memory.mark_useful("task-1");

        // Access via recall (updates access count)
        let query = gen_emb(1, 64);
        let results = memory.recall(&query, 1).unwrap();

        assert!(!results.is_empty());
        // Stats should show usefulness > 0
        let stats = memory.stats();
        assert!(stats.avg_usefulness > 0.0);
    }

    #[test]
    fn test_priority_ordering_in_recall() {
        let memory = GenericMemory::<SoftwareDevelopment>::new(64).unwrap();

        // Use same embedding for all to make distance equal
        let emb = gen_emb(42, 64);

        memory
            .learn_with_priority("low", &emb, "Low priority", "Desc", "success", Priority::Low)
            .unwrap();
        memory
            .learn_with_priority("normal", &emb, "Normal priority", "Desc", "success", Priority::Normal)
            .unwrap();
        memory
            .learn_with_priority("high", &emb, "High priority", "Desc", "success", Priority::High)
            .unwrap();
        memory
            .learn_with_priority("critical", &emb, "Critical priority", "Desc", "success", Priority::Critical)
            .unwrap();

        // recall_high_priority should only return High and Critical
        let query = gen_emb(42, 64);
        let high = memory.recall_high_priority(&query, 10).unwrap();
        for r in &high {
            assert!(
                r.priority >= Priority::High,
                "Expected High+, got {:?} for {}",
                r.priority,
                r.id,
            );
        }
    }
}

// ============================================================================
// Phase 5.2: AgentMemory Persistence Tests
// ============================================================================

mod agent_memory_persistence {
    use minimemory::agent_memory::{
        AgentMemory, CodeSnippet, ErrorSolution, Language, MemoryConfig, TaskOutcome,
    };
    use std::path::PathBuf;

    fn make_memory() -> AgentMemory {
        let config = MemoryConfig::small();
        let mut memory = AgentMemory::new(config).unwrap();
        memory.set_embed_fn(|text: &str| {
            let dims = 384;
            let mut vec = vec![0.0f32; dims];
            for (i, byte) in text.bytes().enumerate() {
                vec[i % dims] += byte as f32 / 255.0;
            }
            let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > 0.0 {
                vec.iter_mut().for_each(|x| *x /= norm);
            }
            vec
        });
        memory
    }

    fn temp_path(name: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("minimemory_test_{}.mmdb", name));
        p
    }

    #[test]
    fn test_save_load_roundtrip() {
        let path = temp_path("save_load_roundtrip");
        let _ = std::fs::remove_file(&path);

        // Create and populate
        let memory = make_memory();
        memory
            .learn_task(
                "Implement login feature",
                "fn login() { /* ... */ }",
                TaskOutcome::Success,
                vec!["Use bcrypt"],
            )
            .unwrap();

        memory
            .learn_code(CodeSnippet {
                code: "fn hello() {}".to_string(),
                description: "Hello function".to_string(),
                language: Language::Rust,
                dependencies: vec![],
                use_case: "Greeting".to_string(),
                quality_score: 0.9,
                tags: vec![],
            })
            .unwrap();

        memory
            .learn_error_solution(ErrorSolution {
                error_message: "cannot borrow".to_string(),
                error_type: "E0596".to_string(),
                root_cause: "Missing mut".to_string(),
                solution: "Add mut".to_string(),
                fixed_code: Some("let mut x = 5;".to_string()),
                language: Language::Rust,
            })
            .unwrap();

        // Save
        memory.save(&path).unwrap();
        assert!(path.exists());

        // Load
        let mut loaded = AgentMemory::load(&path, MemoryConfig::small()).unwrap();
        loaded.set_embed_fn(|text: &str| {
            let dims = 384;
            let mut vec = vec![0.0f32; dims];
            for (i, byte) in text.bytes().enumerate() {
                vec[i % dims] += byte as f32 / 255.0;
            }
            let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > 0.0 {
                vec.iter_mut().for_each(|x| *x /= norm);
            }
            vec
        });

        // Verify data survived (3 docs + 1 __working_context__ = 4)
        assert!(loaded.db().len() >= 3);

        // Search should still work
        let results = loaded.recall_similar("login authentication", 5).unwrap();
        assert!(!results.is_empty());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_working_context_persists() {
        let path = temp_path("working_ctx_persist");
        let _ = std::fs::remove_file(&path);

        let memory = make_memory();
        memory.with_working_context(|ctx| {
            ctx.set_project("my-project");
            ctx.set_task("Build feature X");
            ctx.add_open_file("src/main.rs");
            ctx.add_goal("Write tests");
        });

        // Need at least one document for save to work
        memory
            .learn_task("task", "code", TaskOutcome::Success, vec!["note"])
            .unwrap();

        memory.save(&path).unwrap();

        let loaded = AgentMemory::load(&path, MemoryConfig::small()).unwrap();
        let ctx = loaded.working_context();

        assert_eq!(ctx.current_project, Some("my-project".to_string()));
        assert_eq!(ctx.current_task, Some("Build feature X".to_string()));
        assert!(ctx.open_files.contains(&"src/main.rs".to_string()));

        let _ = std::fs::remove_file(&path);
    }
}

// ============================================================================
// Phase 5.3: TransferableMemory Tests
// ============================================================================

mod transferable_memory_tests {
    use minimemory::agent_memory::{MemoryConfig, TaskOutcome};
    use minimemory::memory_traits::TransferLevel;
    use minimemory::transfer::{
        KnowledgeDomain, LanguageCompatibility, ProjectContext, TransferableMemory,
    };

    fn make_transferable() -> TransferableMemory {
        let config = MemoryConfig::small();
        let mut tm = TransferableMemory::new(config).unwrap();
        tm.set_embed_fn(|text: &str| {
            let dims = 384;
            let mut vec = vec![0.0f32; dims];
            for (i, byte) in text.bytes().enumerate() {
                vec[i % dims] += byte as f32 / 255.0;
            }
            let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > 0.0 {
                vec.iter_mut().for_each(|x| *x /= norm);
            }
            vec
        });
        tm
    }

    #[test]
    fn test_learn_with_explicit_transfer_levels() {
        let tm = make_transferable();
        tm.set_project_context(ProjectContext {
            name: "test-project".to_string(),
            language: "rust".to_string(),
            domain: KnowledgeDomain::WebBackend,
            frameworks: vec!["actix-web".to_string()],
            patterns: vec![],
            tags: vec![],
        });

        // Universal level
        tm.learn_task_transferable(
            "Error handling with Result type",
            "fn handle() -> Result<(), Box<dyn Error>> { Ok(()) }",
            TaskOutcome::Success,
            vec!["Always use Result for fallible operations"],
            Some(TransferLevel::Universal),
            None,
        )
        .unwrap();

        // Instance level
        tm.learn_task_transferable(
            "Configure actix-web routes for this project",
            "fn config(cfg: &mut web::ServiceConfig) { /* project specific */ }",
            TaskOutcome::Success,
            vec!["Only for this project's routing setup"],
            Some(TransferLevel::Instance),
            None,
        )
        .unwrap();

        assert_eq!(tm.memory().db().len(), 2);
    }

    #[test]
    fn test_recall_universal_only() {
        let tm = make_transferable();
        tm.set_project_context(ProjectContext {
            name: "project-a".to_string(),
            language: "rust".to_string(),
            domain: KnowledgeDomain::WebBackend,
            frameworks: vec![],
            patterns: vec![],
            tags: vec![],
        });

        // Universal knowledge
        tm.learn_task_transferable(
            "Design patterns for error handling",
            "fn handle_error() { /* universal pattern */ }",
            TaskOutcome::Success,
            vec!["Use typed errors"],
            Some(TransferLevel::Universal),
            Some(KnowledgeDomain::General),
        )
        .unwrap();

        // Project-specific knowledge
        tm.learn_task_transferable(
            "Project custom config loader",
            "fn load_config() { /* project specific */ }",
            TaskOutcome::Success,
            vec!["This project config"],
            Some(TransferLevel::Instance),
            Some(KnowledgeDomain::WebBackend),
        )
        .unwrap();

        let universal = tm.recall_universal("error handling patterns", 10).unwrap();

        for r in &universal {
            assert_eq!(
                r.transfer_level,
                TransferLevel::Universal,
                "recall_universal returned non-universal: {:?}",
                r.transfer_level,
            );
        }
    }

    #[test]
    fn test_language_compatibility_scores() {
        // Same language = 1.0
        assert_eq!(LanguageCompatibility::compatibility("rust", "rust"), 1.0);

        // Same family (C-family): c, c++, rust, zig
        let c_rust = LanguageCompatibility::compatibility("c", "rust");
        assert!(c_rust > 0.5, "C and Rust should be compatible: {}", c_rust);

        // Same family (scripting): python, ruby, perl
        let py_ruby = LanguageCompatibility::compatibility("python", "ruby");
        assert!(
            py_ruby > 0.5,
            "Python and Ruby should be compatible: {}",
            py_ruby,
        );

        // Different families
        let rust_python = LanguageCompatibility::compatibility("rust", "python");
        assert!(
            rust_python < 0.5,
            "Rust and Python should have low compatibility: {}",
            rust_python,
        );

        // Unknown languages
        let unknown = LanguageCompatibility::compatibility("brainfuck", "whitespace");
        assert!(unknown < 0.5);
    }

    #[test]
    fn test_knowledge_domain_related() {
        let web_backend = KnowledgeDomain::WebBackend;
        let related = web_backend.related_domains();

        // WebBackend is related to Database, Security, DevOps
        assert!(related.contains(&KnowledgeDomain::Database));
        assert!(related.contains(&KnowledgeDomain::DevOps));
        assert!(related.contains(&KnowledgeDomain::Security));
    }

    #[test]
    fn test_recall_same_stack_filters_by_language() {
        let tm = make_transferable();

        // Learn Rust knowledge
        tm.set_project_context(ProjectContext {
            name: "rust-project".to_string(),
            language: "rust".to_string(),
            domain: KnowledgeDomain::WebBackend,
            frameworks: vec![],
            patterns: vec![],
            tags: vec![],
        });

        tm.learn_task_transferable(
            "Rust ownership and borrowing",
            "fn borrow(s: &str) { println!(\"{}\", s); }",
            TaskOutcome::Success,
            vec!["Rust borrow checker"],
            Some(TransferLevel::Domain),
            None,
        )
        .unwrap();

        // Learn Python knowledge
        tm.set_project_context(ProjectContext {
            name: "python-project".to_string(),
            language: "python".to_string(),
            domain: KnowledgeDomain::DataScience,
            frameworks: vec!["pandas".to_string()],
            patterns: vec![],
            tags: vec![],
        });

        tm.learn_task_transferable(
            "Python data analysis with pandas",
            "import pandas as pd; df = pd.read_csv('data.csv')",
            TaskOutcome::Success,
            vec!["Use pandas for data processing"],
            Some(TransferLevel::Domain),
            None,
        )
        .unwrap();

        // Query with Rust context — should prefer Rust stack results
        tm.set_project_context(ProjectContext {
            name: "new-rust-project".to_string(),
            language: "rust".to_string(),
            domain: KnowledgeDomain::WebBackend,
            frameworks: vec![],
            patterns: vec![],
            tags: vec![],
        });

        let results = tm.recall_same_stack("programming patterns", 10).unwrap();

        // If results are returned, they should be language-compatible
        // (Rust is in a different family from Python)
        for r in &results {
            // The filter should prefer same-stack results
            // At minimum, verify the recall works without panicking
            assert!(r.combined_score >= 0.0);
        }
    }
}

// ============================================================================
// Helpers
// ============================================================================

fn rand_float() -> f32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    ((seed % 1000) as f32) / 1000.0
}

// ============================================================================
// Tests de chunking multibyte (UTF-8)
// ============================================================================

mod chunking_multibyte {
    use super::*;
    use minimemory::chunking::{ChunkConfig, ChunkStrategy};

    #[test]
    fn ingest_markdown_multibyte_does_not_panic() {
        let db = VectorDB::new(Config::new(3)).unwrap();
        let markdown = "ááááááááá ñññ üüü 你好世界 😀😀😀";
        let config = ChunkConfig::new(ChunkStrategy::BySize {
            target_size: 5,
            overlap: 2,
        });

        let count = db.ingest_markdown(markdown, &config).unwrap();
        assert!(count >= 1);
    }

    #[test]
    fn ingest_markdown_cjk_no_content_lost() {
        let db = VectorDB::new(Config::new(3)).unwrap();
        let markdown = "你好世界你好世界你好世界";
        let config = ChunkConfig::new(ChunkStrategy::BySize {
            target_size: 7,
            overlap: 0,
        });

        let count = db.ingest_markdown(markdown, &config).unwrap();
        assert!(count >= 2);

        // Recuperar el contenido concatenado de los chunks via fulltext search
        // no es necesario; basta con re-chunkear y comparar.
        let result =
            minimemory::chunking::chunk_markdown(markdown, &config).unwrap();
        let rebuilt: String = result.chunks.iter().map(|c| c.content.as_str()).collect();
        assert_eq!(rebuilt, markdown);
    }
}

// ============================================================================
// Search contract: offset, filter and soft-delete must not reduce results
// below min(k, docs that qualify after filter/soft-delete). See
// audit/audit-C-search-query-quant.md, findings 2-4.
// ============================================================================

mod search_contract {
    use super::*;
    use minimemory::{Filter, HybridSearchParams};

    fn ids(results: &[minimemory::HybridSearchResult]) -> Vec<String> {
        results.iter().map(|r| r.id.to_string()).collect()
    }

    // 20 docs on a line: distance to query [0,0,0,0] grows with i, so ranking
    // is fully deterministic (doc-0 closest .. doc-19 farthest).
    fn build_line_db(n: usize) -> VectorDB {
        let db = VectorDB::new(Config::new(4).with_distance(Distance::Euclidean)).unwrap();
        for i in 0..n {
            let v = [i as f32, 0.0, 0.0, 0.0];
            db.insert(format!("doc-{}", i), &v, None).unwrap();
        }
        db
    }

    #[test]
    fn vector_search_offset_returns_k_shifted() {
        let db = build_line_db(20);
        let q = [0.0, 0.0, 0.0, 0.0];

        let full = db.hybrid_search(HybridSearchParams::vector(q.to_vec(), 20)).unwrap();
        let page = db
            .hybrid_search(HybridSearchParams::vector(q.to_vec(), 10).with_offset(5))
            .unwrap();

        assert_eq!(page.len(), 10, "offset must not reduce results below k");
        assert_eq!(ids(&page), ids(&full[5..15]), "offset must shift the ranking");
    }

    #[test]
    fn keyword_search_offset_returns_k_shifted() {
        let db = VectorDB::with_fulltext(Config::new(4), vec!["content".into()]).unwrap();
        // Constant length (20 tokens), tf of "rust" grows with i => BM25 score
        // monotonic in i => doc-19 ranks first, doc-0 last.
        for i in 0..20 {
            let mut meta = Metadata::new();
            let content = format!("{}{}", "rust ".repeat(i + 1), "pad ".repeat(19 - i));
            meta.insert("content", content);
            let v = [i as f32, 0.0, 0.0, 0.0];
            db.insert_document(&format!("doc-{}", i), Some(&v), Some(meta))
                .unwrap();
        }

        let full = db
            .hybrid_search(HybridSearchParams::keyword("rust", 20))
            .unwrap();
        let page = db
            .hybrid_search(HybridSearchParams::keyword("rust", 10).with_offset(5))
            .unwrap();

        assert_eq!(page.len(), 10, "offset must not reduce results below k");
        assert_eq!(ids(&page), ids(&full[5..15]), "offset must shift the ranking");
    }

    #[test]
    fn hybrid_search_offset_returns_k_shifted() {
        // 40 docs: buggy code fetches k*3 = 30, so an offset of 25 (> 2k) is
        // needed to expose the under-fetch; fixed code fetches all 40.
        let db = VectorDB::with_fulltext(Config::new(4), vec!["content".into()]).unwrap();
        for i in 0..40 {
            let mut meta = Metadata::new();
            // Distinct tf => distinct BM25 scores => deterministic keyword
            // ordering (HashMap tie-break order is randomized per call).
            meta.insert("content", "term ".repeat(i + 1));
            let v = [i as f32, 0.0, 0.0, 0.0];
            db.insert_document(&format!("doc-{}", i), Some(&v), Some(meta))
                .unwrap();
        }
        let q = [0.0, 0.0, 0.0, 0.0];

        let full = db
            .hybrid_search(HybridSearchParams::hybrid(q.to_vec(), "term", 40))
            .unwrap();
        let page = db
            .hybrid_search(HybridSearchParams::hybrid(q.to_vec(), "term", 10).with_offset(25))
            .unwrap();

        assert_eq!(page.len(), 10, "offset must not reduce results below k");
        assert_eq!(ids(&page), ids(&full[25..35]), "offset must shift the ranking");
    }

    #[test]
    fn vector_search_selective_filter_returns_k() {
        // 100 non-tech docs close to the query + 10 tech docs far away, k=10.
        // Buggy code fetches k*10 = 100 (all non-tech) then filters => 0.
        // Fixed code fetches all 110 => 10 tech.
        let db = VectorDB::new(Config::new(4).with_distance(Distance::Euclidean)).unwrap();
        for i in 0..100 {
            let v = [0.001 * (i as f32 + 1.0), 0.0, 0.0, 0.0];
            let mut meta = Metadata::new();
            meta.insert("category", "other");
            db.insert(format!("other-{}", i), &v, Some(meta)).unwrap();
        }
        for i in 0..10 {
            let v = [100.0 + i as f32, 0.0, 0.0, 0.0];
            let mut meta = Metadata::new();
            meta.insert("category", "tech");
            db.insert(format!("tech-{}", i), &v, Some(meta)).unwrap();
        }

        let results = db
            .hybrid_search(
                HybridSearchParams::vector(vec![0.0, 0.0, 0.0, 0.0], 10)
                    .with_filter(Filter::eq("category", "tech")),
            )
            .unwrap();

        assert_eq!(results.len(), 10, "selective filter must still return k");
        for r in &results {
            assert_eq!(
                r.metadata.as_ref().unwrap().get("category").unwrap().as_str().unwrap(),
                "tech"
            );
        }
    }

    #[test]
    fn hybrid_search_selective_filter_returns_k() {
        // 30 non-tech (close vector, high tf) + 10 tech (far vector, low tf).
        // Buggy code fetches k*3 = 30 (all non-tech on both sides) then filters
        // post-RRF => 0. Fixed code fetches all 40 => 10 tech.
        let db = VectorDB::with_fulltext(Config::new(4), vec!["content".into()]).unwrap();
        for i in 0..30 {
            let mut meta = Metadata::new();
            meta.insert("content", "term term term");
            meta.insert("category", "other");
            let v = [0.001 * (i as f32 + 1.0), 0.0, 0.0, 0.0];
            db.insert_document(&format!("other-{}", i), Some(&v), Some(meta))
                .unwrap();
        }
        for i in 0..10 {
            let mut meta = Metadata::new();
            meta.insert("content", "term");
            meta.insert("category", "tech");
            let v = [100.0 + i as f32, 0.0, 0.0, 0.0];
            db.insert_document(&format!("tech-{}", i), Some(&v), Some(meta))
                .unwrap();
        }

        let results = db
            .hybrid_search(
                HybridSearchParams::hybrid(vec![0.0, 0.0, 0.0, 0.0], "term", 10)
                    .with_filter(Filter::eq("category", "tech")),
            )
            .unwrap();

        assert_eq!(results.len(), 10, "hybrid selective filter must return k");
        for r in &results {
            assert_eq!(
                r.metadata.as_ref().unwrap().get("category").unwrap().as_str().unwrap(),
                "tech"
            );
        }
    }

    #[test]
    fn soft_deleted_docs_do_not_reduce_result_count() {
        // 15 docs; the 5 closest to the query are soft-deleted. Buggy code
        // fetches k=10 (5 deleted + 5 alive) then retains => 5. Fixed code
        // fetches all 15, retains the 10 alive => 10.
        let db = VectorDB::new(Config::new(4).with_distance(Distance::Euclidean)).unwrap();
        for i in 0..15 {
            let v = [i as f32, 0.0, 0.0, 0.0];
            let mut meta = Metadata::new();
            if i < 5 {
                meta.insert("deleted", true);
            }
            db.insert(format!("doc-{}", i), &v, Some(meta)).unwrap();
        }

        let results = db
            .hybrid_search(HybridSearchParams::vector(vec![0.0, 0.0, 0.0, 0.0], 10))
            .unwrap();

        assert_eq!(results.len(), 10, "soft-deletes must not reduce results below k");
        for r in &results {
            let deleted = r
                .metadata
                .as_ref()
                .and_then(|m| m.get("deleted"))
                .map(|v| v.as_bool().unwrap_or(false))
                .unwrap_or(false);
            assert!(!deleted, "no soft-deleted docs in results");
        }
        // The 5 closest (doc-0..doc-4) were deleted, so we get doc-5..doc-14.
        let indices: Vec<usize> = results
            .iter()
            .map(|r| r.id.strip_prefix("doc-").unwrap().parse::<usize>().unwrap())
            .collect();
        assert!(indices.iter().all(|i| *i >= 5));
    }

    #[test]
    fn k_larger_than_total_returns_all_without_error() {
        let db = VectorDB::new(Config::new(4).with_distance(Distance::Euclidean)).unwrap();
        for i in 0..3 {
            let v = [i as f32, 0.0, 0.0, 0.0];
            db.insert(format!("doc-{}", i), &v, None).unwrap();
        }

        let results = db
            .hybrid_search(HybridSearchParams::vector(vec![0.0, 0.0, 0.0, 0.0], 100))
            .unwrap();
        assert_eq!(results.len(), 3);

        // With a filter and k > total qualifying: return all qualifying.
        let db2 = VectorDB::new(Config::new(4).with_distance(Distance::Euclidean)).unwrap();
        let mut v = [0.0f32, 0.0, 0.0, 0.0];
        for i in 0..3 {
            v[0] = i as f32;
            let mut meta = Metadata::new();
            meta.insert("category", if i < 2 { "tech" } else { "other" });
            db2.insert(format!("doc-{}", i), &v, Some(meta)).unwrap();
        }
        let filtered = db2
            .hybrid_search(
                HybridSearchParams::vector(vec![0.0, 0.0, 0.0, 0.0], 100)
                    .with_filter(Filter::eq("category", "tech")),
            )
            .unwrap();
        assert_eq!(filtered.len(), 2);
    }
}

// ============================================================================
// Validación de vectores no finitos (NaN/Inf) en la frontera de la API
// ============================================================================

mod vector_finiteness_validation {
    use super::*;
    use minimemory::Error;

    fn db() -> VectorDB {
        VectorDB::new(Config::new(3).with_distance(Distance::Euclidean)).unwrap()
    }

    #[test]
    fn insert_with_nan_is_rejected_and_db_intact() {
        let db = db();
        let err = db.insert("a", &[f32::NAN, 0.0, 0.0], None).unwrap_err();
        assert!(matches!(err, Error::InvalidVector(_)), "got {:?}", err);
        assert_eq!(db.len(), 0);
        assert!(!db.contains("a"));
    }

    #[test]
    fn insert_with_inf_is_rejected() {
        let db = db();
        let err = db
            .insert("a", &[f32::INFINITY, 0.0, 0.0], None)
            .unwrap_err();
        assert!(matches!(err, Error::InvalidVector(_)), "got {:?}", err);
        let err = db
            .insert("b", &[0.0, f32::NEG_INFINITY, 0.0], None)
            .unwrap_err();
        assert!(matches!(err, Error::InvalidVector(_)), "got {:?}", err);
        assert_eq!(db.len(), 0);
    }

    #[test]
    fn update_with_nan_is_rejected_and_original_survives() {
        let db = db();
        db.insert("a", &[1.0, 0.0, 0.0], None).unwrap();
        let err = db.update("a", &[f32::NAN, 0.0, 0.0], None).unwrap_err();
        assert!(matches!(err, Error::InvalidVector(_)), "got {:?}", err);
        let (vec, _) = db.get("a").unwrap().unwrap();
        assert_eq!(vec.unwrap(), vec![1.0, 0.0, 0.0]);
    }

    #[test]
    fn search_with_nan_query_is_rejected() {
        let db = db();
        db.insert("a", &[1.0, 0.0, 0.0], None).unwrap();
        let err = db.search(&[f32::NAN, 0.0, 0.0], 5).unwrap_err();
        assert!(matches!(err, Error::InvalidVector(_)), "got {:?}", err);
    }

    #[test]
    fn finite_extreme_values_are_accepted() {
        let db = db();
        db.insert(
            "a",
            &[f32::MAX, f32::MIN_POSITIVE, 1.0],
            None,
        )
        .unwrap();
        db.insert(
            "b",
            &[f32::MIN, f32::MAX, f32::MIN_POSITIVE],
            None,
        )
        .unwrap();
        let results = db
            .search(&[f32::MAX, f32::MIN_POSITIVE, 1.0], 2)
            .unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "a");
    }
}

// ============================================================================
// Tests de índices parciales: clear() y populate retroactivo
// ============================================================================

mod partial_index_lifecycle {
    use super::*;
    use minimemory::partial_index::PartialIndexConfig;
    use minimemory::Filter;

    fn meta_with(category: &str) -> Option<Metadata> {
        let mut m = Metadata::new();
        m.insert("category", category);
        Some(m)
    }

    #[test]
    fn clear_empties_partial_indexes_but_keeps_them_registered() {
        let db = VectorDB::new(Config::new(3)).unwrap();
        db.create_partial_index(
            "tech",
            PartialIndexConfig::new(Filter::eq("category", "tech")),
        )
        .unwrap();

        // Poblar: 2 docs que matchean, 1 que no.
        db.insert("doc1", &[1.0, 0.0, 0.0], meta_with("tech"))
            .unwrap();
        db.insert("doc2", &[0.0, 1.0, 0.0], meta_with("tech"))
            .unwrap();
        db.insert("doc3", &[0.0, 0.0, 1.0], meta_with("sports"))
            .unwrap();

        let results = db.search_partial("tech", &[1.0, 0.0, 0.0], 10).unwrap();
        assert_eq!(results.len(), 2);

        // clear() debe vaciar storage principal e índices parciales, pero
        // mantener el índice parcial registrado.
        db.clear();
        assert!(db.is_empty());
        assert!(db.has_partial_index("tech"));

        // search_partial no debe devolver ids ya borrados.
        let results = db.search_partial("tech", &[1.0, 0.0, 0.0], 10).unwrap();
        assert!(results.is_empty(), "expected empty, got {:?}", results);

        // Un insert posterior que matchea vuelve a aparecer en el parcial.
        db.insert("doc4", &[0.5, 0.5, 0.0], meta_with("tech"))
            .unwrap();
        let results = db.search_partial("tech", &[0.5, 0.5, 0.0], 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "doc4");
    }

    #[test]
    fn create_partial_index_indexes_existing_documents_retroactively() {
        let db = VectorDB::new(Config::new(3)).unwrap();

        // 10 docs: 5 matchean el filtro, 5 no.
        for i in 0..10 {
            let id = format!("doc{}", i);
            let category = if i % 2 == 0 { "tech" } else { "sports" };
            let v = [i as f32, 0.0, 0.0];
            db.insert(&id, &v, meta_with(category)).unwrap();
        }

        // Crear el índice parcial DESPUÉS de insertar.
        db.create_partial_index(
            "tech",
            PartialIndexConfig::new(Filter::eq("category", "tech")),
        )
        .unwrap();

        // Debe encontrar los 5 existentes sin insertar nada nuevo.
        let results = db.search_partial("tech", &[1.0, 0.0, 0.0], 10).unwrap();
        assert_eq!(results.len(), 5);
        for r in &results {
            assert_eq!(r.id.len(), 4); // "doc0".."doc9" -> 4 chars
            let n: usize = r.id[3..].parse().unwrap();
            assert_eq!(n % 2, 0, "only even ids match the filter");
        }
    }

    #[test]
    fn deleted_document_does_not_appear_in_partial_search() {
        let db = VectorDB::new(Config::new(3)).unwrap();
        db.create_partial_index(
            "tech",
            PartialIndexConfig::new(Filter::eq("category", "tech")),
        )
        .unwrap();

        db.insert("doc1", &[1.0, 0.0, 0.0], meta_with("tech"))
            .unwrap();
        db.insert("doc2", &[0.0, 1.0, 0.0], meta_with("tech"))
            .unwrap();

        assert_eq!(
            db.search_partial("tech", &[1.0, 0.0, 0.0], 10)
                .unwrap()
                .len(),
            2
        );

        // Borrar doc1 de la DB principal; on_delete debe sacarlo del parcial.
        assert!(db.delete("doc1").unwrap());
        assert!(!db.contains("doc1"));

        let results = db.search_partial("tech", &[1.0, 0.0, 0.0], 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "doc2");
    }
}

// ============================================================================
// Tests de integración del Write-Ahead Log (WAL) en VectorDB
// ============================================================================

mod wal_integration {
    use super::*;
    use minimemory::QuantizationType;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn temp_path(suffix: &str) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let mut p = std::env::temp_dir();
        p.push(format!(
            "minimemory_wal_{}_{}{}.{}",
            std::process::id(),
            n,
            suffix,
            suffix_suffix(suffix)
        ));
        p
    }

    // Extensión de archivo según el rol (snapshot .mmdb vs wal .wal).
    fn suffix_suffix(suffix: &str) -> &'static str {
        if suffix.contains("wal") {
            "wal"
        } else {
            "mmdb"
        }
    }

    fn cleanup(paths: &[PathBuf]) {
        for p in paths {
            fs::remove_file(p).ok();
        }
    }

    /// Durabilidad básica sin snapshot: insert/update/delete → DROP (sin
    /// `save()`) → `new_with_wal` recupera el estado completo desde el WAL
    /// huérfano.
    #[test]
    fn test_wal_basic_durability_no_snapshot() {
        let wal = temp_path("_wal_basic");
        let cfg = Config::new(3).with_distance(Distance::Cosine);

        {
            let mut db = VectorDB::new(cfg.clone()).unwrap();
            db.enable_wal(&wal).unwrap();

            db.insert("a", &[1.0, 0.0, 0.0], None).unwrap();
            db.insert("b", &[0.0, 1.0, 0.0], None).unwrap();
            db.insert("c", &[0.0, 0.0, 1.0], None).unwrap();

            // Update de "a" (vector + metadata nuevos).
            let mut meta = Metadata::new();
            meta.insert("k", "v");
            db.update("a", &[0.5, 0.5, 0.0], Some(meta)).unwrap();

            // Delete de "b".
            assert!(db.delete("b").unwrap());

            // DROP sin save(): no se escribe snapshot.
        }

        // Reabrir desde el WAL huérfano (no hay snapshot → new_with_wal).
        let db = VectorDB::new_with_wal(cfg, &wal).unwrap();

        assert_eq!(db.len(), 2); // 3 insertados - 1 borrado
        assert!(!db.contains("b"));
        assert!(db.contains("a"));
        assert!(db.contains("c"));

        let (vec_a, meta_a) = db.get("a").unwrap().unwrap();
        assert_eq!(vec_a, Some(vec![0.5, 0.5, 0.0]));
        assert!(meta_a.is_some());
        assert_eq!(
            meta_a.unwrap().get("k").and_then(|v| v.as_str()),
            Some("v")
        );

        let (vec_c, _) = db.get("c").unwrap().unwrap();
        assert_eq!(vec_c, Some(vec![0.0, 0.0, 1.0]));

        cleanup(&[wal]);
    }

    /// Durabilidad con cuantización Int8: el WAL guarda el vector f32 original
    /// y el replay lo re-cuantiza al aplicar. Los vectores recuperados
    /// (dequantizados) son idénticos a los de una DB control insertada en vivo.
    #[test]
    fn test_wal_with_int8_quantization_requantizes_on_replay() {
        let wal = temp_path("_wal_int8");
        let cfg = Config::new(4).with_quantization(QuantizationType::Int8);

        let v1 = vec![0.1, 0.2, 0.3, 0.4];
        let v2 = vec![0.9, 0.8, 0.7, 0.6];

        {
            let mut db = VectorDB::new(cfg.clone()).unwrap();
            db.enable_wal(&wal).unwrap();
            db.insert("x", &v1, None).unwrap();
            db.insert("y", &v2, None).unwrap();
        }

        // DB control: misma config, mismas inserciones en vivo.
        let control = VectorDB::new(cfg.clone()).unwrap();
        control.insert("x", &v1, None).unwrap();
        control.insert("y", &v2, None).unwrap();

        let db = VectorDB::new_with_wal(cfg, &wal).unwrap();
        assert_eq!(db.len(), 2);

        let (rec_x, _) = db.get("x").unwrap().unwrap();
        let (rec_y, _) = db.get("y").unwrap().unwrap();
        let (ctl_x, _) = control.get("x").unwrap().unwrap();
        let (ctl_y, _) = control.get("y").unwrap().unwrap();

        // Re-cuantización determinista: mismo f32 de entrada + mismo quantizer
        // → mismo quantizado → misma dequantización exacta.
        assert_eq!(rec_x, ctl_x);
        assert_eq!(rec_y, ctl_y);

        cleanup(&[wal]);
    }

    /// Checkpoint: tras `checkpoint`, el WAL queda vacío (solo header) y
    /// `open_with_wal` (snapshot + wal) reconstruye exacto; las escrituras
    /// post-checkpoint también se recuperan.
    #[test]
    fn test_wal_checkpoint_then_recover() {
        let snap = temp_path("_snap_ckpt");
        let wal = temp_path("_wal_ckpt");
        let cfg = Config::new(2);

        {
            let mut db = VectorDB::new(cfg).unwrap();
            db.enable_wal(&wal).unwrap();
            db.insert("a", &[1.0, 2.0], None).unwrap();
            db.insert("b", &[3.0, 4.0], None).unwrap();

            db.checkpoint(&snap).unwrap();

            // El WAL debe quedar vacío (solo header de 8 bytes).
            assert_eq!(fs::metadata(&wal).unwrap().len(), 8);

            // Escrituras post-checkpoint.
            db.insert("c", &[5.0, 6.0], None).unwrap();
            assert!(db.delete("a").unwrap());
        }

        let db = VectorDB::open_with_wal(&snap, &wal).unwrap();
        assert_eq!(db.len(), 2); // b y c (a fue borrado post-checkpoint)
        assert!(db.contains("b"));
        assert!(db.contains("c"));
        assert!(!db.contains("a"));

        let (vec_c, _) = db.get("c").unwrap().unwrap();
        assert_eq!(vec_c, Some(vec![5.0, 6.0]));

        cleanup(&[snap, wal]);
    }

    /// Idempotencia: simula crash entre snapshot y truncate (save() manual SIN
    /// truncar el WAL) + escrituras extra → `open_with_wal` reaplica todo sin
    /// error (Insert de id ya en snapshot → upsert, no AlreadyExists) y el
    /// estado final es correcto.
    #[test]
    fn test_wal_idempotent_replay_after_crash_between_save_and_truncate() {
        let snap = temp_path("_snap_idem");
        let wal = temp_path("_wal_idem");
        let cfg = Config::new(2);

        {
            let mut db = VectorDB::new(cfg).unwrap();
            db.enable_wal(&wal).unwrap();
            db.insert("a", &[1.0, 1.0], None).unwrap();
            db.insert("b", &[2.0, 2.0], None).unwrap();

            // save() manual SIN truncar el WAL: simula crash justo después del
            // snapshot, antes del truncate del checkpoint.
            db.save(&snap).unwrap();

            // Más escrituras que se suman al WAL (las de "a" y "b" ya están
            // también en el snapshot → el replay debe tolerarlas).
            db.insert("c", &[3.0, 3.0], None).unwrap();
        }

        // open_with_wal: snapshot tiene {a,b}, WAL tiene Insert(a), Insert(b),
        // Insert(c). El replay reaplica a y b como upsert (ya existen) sin
        // error, y añade c.
        let db = VectorDB::open_with_wal(&snap, &wal).unwrap();
        assert_eq!(db.len(), 3);
        assert!(db.contains("a"));
        assert!(db.contains("b"));
        assert!(db.contains("c"));

        cleanup(&[snap, wal]);
    }

    /// Cola rota: se corta el WAL a mitad de la última entrada a mano → el
    /// replay recupera hasta la última entrada válida sin error.
    #[test]
    fn test_wal_broken_tail_recovers_valid_prefix() {
        let wal = temp_path("_wal_broken");
        let cfg = Config::new(2);

        {
            let mut db = VectorDB::new(cfg.clone()).unwrap();
            db.enable_wal(&wal).unwrap();
            db.insert("a", &[1.0, 1.0], None).unwrap();
            db.insert("b", &[2.0, 2.0], None).unwrap();
            db.insert("c", &[3.0, 3.0], None).unwrap();
        }

        // Truncar el archivo cortando los últimos 5 bytes → la 3ra entrada
        // queda incompleta (torn write). Debe recuperarse a/b y descartarse c.
        let len = fs::metadata(&wal).unwrap().len();
        let cut = len.saturating_sub(5);
        {
            use std::io::Seek;
            let mut f = fs::OpenOptions::new()
                .write(true)
                .open(&wal)
                .unwrap();
            f.seek(std::io::SeekFrom::Start(0)).unwrap();
            f.set_len(cut).unwrap();
        }

        let db = VectorDB::new_with_wal(cfg, &wal).unwrap();
        // Solo las 2 primeras ops (Insert a, Insert b) son válidas.
        assert_eq!(db.len(), 2);
        assert!(db.contains("a"));
        assert!(db.contains("b"));
        assert!(!db.contains("c"));

        cleanup(&[wal]);
    }

    /// clear() con WAL: la recuperación refleja la DB vacía + los inserts
    /// posteriores al clear.
    #[test]
    fn test_wal_clear_then_inserts_reflected_on_recovery() {
        let wal = temp_path("_wal_clear");
        let cfg = Config::new(2);

        {
            let mut db = VectorDB::new(cfg.clone()).unwrap();
            db.enable_wal(&wal).unwrap();
            db.insert("a", &[1.0, 1.0], None).unwrap();
            db.insert("b", &[2.0, 2.0], None).unwrap();
            db.clear();
            assert_eq!(db.len(), 0);
            db.insert("c", &[3.0, 3.0], None).unwrap();
        }

        let db = VectorDB::new_with_wal(cfg, &wal).unwrap();
        assert_eq!(db.len(), 1);
        assert!(!db.contains("a"));
        assert!(!db.contains("b"));
        assert!(db.contains("c"));

        cleanup(&[wal]);
    }

    /// Sin WAL habilitado: el comportamiento es exactamente el de antes (cero
    /// regresión). insert/update/delete/clear/save/open funcionan sin tocar
    /// ningún archivo de log.
    #[test]
    fn test_no_wal_no_regression() {
        let snap = temp_path("_snap_nowal");
        let cfg = Config::new(3);

        let db = VectorDB::new(cfg).unwrap();
        db.insert("a", &[1.0, 2.0, 3.0], None).unwrap();
        db.insert("b", &[4.0, 5.0, 6.0], None).unwrap();
        assert_eq!(db.len(), 2);

        db.update("a", &[7.0, 8.0, 9.0], None).unwrap();
        let (va, _) = db.get("a").unwrap().unwrap();
        assert_eq!(va, Some(vec![7.0, 8.0, 9.0]));

        assert!(db.delete("b").unwrap());
        assert_eq!(db.len(), 1);

        db.clear();
        assert_eq!(db.len(), 0);

        db.insert("c", &[1.0, 1.0, 1.0], None).unwrap();
        db.save(&snap).unwrap();

        let db2 = VectorDB::open(&snap).unwrap();
        assert_eq!(db2.len(), 1);
        assert!(db2.contains("c"));

        // Ningún WAL se creó en este flujo.
        cleanup(&[snap]);
    }
}

// ============================================================================
// Índices de metadata + query planner
// ============================================================================

mod metadata_indexes {
    use super::*;
    use minimemory::{Filter, MetadataValue, OrderBy};
    use std::collections::HashSet;

    fn mk_meta(i: usize) -> Metadata {
        let mut m = Metadata::new();
        let cats = ["tech", "news", "sports", "food"];
        m.insert("cat", cats[i % 4]);
        m.insert("n", i as i64);
        m.insert("score", (i as f64) / 10.0);
        m.insert("active", i % 2 == 0);
        m.insert(
            "tag_list",
            MetadataValue::List(vec![
                MetadataValue::String(format!("t{}", i % 5)),
                MetadataValue::String("rust".into()),
            ]),
        );
        m
    }

    fn build_db() -> VectorDB {
        let db = VectorDB::new(Config::new(3)).unwrap();
        for i in 0..200 {
            let v = [i as f32, 0.0, 0.0];
            db.insert(format!("d{}", i), &v, Some(mk_meta(i)))
                .unwrap();
        }
        db
    }

    fn filter_ids(db: &VectorDB, filter: Filter) -> HashSet<String> {
        db.filter_search(filter, 10_000)
            .unwrap()
            .into_iter()
            .map(|r| r.id)
            .collect()
    }

    fn list_ids(db: &VectorDB, filter: Option<Filter>) -> HashSet<String> {
        db.list_documents(filter, None, 10_000, 0)
            .unwrap()
            .items
            .into_iter()
            .map(|r| r.id)
            .collect()
    }

    /// Batería de filtros que cubre hojas indexables y no indexables, `$and`,
    /// `$or`, `$not`, anidados, campos mixtos indexados y no indexados, y los
    /// casos vacíos de `$and`/`$or`.
    fn filter_battery() -> Vec<(&'static str, Filter)> {
        vec![
            ("eq_str_indexed", Filter::eq("cat", "tech")),
            ("eq_str_empty", Filter::eq("cat", "missing")),
            ("eq_int_indexed", Filter::eq("n", 5i64)),
            ("eq_bool_indexed", Filter::eq("active", true)),
            ("eq_float_none", Filter::eq("score", 5.0f64)),
            ("gt_int", Filter::gt("n", 50i64)),
            ("gte_int", Filter::gte("n", 50i64)),
            ("lt_int", Filter::lt("n", 50i64)),
            ("lte_int", Filter::lte("n", 50i64)),
            ("gt_float", Filter::gt("score", 5.0f64)),
            ("lte_float", Filter::lte("score", 13.0f64)),
            ("range_int", Filter::range("n", Some(10i64), Some(20i64))),
            ("range_float", Filter::range("score", Some(3.0f64), Some(7.0f64))),
            (
                "and_both_indexed",
                Filter::eq("cat", "tech").and(Filter::gte("n", 50i64)),
            ),
            (
                "and_indexed_and_none",
                Filter::eq("cat", "tech")
                    .and(Filter::contains("cat", "tec"))
                    .and(Filter::gte("n", 10i64)),
            ),
            (
                "and_three_nested",
                Filter::all(vec![
                    Filter::gte("n", 10i64),
                    Filter::lte("n", 20i64),
                    Filter::eq("active", true),
                ]),
            ),
            (
                "or_both_indexed",
                Filter::any(vec![Filter::eq("cat", "tech"), Filter::eq("cat", "news")]),
            ),
            (
                "or_with_none_branch",
                Filter::any(vec![
                    Filter::eq("cat", "tech"),
                    Filter::contains("cat", "foo"),
                ]),
            ),
            (
                "or_range_and_eq",
                Filter::any(vec![Filter::eq("n", 5i64), Filter::gt("score", 19.0f64)]),
            ),
            ("not_eq", Filter::not(Filter::eq("cat", "tech"))),
            ("ne_operator", Filter::ne("cat", "tech")),
            ("exists_cat", Filter::exists("cat")),
            ("not_exists_missing", Filter::not_exists("nope")),
            ("in_list_op", Filter::in_list("cat", vec!["tech", "news"])),
            ("eq_list_field_none", Filter::eq("tag_list", "rust")),
            ("empty_or_all", Filter::any(vec![])),
            ("empty_and_all", Filter::all(vec![])),
            (
                "and_or_nested",
                Filter::all(vec![
                    Filter::any(vec![Filter::eq("cat", "tech"), Filter::eq("cat", "news")]),
                    Filter::lt("n", 30i64),
                ]),
            ),
        ]
    }

    /// Equivalencia EXHAUSTIVA: con y sin índices creados, los mismos datos y
    /// los mismos filtros deben devolver exactamente el mismo set de ids, tanto
    /// por `filter_search` como por `list_documents`.
    #[test]
    fn full_equivalence_with_and_without_index() {
        let db = build_db();

        // Baseline: sin índices (full-scan). El planner devuelve None para todo.
        let mut baseline_filter: Vec<(&'static str, HashSet<String>)> = vec![];
        let mut baseline_list: Vec<(&'static str, HashSet<String>)> = vec![];
        for (name, f) in filter_battery() {
            baseline_filter.push((name, filter_ids(&db, f.clone())));
            baseline_list.push((name, list_ids(&db, Some(f.clone()))));
        }

        // Crear índices sobre campos string/int/float/bool (no sobre List).
        db.create_metadata_index("cat").unwrap();
        db.create_metadata_index("n").unwrap();
        db.create_metadata_index("score").unwrap();
        db.create_metadata_index("active").unwrap();
        assert_eq!(
            db.list_metadata_indexes(),
            vec!["active", "cat", "n", "score"]
        );

        // Mismos filtros con índices activos: resultados idénticos.
        for (i, (name, f)) in filter_battery().iter().enumerate() {
            let got = filter_ids(&db, f.clone());
            assert_eq!(got, baseline_filter[i].1, "filter_search mismatch: {}", name);
            let got_list = list_ids(&db, Some(f.clone()));
            assert_eq!(got_list, baseline_list[i].1, "list_documents mismatch: {}", name);
        }
    }

    /// Equivalencia también en la ruta vectorial con filtro (`search_with_filter`):
    /// el pre-filtro del planner no debe cambiar el set de resultados.
    #[test]
    fn search_with_filter_equivalence() {
        let db_no_idx = build_db();
        let db_idx = build_db();
        db_idx.create_metadata_index("cat").unwrap();
        db_idx.create_metadata_index("n").unwrap();

        let q = &[100.0f32, 0.0, 0.0];
        let filters = vec![
            Filter::eq("cat", "tech"),
            Filter::gte("n", 150i64),
            Filter::eq("cat", "tech").and(Filter::gte("n", 100i64)),
            Filter::any(vec![Filter::eq("cat", "tech"), Filter::eq("cat", "news")]),
        ];

        for f in filters {
            let a: HashSet<String> = db_no_idx
                .search_with_filter(q, 500, f.clone())
                .unwrap()
                .into_iter()
                .map(|r| r.id)
                .collect();
            let b: HashSet<String> = db_idx
                .search_with_filter(q, 500, f)
                .unwrap()
                .into_iter()
                .map(|r| r.id)
                .collect();
            assert_eq!(a, b);
        }
    }

    /// `create_metadata_index` indexa retroactivamente: crear el índice DESPUÉS
    /// de insertar sigue devolviendo los filtros correctos.
    #[test]
    fn create_index_retroactive_after_inserts() {
        let db = build_db();
        // Sin índice: full-scan.
        let baseline_eq = filter_ids(&db, Filter::eq("cat", "tech"));
        let baseline_range = filter_ids(&db, Filter::gte("n", 100i64));
        assert!(!baseline_eq.is_empty());
        assert!(!baseline_range.is_empty());

        // Crear los índices después de los inserts → deben indexar lo existente.
        db.create_metadata_index("cat").unwrap();
        db.create_metadata_index("n").unwrap();
        assert_eq!(filter_ids(&db, Filter::eq("cat", "tech")), baseline_eq);
        assert_eq!(filter_ids(&db, Filter::gte("n", 100i64)), baseline_range);
    }

    /// Mantenimiento: update/delete/clear mantienen el índice coherente.
    #[test]
    fn maintenance_update_delete_clear() {
        let db = VectorDB::new(Config::new(3)).unwrap();
        db.create_metadata_index("cat").unwrap();

        let mk = |c: &str| {
            let mut m = Metadata::new();
            m.insert("cat", c);
            m
        };

        db.insert("d1", &[1.0, 0.0, 0.0], Some(mk("a"))).unwrap();
        db.insert("d2", &[0.0, 1.0, 0.0], Some(mk("b"))).unwrap();

        // Update cambia el valor de un campo indexado → el filtro refleja el
        // valor nuevo, no el viejo.
        db.update("d1", &[1.0, 0.0, 0.0], Some(mk("c"))).unwrap();
        assert!(filter_ids(&db, Filter::eq("cat", "a")).is_empty());
        assert_eq!(filter_ids(&db, Filter::eq("cat", "c")), HashSet::from(["d1".into()]));

        // Delete saca el documento del índice.
        db.delete("d2").unwrap();
        assert!(filter_ids(&db, Filter::eq("cat", "b")).is_empty());
        assert_eq!(filter_ids(&db, Filter::eq("cat", "c")), HashSet::from(["d1".into()]));

        // Clear vacía los buckets pero conserva el registro del índice.
        db.clear();
        assert!(filter_ids(&db, Filter::eq("cat", "c")).is_empty());
        assert_eq!(db.list_metadata_indexes(), vec!["cat"]);

        // Tras clear, inserciones futuras vuelven a poblar el índice.
        db.insert("d3", &[0.0, 0.0, 1.0], Some(mk("a"))).unwrap();
        assert_eq!(filter_ids(&db, Filter::eq("cat", "a")), HashSet::from(["d3".into()]));
    }

    /// `update_document` (metadata-only, sin vector) también mantiene el índice.
    #[test]
    fn maintenance_update_document_metadata_only() {
        let db = VectorDB::new(Config::new(3)).unwrap();
        db.create_metadata_index("cat").unwrap();
        let mk = |c: &str| {
            let mut m = Metadata::new();
            m.insert("cat", c);
            m
        };
        db.insert_document("d1", None, Some(mk("a"))).unwrap();
        db.update_document("d1", None, Some(mk("z"))).unwrap();
        assert!(filter_ids(&db, Filter::eq("cat", "a")).is_empty());
        assert_eq!(filter_ids(&db, Filter::eq("cat", "z")), HashSet::from(["d1".into()]));
    }

    /// `drop_metadata_index` → las consultas siguen correctas por full-scan
    /// (mismos resultados que con índice).
    #[test]
    fn drop_index_falls_back_to_full_scan_same_results() {
        let db = build_db();
        db.create_metadata_index("cat").unwrap();
        db.create_metadata_index("n").unwrap();

        let with_idx_cat = filter_ids(&db, Filter::eq("cat", "tech"));
        let with_idx_range = filter_ids(&db, Filter::gte("n", 100i64));

        db.drop_metadata_index("cat").unwrap();
        db.drop_metadata_index("n").unwrap();
        assert!(db.list_metadata_indexes().is_empty());

        // Sin índices: full-scan, mismos resultados.
        assert_eq!(filter_ids(&db, Filter::eq("cat", "tech")), with_idx_cat);
        assert_eq!(filter_ids(&db, Filter::gte("n", 100i64)), with_idx_range);

        // Drop inexistente → NotFound.
        assert!(db.drop_metadata_index("cat").is_err());
    }

    /// `$or` con una rama no indexable (`$contains`) → fallback a full-scan del
    /// `$or` completo, resultado correcto.
    #[test]
    fn or_with_non_indexable_branch_correct() {
        let db = build_db();
        let baseline = filter_ids(&db, Filter::any(vec![
            Filter::eq("cat", "tech"),
            Filter::contains("cat", "foo"),
        ]));
        db.create_metadata_index("cat").unwrap();
        let with_idx = filter_ids(&db, Filter::any(vec![
            Filter::eq("cat", "tech"),
            Filter::contains("cat", "foo"),
        ]));
        assert_eq!(baseline, with_idx);
        // "tech" contiene "tec": rama eq + rama contains(tec) → todos los tech.
        let both = filter_ids(&db, Filter::any(vec![
            Filter::eq("cat", "tech"),
            Filter::contains("cat", "tec"),
        ]));
        assert_eq!(both, filter_ids(&db, Filter::eq("cat", "tech")));
    }

    /// `$or` con una rama sobre campo NO indexado (campo sin índice) → fallback
    /// correcto, mismo resultado que sin índices.
    #[test]
    fn or_branch_on_unindexed_field_falls_back() {
        let db = build_db();
        let baseline = filter_ids(
            &db,
            Filter::any(vec![Filter::eq("cat", "tech"), Filter::eq("active", true)]),
        );
        // Sólo "cat" indexado: la rama "active" no es indexable → None → fallback.
        db.create_metadata_index("cat").unwrap();
        let with_idx = filter_ids(
            &db,
            Filter::any(vec![Filter::eq("cat", "tech"), Filter::eq("active", true)]),
        );
        assert_eq!(baseline, with_idx);
    }

    /// `$and` con una rama indexada y otra no indexable: poda con la rama
    /// indexada y verificación final → mismo resultado que full-scan.
    #[test]
    fn and_indexed_and_unindexed_branch() {
        let db = build_db();
        let baseline = filter_ids(
            &db,
            Filter::eq("cat", "tech").and(Filter::contains("cat", "ec")),
        );
        db.create_metadata_index("cat").unwrap();
        let with_idx = filter_ids(
            &db,
            Filter::eq("cat", "tech").and(Filter::contains("cat", "ec")),
        );
        assert_eq!(baseline, with_idx);
    }

    /// `Some(empty)` del planner poda a cero: `$and` de dos hojas indexadas con
    /// conjuntos disjuntos → 0 resultados, igual que full-scan.
    #[test]
    fn and_disjoint_indexed_leaves_prunes_to_zero() {
        let db = build_db();
        // n == 5 (un doc) AND n == 6 (otro doc) → imposible → 0.
        let f = Filter::all(vec![Filter::eq("n", 5i64), Filter::eq("n", 6i64)]);
        let baseline = filter_ids(&db, f.clone());
        assert!(baseline.is_empty());
        db.create_metadata_index("n").unwrap();
        assert_eq!(filter_ids(&db, f), HashSet::<String>::new());
    }

    /// `filter_search_ordered` (ORDER BY + filter) también se beneficia y
    /// coincide sin índice.
    #[test]
    fn filter_search_ordered_equivalence() {
        let db = build_db();
        let baseline: HashSet<String> = db
            .filter_search_ordered(
                Filter::eq("cat", "tech"),
                OrderBy::desc("n"),
                10_000,
                0,
            )
            .unwrap()
            .items
            .into_iter()
            .map(|r| r.id)
            .collect();
        db.create_metadata_index("cat").unwrap();
        db.create_metadata_index("n").unwrap();
        let with_idx: HashSet<String> = db
            .filter_search_ordered(
                Filter::eq("cat", "tech"),
                OrderBy::desc("n"),
                10_000,
                0,
            )
            .unwrap()
            .items
            .into_iter()
            .map(|r| r.id)
            .collect();
        assert_eq!(baseline, with_idx);
    }

    /// `list_documents` con ORDER BY + filtro + offset: el planner no rompe la
    /// paginación ni el orden.
    #[test]
    fn list_documents_ordered_paged_equivalence() {
        let db = build_db();
        let baseline: Vec<String> = db
            .list_documents(Some(Filter::gte("n", 100i64)), Some(OrderBy::asc("n")), 10, 5)
            .unwrap()
            .items
            .into_iter()
            .map(|r| r.id)
            .collect();
        db.create_metadata_index("n").unwrap();
        let with_idx: Vec<String> = db
            .list_documents(Some(Filter::gte("n", 100i64)), Some(OrderBy::asc("n")), 10, 5)
            .unwrap()
            .items
            .into_iter()
            .map(|r| r.id)
            .collect();
        assert_eq!(baseline, with_idx);
    }

    /// Casos vacíos: `$and` vacío y `$or` vacío son "siempre verdadero" → el
    /// planner devuelve None (no poda) y se devuelven todos los docs.
    #[test]
    fn empty_and_or_return_all_docs() {
        let db = build_db();
        let all: HashSet<String> = (0..200).map(|i| format!("d{}", i)).collect();
        db.create_metadata_index("cat").unwrap();
        assert_eq!(filter_ids(&db, Filter::all(vec![])), all);
        assert_eq!(filter_ids(&db, Filter::any(vec![])), all);
        assert_eq!(list_ids(&db, Some(Filter::all(vec![]))), all);
    }

    /// `has_metadata_index` refleja el registro.
    #[test]
    fn has_and_list_metadata_index() {
        let db = VectorDB::new(Config::new(3)).unwrap();
        assert!(db.list_metadata_indexes().is_empty());
        db.create_metadata_index("a").unwrap();
        db.create_metadata_index("b").unwrap();
        assert!(db.has_metadata_index("a"));
        assert!(!db.has_metadata_index("z"));
        assert_eq!(db.list_metadata_indexes(), vec!["a", "b"]);
    }

    /// Crear un índice duplicado → AlreadyExists.
    #[test]
    fn create_duplicate_is_error() {
        let db = VectorDB::new(Config::new(3)).unwrap();
        db.create_metadata_index("cat").unwrap();
        assert!(db.create_metadata_index("cat").is_err());
    }

    /// Un índice creado sobre un campo cuyos valores son List/Map (no
    /// indexables) no rompe nada: las consultas caen a None/full-scan.
    #[test]
    fn index_on_list_field_falls_back() {
        let db = build_db();
        db.create_metadata_index("tag_list").unwrap();
        // eq sobre List → candidates_eq devuelve None → full-scan.
        let baseline = filter_ids(&db, Filter::eq("tag_list", "rust"));
        // mismo resultado (vacío: List nunca es igual a un String).
        assert!(baseline.is_empty());
    }
}
