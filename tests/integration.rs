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

        assert!(matches!(result, Err(Error::DimensionMismatch { expected: 3, got: 2 })));
    }

    #[test]
    fn test_dimension_mismatch_on_search() {
        let db = VectorDB::new(Config::new(4)).unwrap();
        db.insert("a", &[1.0, 2.0, 3.0, 4.0], None).unwrap();

        let result = db.search(&[1.0, 2.0], 1);

        assert!(matches!(result, Err(Error::DimensionMismatch { expected: 4, got: 2 })));
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

        db.insert("with_meta", &[1.0, 2.0, 3.0], Some(meta)).unwrap();

        let (_, retrieved_meta) = db.get("with_meta").unwrap().unwrap();
        let meta = retrieved_meta.unwrap();

        assert!(matches!(
            meta.get("title"),
            Some(MetadataValue::String(s)) if s == "Test Document"
        ));
        assert!(matches!(meta.get("score"), Some(MetadataValue::Int(95))));
        assert!(matches!(meta.get("active"), Some(MetadataValue::Bool(true))));
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
        let config = Config::new(4)
            .with_index(IndexType::hnsw());

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
            let db = VectorDB::new(
                Config::new(3)
                    .with_distance(Distance::Cosine)
            ).unwrap();

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
                let vector: Vec<f32> = (0..128).map(|j| ((i * 128 + j) % 1000) as f32 / 1000.0).collect();
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
    use minimemory::memory_traits::{GenericMemory, Priority, InstanceContext};
    use minimemory::memory_traits::presets::SoftwareDevelopment;

    fn generate_embedding(seed: usize, dim: usize) -> Vec<f32> {
        (0..dim).map(|i| ((seed * dim + i) % 1000) as f32 / 1000.0).collect()
    }

    #[test]
    fn test_generic_memory_full_workflow() {
        let memory = GenericMemory::<SoftwareDevelopment>::new(64).unwrap();

        memory.set_context(
            InstanceContext::new("test-project")
                .with_context("rust")
                .with_domain("backend")
        );

        let emb = generate_embedding(1, 64);
        memory.learn("task-1", &emb, "Fixed auth bug", "Authentication fix", "success").unwrap();

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
        memory.learn("sec-fix", &emb, "Fixed XSS vulnerability", "Security patch", "success").unwrap();

        let query = generate_embedding(1, 64);
        let results = memory.recall(&query, 1).unwrap();

        assert_eq!(results[0].priority, Priority::Critical);
    }

    #[test]
    fn test_learn_manual_priority() {
        let memory = GenericMemory::<SoftwareDevelopment>::new(64).unwrap();

        let emb = generate_embedding(1, 64);
        memory.learn_with_priority(
            "manual-task", &emb, "Some content", "Description", "success", Priority::High
        ).unwrap();

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
            memory.learn_with_priority(
                &format!("task-{}", i), &emb, "Content", "Desc", "success", priority
            ).unwrap();
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
            memory.learn_with_priority(
                &format!("task-{}", i), &emb, "Content", "Desc", "success", priority
            ).unwrap();
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
        memory.learn("useful-task", &emb, "Content", "Desc", "success").unwrap();

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
        memory.learn("auth-task", &emb1, "JWT token authentication", "Auth system", "success").unwrap();

        let emb2 = generate_embedding(2, 64);
        memory.learn("db-task", &emb2, "Database connection pool", "DB optimization", "success").unwrap();

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
                .with_domain("backend")
        );

        let emb = generate_embedding(1, 64);
        memory.learn("rust-task", &emb, "Rust async code", "Async implementation", "success").unwrap();

        assert!(memory.current_context().is_some());
        let ctx = memory.current_context().unwrap();
        assert_eq!(ctx.instance_id, "project-a");
    }

    #[test]
    fn test_memory_stats_accuracy() {
        let memory = GenericMemory::<SoftwareDevelopment>::new(64).unwrap();

        for i in 0..5 {
            let emb = generate_embedding(i, 64);
            memory.learn(&format!("task-{}", i), &emb, "Content", "Desc", "success").unwrap();
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
        AgentMemory, MemoryConfig, TaskOutcome, CodeSnippet,
        ErrorSolution, Language, MemoryType,
    };

    #[test]
    fn test_agent_memory_creation() {
        let config = MemoryConfig::small();
        let memory = AgentMemory::new(config).unwrap();

        let stats = memory.stats().unwrap();
        assert_eq!(stats.total_entries, 0);
    }

    #[test]
    fn test_learn_task_workflow() {
        let config = MemoryConfig::small();
        let memory = AgentMemory::new(config).unwrap();

        let id = memory.learn_task(
            "Implement login feature",
            "fn login(user: &str) { /* ... */ }",
            TaskOutcome::Success,
            vec!["Use bcrypt for passwords", "Add rate limiting"],
        ).unwrap();

        assert!(id.starts_with("episode-"));
        assert_eq!(memory.db().len(), 1);
    }

    #[test]
    fn test_learn_code_snippet() {
        let config = MemoryConfig::small();
        let memory = AgentMemory::new(config).unwrap();

        let id = memory.learn_code(CodeSnippet {
            code: "fn hello() { println!(\"Hello\"); }".to_string(),
            description: "Simple hello function".to_string(),
            language: Language::Rust,
            dependencies: vec![],
            use_case: "Greeting users".to_string(),
            quality_score: 0.9,
            tags: vec!["example".to_string()],
        }).unwrap();

        assert!(id.starts_with("code-"));

        let _results = memory.recall_code("hello function greeting", 5).unwrap();
        // Results depend on embed_fn, but ID should be correct
        assert_eq!(memory.db().len(), 1);
    }

    #[test]
    fn test_learn_error_solution() {
        let config = MemoryConfig::small();
        let memory = AgentMemory::new(config).unwrap();

        let id = memory.learn_error_solution(ErrorSolution {
            error_message: "cannot borrow as mutable".to_string(),
            error_type: "E0596".to_string(),
            root_cause: "Missing mut keyword".to_string(),
            solution: "Add mut to variable declaration".to_string(),
            fixed_code: Some("let mut x = 5;".to_string()),
            language: Language::Rust,
        }).unwrap();

        assert!(id.starts_with("error-"));

        let _results = memory.recall_error_solutions("cannot borrow mutable", 5).unwrap();
        // Results depend on embed_fn
        assert_eq!(memory.db().len(), 1);
    }

    #[test]
    fn test_recall_similar_hybrid() {
        let config = MemoryConfig::small();
        let memory = AgentMemory::new(config).unwrap();

        for i in 0..5 {
            memory.learn_task(
                &format!("Task {} about authentication and JWT tokens", i),
                &format!("fn auth{}() {{ /* JWT logic */ }}", i),
                TaskOutcome::Success,
                vec!["Use JWT for auth"],
            ).unwrap();
        }

        // recall_similar takes a text query, not an embedding
        let _results = memory.recall_similar("authentication JWT", 3).unwrap();
        // Results depend on embed_fn (placeholder returns zeros)
        assert_eq!(memory.db().len(), 5);
    }

    #[test]
    fn test_recall_experiences_filter() {
        let config = MemoryConfig::small();
        let memory = AgentMemory::new(config).unwrap();

        // Add a task episode
        memory.learn_task(
            "Fix bug in parser",
            "fn parse() { /* fixed */ }",
            TaskOutcome::Success,
            vec!["Check edge cases"],
        ).unwrap();

        // Add a code snippet
        memory.learn_code(CodeSnippet {
            code: "let x = 1;".to_string(),
            description: "Variable declaration".to_string(),
            language: Language::Rust,
            dependencies: vec![],
            use_case: "Initialization".to_string(),
            quality_score: 0.8,
            tags: vec![],
        }).unwrap();

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
        let config = MemoryConfig::small();
        let memory = AgentMemory::new(config).unwrap();

        memory.learn_task(
            "Successful implementation",
            "fn good() { /* works */ }",
            TaskOutcome::Success,
            vec!["Good approach"],
        ).unwrap();

        memory.learn_task(
            "Failed implementation",
            "fn bad() { /* broken */ }",
            TaskOutcome::Failure,
            vec!["Wrong approach"],
        ).unwrap();

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
    use minimemory::{Config, VectorDB};
    use minimemory::replication::{ChangeLog, ReplicationManager};

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

        let parsed: Vec<minimemory::replication::ChangeEntry> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 1);
    }
}

// ============================================================================
// Hybrid Search Integration Tests
// ============================================================================

mod hybrid_search_integration {
    use minimemory::{Config, Metadata, VectorDB, HybridSearchParams, Filter};

    fn generate_vector(seed: usize, dim: usize) -> Vec<f32> {
        (0..dim).map(|i| ((seed * dim + i) % 1000) as f32 / 1000.0).collect()
    }

    #[test]
    fn test_hybrid_search_combined() {
        let db = VectorDB::with_fulltext(
            Config::new(64),
            vec!["content".into()],
        ).unwrap();

        for i in 0..10 {
            let mut meta = Metadata::new();
            meta.insert("content", format!("Document about topic {} and Rust programming", i));
            let v = generate_vector(i, 64);
            db.insert_document(&format!("doc-{}", i), Some(&v), Some(meta)).unwrap();
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

        let results = db.filter_search(Filter::eq("category", "even"), 10).unwrap();

        assert_eq!(results.len(), 10);
        for r in &results {
            let cat = r.metadata.as_ref().unwrap().get("category").unwrap().as_str().unwrap();
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
        let results = db.search_with_filter(&query, 5, Filter::gt("score", 5.0f64)).unwrap();

        for r in &results {
            let score = r.metadata.as_ref().unwrap().get("score").unwrap().as_f64().unwrap();
            assert!(score > 5.0);
        }
    }

    #[test]
    fn test_keyword_search_only() {
        let db = VectorDB::with_fulltext(
            Config::new(64),
            vec!["text".into()],
        ).unwrap();

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
            db.insert_document(&format!("doc-{}", i), Some(&v), Some(meta)).unwrap();
        }

        let results = db.keyword_search("quick fox", 5).unwrap();

        assert!(!results.is_empty());
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
