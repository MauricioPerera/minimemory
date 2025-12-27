//! Tests de integración para minimemory.
//!
//! Estos tests verifican el funcionamiento completo de la librería
//! incluyendo todas las funcionalidades principales.

use minimemory::{Config, Distance, IndexType, Metadata, VectorDB};
use std::collections::HashSet;

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
        assert_eq!(retrieved, original);
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
        assert_eq!(vector, vec![4.0, 5.0, 6.0]);
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
    use minimemory::types::MetadataValue;

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

    fn temp_db_path() -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("minimemory_test_{}.mmdb", std::process::id()));
        path
    }

    #[test]
    fn test_save_and_open() {
        let path = temp_db_path();

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
            assert_eq!(vector, vec![1.0, 2.0, 3.0]);
        }

        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_save_with_metadata() {
        let path = temp_db_path();

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
        let path = temp_db_path();

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
        let path = temp_db_path();

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
