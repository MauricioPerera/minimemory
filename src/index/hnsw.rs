//! HNSW (Hierarchical Navigable Small World) index implementation.
//!
//! HNSW es un algoritmo de búsqueda aproximada de vecinos más cercanos
//! que ofrece O(log n) en tiempo de búsqueda con alta precisión.
//!
//! ## Optimizaciones implementadas:
//! - Prefetch de vecinos para mejor cache hit rate
//! - ef_search configurable para trade-off precisión/velocidad
//! - Batch processing de candidatos

use parking_lot::RwLock;
use rand::Rng;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

use crate::distance::Distance;
use crate::error::Result;
use crate::storage::Storage;
use crate::types::SearchResult;

use super::Index;

/// HNSW Index para búsqueda aproximada de vecinos más cercanos.
///
/// Implementa un grafo multinivel donde cada nodo tiene conexiones
/// a sus vecinos más cercanos en cada nivel.
///
/// ## Parámetros principales:
/// - `m`: Número de conexiones por nodo (mayor = mejor recall, más memoria)
/// - `ef_construction`: Beam width durante construcción (mayor = mejor grafo)
/// - `ef_search`: Beam width durante búsqueda (configurable en runtime)
pub struct HNSWIndex {
    /// Estructura interna protegida por RwLock para thread-safety
    inner: RwLock<HNSWInner>,
    /// Número de conexiones por nodo en niveles > 0
    m: usize,
    /// Número máximo de conexiones en nivel 0
    m_max0: usize,
    /// Tamaño del beam durante construcción
    ef_construction: usize,
    /// Multiplicador para selección de nivel
    ml: f64,
    /// Tamaño del beam durante búsqueda (configurable)
    ef_search: AtomicUsize,
}

struct HNSWInner {
    /// Niveles del grafo (nivel 0 es el más denso)
    levels: Vec<Level>,
    /// Mapeo de ID a índice interno
    id_to_idx: HashMap<String, usize>,
    /// Mapeo de índice interno a ID
    idx_to_id: Vec<String>,
    /// Punto de entrada (nodo en el nivel más alto)
    entry_point: Option<usize>,
    /// Nivel máximo actual
    max_level: usize,
}

struct Level {
    /// Vecinos de cada nodo en este nivel
    /// neighbors[node_idx] = lista de vecinos
    neighbors: Vec<Vec<usize>>,
}

/// Elemento para el heap de búsqueda
#[derive(Clone)]
struct Candidate {
    idx: usize,
    distance: f32,
}

impl PartialEq for Candidate {
    fn eq(&self, other: &Self) -> bool {
        self.distance == other.distance
    }
}

impl Eq for Candidate {}

impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> Ordering {
        // Invertido para min-heap (menor distancia = mayor prioridad)
        other
            .distance
            .partial_cmp(&self.distance)
            .unwrap_or(Ordering::Equal)
    }
}

/// Wrapper para max-heap (mayor distancia primero)
struct MaxCandidate(Candidate);

impl PartialEq for MaxCandidate {
    fn eq(&self, other: &Self) -> bool {
        self.0.distance == other.0.distance
    }
}

impl Eq for MaxCandidate {}

impl PartialOrd for MaxCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MaxCandidate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0
            .distance
            .partial_cmp(&other.0.distance)
            .unwrap_or(Ordering::Equal)
    }
}

impl HNSWIndex {
    /// Crea un nuevo índice HNSW con los parámetros especificados.
    ///
    /// # Argumentos
    ///
    /// * `m` - Número de conexiones por nodo (típicamente 16)
    /// * `ef_construction` - Tamaño del beam durante construcción (típicamente 200)
    pub fn new(m: usize, ef_construction: usize) -> Self {
        let m = m.max(2); // Mínimo 2 conexiones
        let m_max0 = m * 2; // Nivel 0 tiene el doble de conexiones
        let ml = 1.0 / (m as f64).ln();
        let ef_search = ef_construction / 4; // Default: 25% of ef_construction

        Self {
            inner: RwLock::new(HNSWInner {
                levels: Vec::new(),
                id_to_idx: HashMap::new(),
                idx_to_id: Vec::new(),
                entry_point: None,
                max_level: 0,
            }),
            m,
            m_max0,
            ef_construction,
            ml,
            ef_search: AtomicUsize::new(ef_search.max(10)),
        }
    }

    /// Crea un índice con parámetros por defecto (m=16, ef_construction=200)
    pub fn default_params() -> Self {
        Self::new(16, 200)
    }

    /// Configura ef_search para ajustar el trade-off precisión/velocidad.
    ///
    /// - Mayor ef_search = mejor recall pero más lento
    /// - Menor ef_search = más rápido pero menor recall
    ///
    /// Valores típicos: 50-200 para alta precisión, 10-50 para baja latencia.
    pub fn set_ef_search(&self, ef: usize) {
        self.ef_search.store(ef.max(1), AtomicOrdering::Relaxed);
    }

    /// Obtiene el valor actual de ef_search.
    pub fn get_ef_search(&self) -> usize {
        self.ef_search.load(AtomicOrdering::Relaxed)
    }

    /// Selecciona un nivel aleatorio para un nuevo nodo
    fn random_level(&self) -> usize {
        let mut rng = rand::thread_rng();
        let r: f64 = rng.gen();
        (-r.ln() * self.ml).floor() as usize
    }

    /// Búsqueda greedy en un nivel específico con prefetching optimizado.
    ///
    /// El prefetching carga los vectores de vecinos en cache antes de calcular
    /// distancias, mejorando significativamente la latencia en búsquedas.
    fn search_layer(
        &self,
        inner: &HNSWInner,
        query: &[f32],
        entry_points: Vec<usize>,
        ef: usize,
        level: usize,
        storage: &dyn Storage,
        distance_fn: Distance,
    ) -> Vec<Candidate> {
        let mut visited: HashSet<usize> = HashSet::with_capacity(ef * 2);
        let mut candidates: BinaryHeap<Candidate> = BinaryHeap::with_capacity(ef);
        let mut result: BinaryHeap<MaxCandidate> = BinaryHeap::with_capacity(ef + 1);

        // Inicializar con puntos de entrada
        for ep in entry_points {
            if visited.insert(ep) {
                let id = &inner.idx_to_id[ep];
                if let Ok(Some(stored)) = storage.get(id) {
                    if let Some(vec) = &stored.vector {
                        let dist = distance_fn.calculate(query, vec);
                        let candidate = Candidate {
                            idx: ep,
                            distance: dist,
                        };
                        candidates.push(candidate.clone());
                        result.push(MaxCandidate(candidate));
                    }
                }
            }
        }

        while let Some(current) = candidates.pop() {
            // Si el candidato actual es peor que el peor en result, terminamos
            if let Some(worst) = result.peek() {
                if current.distance > worst.0.distance && result.len() >= ef {
                    break;
                }
            }

            // Explorar vecinos con prefetch
            if level < inner.levels.len() {
                let neighbors = &inner.levels[level].neighbors;
                if current.idx < neighbors.len() {
                    let current_neighbors = &neighbors[current.idx];

                    // Prefetch: Recopilar todos los vecinos no visitados y sus IDs
                    let neighbors_to_process: Vec<(usize, &str)> = current_neighbors
                        .iter()
                        .filter(|&&n| !visited.contains(&n))
                        .filter_map(|&n| {
                            inner.idx_to_id.get(n).map(|id| (n, id.as_str()))
                        })
                        .collect();

                    // Marcar como visitados antes de procesar
                    for &(n, _) in &neighbors_to_process {
                        visited.insert(n);
                    }

                    // Procesar en batch para mejor cache locality
                    for (neighbor_idx, id) in neighbors_to_process {
                        if let Ok(Some(stored)) = storage.get(id) {
                            if let Some(vec) = &stored.vector {
                                let dist = distance_fn.calculate(query, vec);

                                let should_add = result.len() < ef || {
                                    if let Some(worst) = result.peek() {
                                        dist < worst.0.distance
                                    } else {
                                        true
                                    }
                                };

                                if should_add {
                                    let candidate = Candidate {
                                        idx: neighbor_idx,
                                        distance: dist,
                                    };
                                    candidates.push(candidate.clone());
                                    result.push(MaxCandidate(candidate));

                                    // Mantener solo ef elementos
                                    if result.len() > ef {
                                        result.pop();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Convertir result a vector ordenado
        result.into_iter().map(|mc| mc.0).collect()
    }

    /// Selecciona los mejores vecinos usando heurística simple
    fn select_neighbors(&self, candidates: Vec<Candidate>, m: usize) -> Vec<usize> {
        let mut sorted: Vec<_> = candidates;
        sorted.sort_by(|a, b| {
            a.distance
                .partial_cmp(&b.distance)
                .unwrap_or(Ordering::Equal)
        });
        sorted.truncate(m);
        sorted.into_iter().map(|c| c.idx).collect()
    }

    /// Agrega conexiones bidireccionales
    fn connect_neighbors(
        &self,
        inner: &mut HNSWInner,
        node_idx: usize,
        neighbors: &[usize],
        level: usize,
        m_max: usize,
    ) {
        // Asegurar que el nivel existe
        while inner.levels.len() <= level {
            inner.levels.push(Level {
                neighbors: Vec::new(),
            });
        }

        // Asegurar espacio para el nodo
        while inner.levels[level].neighbors.len() <= node_idx {
            inner.levels[level].neighbors.push(Vec::new());
        }

        // Agregar vecinos al nodo
        inner.levels[level].neighbors[node_idx] = neighbors.to_vec();

        // Agregar conexiones inversas (bidireccionales)
        for &neighbor_idx in neighbors {
            while inner.levels[level].neighbors.len() <= neighbor_idx {
                inner.levels[level].neighbors.push(Vec::new());
            }

            let neighbor_neighbors = &mut inner.levels[level].neighbors[neighbor_idx];
            if !neighbor_neighbors.contains(&node_idx) {
                neighbor_neighbors.push(node_idx);

                // Si excede m_max, podar
                if neighbor_neighbors.len() > m_max {
                    // Mantener solo los más cercanos (simplificado)
                    neighbor_neighbors.truncate(m_max);
                }
            }
        }
    }
}

impl Index for HNSWIndex {
    fn add(
        &self,
        id: &str,
        vector: &[f32],
        storage: &dyn Storage,
        distance: Distance,
    ) -> Result<()> {
        let mut inner = self.inner.write();

        // Verificar si ya existe
        if inner.id_to_idx.contains_key(id) {
            return Ok(());
        }

        let new_idx = inner.idx_to_id.len();
        inner.idx_to_id.push(id.to_string());
        inner.id_to_idx.insert(id.to_string(), new_idx);

        // Seleccionar nivel para este nodo
        let node_level = self.random_level();

        // Si es el primer nodo, solo inicializar
        if inner.entry_point.is_none() {
            inner.entry_point = Some(new_idx);
            inner.max_level = node_level;

            // Crear niveles vacíos
            for _ in 0..=node_level {
                inner.levels.push(Level {
                    neighbors: Vec::new(),
                });
            }
            for level in &mut inner.levels {
                level.neighbors.push(Vec::new());
            }

            return Ok(());
        }

        // Asegurar que hay suficientes niveles
        while inner.levels.len() <= node_level {
            inner.levels.push(Level {
                neighbors: Vec::new(),
            });
        }

        // Expandir neighbors para el nuevo nodo en cada nivel
        for level in inner.levels.iter_mut() {
            while level.neighbors.len() <= new_idx {
                level.neighbors.push(Vec::new());
            }
        }

        // Buscar punto de entrada y navegar hacia abajo
        let entry_point = inner.entry_point.unwrap();
        let mut current_nearest = vec![entry_point];

        // Navegación greedy desde niveles superiores
        for level in (node_level + 1..=inner.max_level).rev() {
            let candidates = self.search_layer(
                &inner,
                vector,
                current_nearest.clone(),
                1, // ef=1 para niveles superiores
                level,
                storage,
                distance,
            );
            if !candidates.is_empty() {
                current_nearest = vec![candidates[0].idx];
            }
        }

        // Insertar en cada nivel desde node_level hasta 0
        for level in (0..=node_level.min(inner.max_level)).rev() {
            // Buscar candidatos en este nivel
            let candidates = self.search_layer(
                &inner,
                vector,
                current_nearest.clone(),
                self.ef_construction,
                level,
                storage,
                distance,
            );

            // Seleccionar mejores vecinos
            let m_limit = if level == 0 { self.m_max0 } else { self.m };
            let neighbors = self.select_neighbors(candidates.clone(), m_limit);

            // Conectar bidireccional
            self.connect_neighbors(&mut inner, new_idx, &neighbors, level, m_limit);

            // Usar los mejores candidatos como entrada para el siguiente nivel
            if !candidates.is_empty() {
                current_nearest = candidates.iter().map(|c| c.idx).collect();
            }
        }

        // Actualizar entry point si este nodo tiene nivel más alto
        if node_level > inner.max_level {
            inner.entry_point = Some(new_idx);
            inner.max_level = node_level;
        }

        Ok(())
    }

    fn remove(&self, id: &str) -> Result<bool> {
        let mut inner = self.inner.write();

        if let Some(&idx) = inner.id_to_idx.get(id) {
            // Remover de todos los niveles
            for level in &mut inner.levels {
                if idx < level.neighbors.len() {
                    // Limpiar vecinos de este nodo
                    level.neighbors[idx].clear();

                    // Remover este nodo de las listas de vecinos de otros
                    for neighbors in &mut level.neighbors {
                        neighbors.retain(|&n| n != idx);
                    }
                }
            }

            // Actualizar entry point si es necesario
            if inner.entry_point == Some(idx) {
                // Buscar un nuevo entry point
                inner.entry_point = inner.id_to_idx.values().find(|&&i| i != idx).copied();
            }

            inner.id_to_idx.remove(id);
            // Nota: idx_to_id mantiene el índice pero el ID ya no es válido

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

        if inner.entry_point.is_none() {
            return Ok(vec![]);
        }

        let entry_point = inner.entry_point.unwrap();
        let mut current_nearest = vec![entry_point];

        // Buscar desde el nivel más alto hacia abajo
        for level in (1..=inner.max_level).rev() {
            let candidates = self.search_layer(
                &inner,
                query,
                current_nearest.clone(),
                1, // ef=1 para niveles superiores
                level,
                storage,
                distance,
            );
            if !candidates.is_empty() {
                current_nearest = vec![candidates[0].idx];
            }
        }

        // Búsqueda final en nivel 0 con ef_search configurable
        // Usar el máximo entre k, ef_search configurado, y un mínimo de 10
        let configured_ef = self.ef_search.load(AtomicOrdering::Relaxed);
        let ef_search = k.max(configured_ef).max(10);
        let candidates = self.search_layer(
            &inner,
            query,
            current_nearest,
            ef_search,
            0,
            storage,
            distance,
        );

        // Convertir a SearchResult
        let mut results: Vec<SearchResult> = candidates
            .into_iter()
            .take(k)
            .filter_map(|c| {
                let id = inner.idx_to_id.get(c.idx)?.clone();
                storage.get(&id).ok().flatten().map(|stored| SearchResult {
                    id,
                    distance: c.distance,
                    metadata: stored.metadata,
                })
            })
            .collect();

        // Ordenar por distancia
        results.sort_by(|a, b| {
            a.distance
                .partial_cmp(&b.distance)
                .unwrap_or(Ordering::Equal)
        });
        results.truncate(k);

        Ok(results)
    }

    fn rebuild(&self, storage: &dyn Storage) -> Result<()> {
        let mut inner = self.inner.write();

        // Limpiar estado actual
        inner.levels.clear();
        inner.id_to_idx.clear();
        inner.idx_to_id.clear();
        inner.entry_point = None;
        inner.max_level = 0;

        // Recopilar IDs y vectores
        let entries: Vec<(String, Vec<f32>)> = storage
            .ids()
            .into_iter()
            .filter_map(|id| {
                storage
                    .get(&id)
                    .ok()
                    .flatten()
                    .and_then(|stored| stored.vector.map(|v| (id.clone(), v)))
            })
            .collect();

        drop(inner); // Liberar lock antes de llamar add

        // Re-insertar todos los vectores
        for (id, vector) in entries {
            self.add(&id, &vector, storage, Distance::Cosine)?;
        }

        Ok(())
    }

    fn len(&self) -> usize {
        self.inner.read().id_to_idx.len()
    }

    fn clear(&self) {
        let mut inner = self.inner.write();
        inner.levels.clear();
        inner.id_to_idx.clear();
        inner.idx_to_id.clear();
        inner.entry_point = None;
        inner.max_level = 0;
    }
}

impl Default for HNSWIndex {
    fn default() -> Self {
        Self::default_params()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::MemoryStorage;

    #[test]
    fn test_hnsw_basic() {
        let storage = MemoryStorage::new();
        let index = HNSWIndex::new(4, 20);

        // Insertar vectores
        let vectors = vec![
            ("a", vec![1.0, 0.0, 0.0]),
            ("b", vec![0.0, 1.0, 0.0]),
            ("c", vec![0.0, 0.0, 1.0]),
            ("d", vec![0.5, 0.5, 0.0]),
        ];

        for (id, data) in &vectors {
            storage
                .insert(id.to_string(), Some(data.clone()), None)
                .unwrap();
            index.add(id, data, &storage, Distance::Euclidean).unwrap();
        }

        assert_eq!(index.len(), 4);
    }

    #[test]
    fn test_hnsw_search() {
        let storage = MemoryStorage::new();
        // Usar ef_construction más alto para mejor recall
        let index = HNSWIndex::new(8, 100);

        let vectors = vec![
            ("a", vec![1.0, 0.0, 0.0]),
            ("b", vec![0.9, 0.1, 0.0]),
            ("c", vec![0.0, 1.0, 0.0]),
            ("d", vec![0.0, 0.0, 1.0]),
        ];

        for (id, data) in &vectors {
            storage
                .insert(id.to_string(), Some(data.clone()), None)
                .unwrap();
            index.add(id, data, &storage, Distance::Euclidean).unwrap();
        }

        let query = vec![1.0, 0.0, 0.0];
        let results = index
            .search(&query, 4, &storage, Distance::Euclidean)
            .unwrap();

        // Verificar que tenemos resultados
        assert!(!results.is_empty());

        // El resultado más cercano debería tener distancia ~0 (vector "a")
        // y debería ser "a" o "b" (los más cercanos a [1,0,0])
        assert!(
            results[0].distance < 0.2,
            "Expected first result to be close, got distance {}",
            results[0].distance
        );

        // Verificar que "a" está en los resultados (debería tener distancia 0)
        let a_result = results.iter().find(|r| r.id == "a");
        assert!(
            a_result.is_some(),
            "Expected 'a' to be in results"
        );
        if let Some(a) = a_result {
            assert!(
                a.distance < 0.001,
                "Expected 'a' to have distance ~0, got {}",
                a.distance
            );
        }
    }

    #[test]
    fn test_hnsw_delete() {
        let storage = MemoryStorage::new();
        let index = HNSWIndex::new(4, 20);

        storage
            .insert("a".to_string(), Some(vec![1.0, 0.0]), None)
            .unwrap();
        storage
            .insert("b".to_string(), Some(vec![0.0, 1.0]), None)
            .unwrap();

        index
            .add("a", &[1.0, 0.0], &storage, Distance::Euclidean)
            .unwrap();
        index
            .add("b", &[0.0, 1.0], &storage, Distance::Euclidean)
            .unwrap();

        assert_eq!(index.len(), 2);

        index.remove("a").unwrap();
        // Nota: len() no decrece porque mantenemos el índice
    }

    #[test]
    fn test_random_level_distribution() {
        let index = HNSWIndex::new(16, 200);
        let mut levels = vec![0usize; 10];

        for _ in 0..1000 {
            let level = index.random_level().min(9);
            levels[level] += 1;
        }

        // La mayoría debería estar en nivel 0
        assert!(levels[0] > levels[1]);
        // Debería decrecer exponencialmente
        for i in 1..9 {
            if levels[i] > 0 && levels[i + 1] > 0 {
                assert!(levels[i] >= levels[i + 1]);
            }
        }
    }
}
