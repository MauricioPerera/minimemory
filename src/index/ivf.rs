//! IVF (Inverted File) index with K-means clustering.
//!
//! Partitions the vector space into clusters using K-means and performs
//! multi-probe search by examining only the nearest clusters.
//!
//! ## Algorithm
//! - **K-means++ initialization** for centroid seeding
//! - **Lloyd's iterations** (max 20) for centroid refinement
//! - **Multi-probe search**: query the `num_probes` nearest clusters
//!
//! ## Trade-offs
//! - `num_clusters`: more clusters = faster search, but needs more data to train well
//! - `num_probes`: more probes = better recall, slower search

use parking_lot::RwLock;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

use crate::distance::Distance;
use crate::error::Result;
use crate::storage::Storage;
use crate::types::SearchResult;

use super::Index;

// ---------------------------------------------------------------------------
// Max-heap wrapper for top-k selection (same pattern as FlatIndex)
// ---------------------------------------------------------------------------

struct MaxSearchResult(SearchResult);

impl PartialEq for MaxSearchResult {
    fn eq(&self, other: &Self) -> bool {
        self.0.distance == other.0.distance
    }
}
impl Eq for MaxSearchResult {}

impl PartialOrd for MaxSearchResult {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MaxSearchResult {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0
            .distance
            .partial_cmp(&other.0.distance)
            .unwrap_or(Ordering::Equal)
    }
}

// ---------------------------------------------------------------------------
// IVF Index
// ---------------------------------------------------------------------------

/// Inverted File index backed by K-means clustering.
///
/// Vectors are assigned to the nearest centroid; search examines
/// `num_probes` closest clusters for approximate nearest neighbors.
pub struct IVFIndex {
    inner: RwLock<IVFInner>,
    num_clusters: usize,
    num_probes: usize,
}

#[derive(Serialize, Deserialize)]
struct IVFInner {
    /// Cluster centroids (one per cluster).
    centroids: Vec<Vec<f32>>,
    /// Sets of vector IDs belonging to each cluster.
    cluster_members: Vec<HashSet<String>>,
    /// Reverse mapping: vector ID -> cluster index.
    id_to_cluster: HashMap<String, usize>,
    /// Whether K-means training has been performed.
    trained: bool,
}

impl IVFInner {
    fn new(num_clusters: usize) -> Self {
        Self {
            centroids: Vec::new(),
            cluster_members: vec![HashSet::new(); num_clusters],
            id_to_cluster: HashMap::new(),
            trained: false,
        }
    }

    fn total_vectors(&self) -> usize {
        self.id_to_cluster.len()
    }
}

impl IVFIndex {
    /// Create a new IVF index.
    ///
    /// # Arguments
    /// * `num_clusters` - Number of Voronoi cells (K)
    /// * `num_probes`   - Number of clusters to examine during search
    pub fn new(num_clusters: usize, num_probes: usize) -> Self {
        let num_probes = num_probes.min(num_clusters).max(1);
        Self {
            inner: RwLock::new(IVFInner::new(num_clusters)),
            num_clusters,
            num_probes,
        }
    }
}

// ---------------------------------------------------------------------------
// K-means helpers (module-private)
// ---------------------------------------------------------------------------

/// Squared Euclidean distance (used internally for K-means regardless of the
/// user-chosen distance metric — centroids are always in L2 space).
fn sq_euclidean(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| {
            let d = x - y;
            d * d
        })
        .sum()
}

/// K-means++ initialization: pick `k` centroids from `vectors`.
fn kmeans_pp_init(vectors: &[Vec<f32>], k: usize) -> Vec<Vec<f32>> {
    let n = vectors.len();
    assert!(k > 0 && n > 0);
    let k = k.min(n);

    let mut rng = rand::thread_rng();
    let mut centroids: Vec<Vec<f32>> = Vec::with_capacity(k);

    // First centroid: random
    centroids.push(vectors[rng.gen_range(0..n)].clone());

    // Remaining centroids: proportional to min-distance-squared
    let mut min_dists = vec![f32::MAX; n];
    for _ in 1..k {
        let last = centroids.last().unwrap();
        for (i, v) in vectors.iter().enumerate() {
            let d = sq_euclidean(v, last);
            if d < min_dists[i] {
                min_dists[i] = d;
            }
        }
        let total: f64 = min_dists.iter().map(|&d| d as f64).sum();
        if total <= 0.0 {
            // All remaining points coincide with existing centroids
            centroids.push(vectors[rng.gen_range(0..n)].clone());
            continue;
        }
        let threshold = rng.gen::<f64>() * total;
        let mut cumulative = 0.0f64;
        let mut chosen = n - 1;
        for (i, &d) in min_dists.iter().enumerate() {
            cumulative += d as f64;
            if cumulative >= threshold {
                chosen = i;
                break;
            }
        }
        centroids.push(vectors[chosen].clone());
    }
    centroids
}

/// Run Lloyd's algorithm for at most `max_iters` iterations.
/// Returns (centroids, assignments).
fn kmeans(
    vectors: &[Vec<f32>],
    k: usize,
    max_iters: usize,
) -> (Vec<Vec<f32>>, Vec<usize>) {
    let n = vectors.len();
    let k = k.min(n);
    let dim = vectors[0].len();

    let mut centroids = kmeans_pp_init(vectors, k);
    let mut assignments = vec![0usize; n];

    for _iter in 0..max_iters {
        // --- Assign step ---
        let mut changed = false;
        for (i, v) in vectors.iter().enumerate() {
            let mut best_c = 0;
            let mut best_d = f32::MAX;
            for (c, centroid) in centroids.iter().enumerate() {
                let d = sq_euclidean(v, centroid);
                if d < best_d {
                    best_d = d;
                    best_c = c;
                }
            }
            if assignments[i] != best_c {
                assignments[i] = best_c;
                changed = true;
            }
        }

        if !changed {
            break; // converged
        }

        // --- Update step ---
        let mut sums = vec![vec![0.0f64; dim]; k];
        let mut counts = vec![0usize; k];
        for (i, v) in vectors.iter().enumerate() {
            let c = assignments[i];
            counts[c] += 1;
            for (j, &val) in v.iter().enumerate() {
                sums[c][j] += val as f64;
            }
        }
        for c in 0..k {
            if counts[c] == 0 {
                continue; // keep old centroid for empty cluster
            }
            for j in 0..dim {
                centroids[c][j] = (sums[c][j] / counts[c] as f64) as f32;
            }
        }
    }

    (centroids, assignments)
}

// ---------------------------------------------------------------------------
// Index trait implementation
// ---------------------------------------------------------------------------

impl Index for IVFIndex {
    fn add(
        &self,
        id: &str,
        vector: &[f32],
        _storage: &dyn Storage,
        _distance: Distance,
    ) -> Result<()> {
        let mut inner = self.inner.write();
        if inner.trained && !inner.centroids.is_empty() {
            // Find nearest centroid and assign
            let cluster = nearest_centroid(&inner.centroids, vector);
            // Remove from old cluster if re-adding
            if let Some(&old) = inner.id_to_cluster.get(id) {
                inner.cluster_members[old].remove(id);
            }
            inner.cluster_members[cluster].insert(id.to_string());
            inner.id_to_cluster.insert(id.to_string(), cluster);
        } else {
            // Not yet trained — just track the ID (cluster 0 as placeholder).
            // Real assignment will happen on next rebuild().
            if inner.cluster_members.is_empty() {
                inner.cluster_members.push(HashSet::new());
            }
            inner.cluster_members[0].insert(id.to_string());
            inner.id_to_cluster.insert(id.to_string(), 0);
        }
        Ok(())
    }

    fn remove(&self, id: &str) -> Result<bool> {
        let mut inner = self.inner.write();
        if let Some(cluster) = inner.id_to_cluster.remove(id) {
            if cluster < inner.cluster_members.len() {
                inner.cluster_members[cluster].remove(id);
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn search(
        &self,
        query: &[f32],
        k: usize,
        storage: &dyn Storage,
        distance: Distance,
    ) -> Result<Vec<SearchResult>> {
        let inner = self.inner.read();

        if inner.id_to_cluster.is_empty() {
            return Ok(Vec::new());
        }

        // Collect candidate IDs from the nearest `num_probes` clusters.
        let candidate_ids: Vec<&String> = if inner.trained && !inner.centroids.is_empty() {
            let probe_clusters = nearest_n_centroids(&inner.centroids, query, self.num_probes);
            probe_clusters
                .iter()
                .flat_map(|&c| inner.cluster_members[c].iter())
                .collect()
        } else {
            // Not trained — fall back to brute-force over all tracked IDs
            inner.id_to_cluster.keys().collect()
        };

        // Brute-force top-k among candidates
        let mut heap: BinaryHeap<MaxSearchResult> = BinaryHeap::with_capacity(k + 1);
        for id in candidate_ids {
            if let Ok(Some(stored)) = storage.get(id) {
                if let Some(vec) = stored.vector.as_ref() {
                    let dist = distance.calculate(query, vec);
                    if heap.len() < k {
                        heap.push(MaxSearchResult(SearchResult {
                            id: stored.id,
                            distance: dist,
                            metadata: stored.metadata,
                        }));
                    } else if let Some(worst) = heap.peek() {
                        if dist < worst.0.distance {
                            heap.pop();
                            heap.push(MaxSearchResult(SearchResult {
                                id: stored.id,
                                distance: dist,
                                metadata: stored.metadata,
                            }));
                        }
                    }
                }
            }
        }

        let mut results: Vec<SearchResult> = heap.into_iter().map(|m| m.0).collect();
        results.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(Ordering::Equal));
        Ok(results)
    }

    fn rebuild(&self, storage: &dyn Storage) -> Result<()> {
        // Collect all vectors from storage
        let mut ids: Vec<String> = Vec::new();
        let mut vectors: Vec<Vec<f32>> = Vec::new();
        for stored in storage.iter_with_vectors() {
            if let Some(vec) = stored.vector {
                ids.push(stored.id);
                vectors.push(vec);
            }
        }

        if vectors.is_empty() {
            let mut inner = self.inner.write();
            *inner = IVFInner::new(self.num_clusters);
            return Ok(());
        }

        // Actual K — may be less than configured if we have fewer vectors
        let k = self.num_clusters.min(vectors.len());

        let (centroids, assignments) = kmeans(&vectors, k, 20);

        let actual_k = centroids.len();
        let mut cluster_members = vec![HashSet::new(); actual_k];
        let mut id_to_cluster = HashMap::with_capacity(ids.len());

        for (i, id) in ids.iter().enumerate() {
            let c = assignments[i];
            cluster_members[c].insert(id.clone());
            id_to_cluster.insert(id.clone(), c);
        }

        let mut inner = self.inner.write();
        inner.centroids = centroids;
        inner.cluster_members = cluster_members;
        inner.id_to_cluster = id_to_cluster;
        inner.trained = true;

        Ok(())
    }

    fn len(&self) -> usize {
        self.inner.read().total_vectors()
    }

    fn clear(&self) {
        let mut inner = self.inner.write();
        *inner = IVFInner::new(self.num_clusters);
    }

    fn serialize_index(&self) -> Result<Option<Vec<u8>>> {
        let inner = self.inner.read();
        let data = bincode::serialize(&*inner)?;
        Ok(Some(data))
    }

    fn load_index(&self, data: &[u8]) -> Result<()> {
        let loaded: IVFInner = bincode::deserialize(data)?;
        let mut inner = self.inner.write();
        *inner = loaded;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Centroid look-up helpers
// ---------------------------------------------------------------------------

/// Return the index of the nearest centroid to `vector`.
fn nearest_centroid(centroids: &[Vec<f32>], vector: &[f32]) -> usize {
    let mut best = 0;
    let mut best_d = f32::MAX;
    for (i, c) in centroids.iter().enumerate() {
        let d = sq_euclidean(vector, c);
        if d < best_d {
            best_d = d;
            best = i;
        }
    }
    best
}

/// Return the indices of the `n` nearest centroids to `vector`.
fn nearest_n_centroids(centroids: &[Vec<f32>], vector: &[f32], n: usize) -> Vec<usize> {
    let mut dists: Vec<(usize, f32)> = centroids
        .iter()
        .enumerate()
        .map(|(i, c)| (i, sq_euclidean(vector, c)))
        .collect();
    dists.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
    dists.iter().take(n).map(|&(i, _)| i).collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::MemoryStorage;

    /// Helper: insert vectors into storage and index, return the storage.
    fn setup(
        pairs: &[(&str, Vec<f32>)],
    ) -> (MemoryStorage, IVFIndex) {
        let storage = MemoryStorage::new();
        // 2 clusters, 2 probes (examine all clusters in tests for exact results)
        let index = IVFIndex::new(2, 2);

        for (id, vec) in pairs {
            storage
                .insert(id.to_string(), Some(vec.clone()), None)
                .unwrap();
            index
                .add(id, vec, &storage, Distance::Euclidean)
                .unwrap();
        }
        (storage, index)
    }

    #[test]
    fn test_ivf_basic() {
        let pairs: Vec<(&str, Vec<f32>)> = vec![
            ("a", vec![1.0, 0.0, 0.0]),
            ("b", vec![0.0, 1.0, 0.0]),
            ("c", vec![0.0, 0.0, 1.0]),
            ("d", vec![1.0, 1.0, 0.0]),
        ];
        let (storage, index) = setup(&pairs);

        // Build clusters
        index.rebuild(&storage).unwrap();
        assert_eq!(index.len(), 4);

        // Search for nearest to [1, 0, 0]
        let results = index
            .search(&[1.0, 0.0, 0.0], 2, &storage, Distance::Euclidean)
            .unwrap();

        assert_eq!(results.len(), 2);
        // Exact match should be first
        assert_eq!(results[0].id, "a");
        assert!(results[0].distance < 1e-6);
    }

    #[test]
    fn test_ivf_empty() {
        let storage = MemoryStorage::new();
        let index = IVFIndex::new(4, 2);

        let results = index
            .search(&[1.0, 0.0], 5, &storage, Distance::Euclidean)
            .unwrap();
        assert!(results.is_empty());
        assert_eq!(index.len(), 0);
        assert!(index.is_empty());
    }

    #[test]
    fn test_ivf_add_after_build() {
        let pairs: Vec<(&str, Vec<f32>)> = vec![
            ("a", vec![1.0, 0.0]),
            ("b", vec![0.0, 1.0]),
            ("c", vec![0.5, 0.5]),
        ];
        let (storage, index) = setup(&pairs);
        index.rebuild(&storage).unwrap();
        assert_eq!(index.len(), 3);

        // Add a new vector after build
        let new_vec = vec![0.9, 0.1];
        storage
            .insert("d".to_string(), Some(new_vec.clone()), None)
            .unwrap();
        index
            .add("d", &new_vec, &storage, Distance::Euclidean)
            .unwrap();

        assert_eq!(index.len(), 4);

        // Should be findable
        let results = index
            .search(&[1.0, 0.0], 4, &storage, Distance::Euclidean)
            .unwrap();
        let ids: Vec<&str> = results.iter().map(|r| r.id.as_str()).collect();
        assert!(ids.contains(&"d"));
    }

    #[test]
    fn test_ivf_remove() {
        let pairs: Vec<(&str, Vec<f32>)> = vec![
            ("a", vec![1.0, 0.0, 0.0]),
            ("b", vec![0.0, 1.0, 0.0]),
            ("c", vec![0.0, 0.0, 1.0]),
        ];
        let (storage, index) = setup(&pairs);
        index.rebuild(&storage).unwrap();
        assert_eq!(index.len(), 3);

        // Remove "a"
        let removed = index.remove("a").unwrap();
        assert!(removed);
        assert_eq!(index.len(), 2);

        // Removing again returns false
        let removed_again = index.remove("a").unwrap();
        assert!(!removed_again);

        // Search should not return "a"
        let results = index
            .search(&[1.0, 0.0, 0.0], 3, &storage, Distance::Euclidean)
            .unwrap();
        let ids: Vec<&str> = results.iter().map(|r| r.id.as_str()).collect();
        assert!(!ids.contains(&"a"));
    }

    #[test]
    fn test_ivf_serialization() {
        let pairs: Vec<(&str, Vec<f32>)> = vec![
            ("a", vec![1.0, 0.0]),
            ("b", vec![0.0, 1.0]),
            ("c", vec![0.5, 0.5]),
            ("d", vec![0.9, 0.1]),
        ];
        let (storage, index) = setup(&pairs);
        index.rebuild(&storage).unwrap();

        // Serialize
        let data = index.serialize_index().unwrap().unwrap();
        assert!(!data.is_empty());

        // Load into a fresh index
        let index2 = IVFIndex::new(2, 2);
        index2.load_index(&data).unwrap();

        assert_eq!(index2.len(), index.len());

        // Search results should match
        let query = vec![1.0, 0.0];
        let r1 = index
            .search(&query, 2, &storage, Distance::Euclidean)
            .unwrap();
        let r2 = index2
            .search(&query, 2, &storage, Distance::Euclidean)
            .unwrap();

        assert_eq!(r1.len(), r2.len());
        for (a, b) in r1.iter().zip(r2.iter()) {
            assert_eq!(a.id, b.id);
            assert!((a.distance - b.distance).abs() < 1e-6);
        }
    }

    #[test]
    fn test_ivf_clear() {
        let pairs: Vec<(&str, Vec<f32>)> = vec![
            ("a", vec![1.0, 0.0]),
            ("b", vec![0.0, 1.0]),
        ];
        let (storage, index) = setup(&pairs);
        index.rebuild(&storage).unwrap();
        assert_eq!(index.len(), 2);

        index.clear();
        assert_eq!(index.len(), 0);
        assert!(index.is_empty());

        let results = index
            .search(&[1.0, 0.0], 5, &storage, Distance::Euclidean)
            .unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_ivf_fewer_vectors_than_clusters() {
        // Edge case: more clusters requested than vectors available
        let storage = MemoryStorage::new();
        let index = IVFIndex::new(100, 10); // 100 clusters but only 3 vectors

        let pairs = vec![
            ("a", vec![1.0, 0.0]),
            ("b", vec![0.0, 1.0]),
            ("c", vec![0.5, 0.5]),
        ];
        for (id, vec) in &pairs {
            storage
                .insert(id.to_string(), Some(vec.clone()), None)
                .unwrap();
            index
                .add(id, vec, &storage, Distance::Euclidean)
                .unwrap();
        }

        // Should not panic — uses min(num_clusters, n)
        index.rebuild(&storage).unwrap();
        assert_eq!(index.len(), 3);

        let results = index
            .search(&[1.0, 0.0], 2, &storage, Distance::Euclidean)
            .unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "a");
    }
}
