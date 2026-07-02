# Auditoría READ-ONLY — Índices y SIMD (minimemory)

Alcance: `src/index/{mod,flat,hnsw,ivf,bm25}.rs`, `src/distance/{mod,simd}.rs`.
Contexto leído para alcanzabilidad: `src/db.rs`, `src/storage/mod.rs`, `src/types.rs`, `src/lib.rs`, `src/partial_index.rs`.

Verificación de build: `cargo check` OK (1 warning ajeno al alcance, en `storage/disk.rs`).
Verificación de features: en un build default x86_64, `target_feature="sse"` está activo y `avx2`/`avx512f` NO. Por tanto la ruta `unsafe` SSE se compila y se ejecuta en runtime (`is_x86_feature_detected!("sse")` es true en todo x86_64); las rutas AVX2/AVX-512 son código muerto salvo que se compile con `target-cpu=native`/`target-feature`.

---

## [CRITICAL] Lectura fuera de límites (UB) en ruta SIMD cuando `a.len() != b.len()`

- Archivo: `src/distance/simd.rs:533-540` (euclidean_sse; mismo patrón en `cosine_sse:561-580`, `dot_sse:606-616`, y equivalentes AVX2/AVX-512/NEON)
- Código:
```rust
#[target_feature(enable = "sse")]
unsafe fn euclidean_sse(a: &[f32], b: &[f32]) -> f32 {
    let mut sum = _mm_setzero_ps();
    let chunks = a.len() / 4;

    for i in 0..chunks {
        let offset = i * 4;
        let va = _mm_loadu_ps(a.as_ptr().add(offset));
        let vb = _mm_loadu_ps(b.as_ptr().add(offset));
        let diff = _mm_sub_ps(va, vb);
        ...
    }
```
- Problema: Todas las funciones SIMD calculan `chunks = a.len() / W` y cargan `W` floats de `b` con `_mm_loadu_ps(b.as_ptr().add(offset))` sin verificar `a.len() == b.len()`. Si `b` es más corto que `a`, se leen floats más allá de `b.len()` (y posiblemente más allá de la capacidad del `Vec`): lectura out-of-bounds = **undefined behavior**. La ruta escalar (`euclidean_scalar`, etc.) sí es segura: indexa `b[base+i]` y hace panic (bounds check) si difieren. Pero la ruta SIMD se toma primero cuando `a.len() >= 4`.
- Escenario de fallo: `Distance::Euclidean.calculate(&[1.0, 2.0, 3.0, 4.0], &[1.0, 2.0])`. En x86_64 default: `a.len()=4 >= 4` → entra `euclidean_sse`; `chunks=1`; `_mm_loadu_ps(b.as_ptr())` lee 4 floats de un slice de longitud 2 → UB (2 floats leídos fuera del slice; potencial segfault si se pasa de la capacity).
- Verificación: `Distance` es pública (`lib.rs:168 pub use distance::Distance`) y `Distance::calculate` es `pub` (`distance/mod.rs:35`). `cargo rustc -- --print cfg` confirma `target_feature="sse"` activo por defecto → la ruta SSE se compila y ejecuta sin need de flags especiales. La API pública `VectorDB` sí valida dimensiones (`db.rs:353`, `db.rs:519`) y por eso internamente no se dispara, pero `Distance::calculate` es utilizable directamente por el usuario.

---

## [HIGH] IVF nunca se entrena desde la API pública `VectorDB` (degrada silenciosamente a brute-force)

- Archivo: `src/db.rs` (ausencia de llamada a `self.index.rebuild()`) + `src/index/ivf.rs:236-263` (add) y `ivf.rs:278-301` (search)
- Código (ivf.rs:244-262, rama `add` sin entrenar):
```rust
if inner.trained && !inner.centroids.is_empty() {
    let cluster = nearest_centroid(&inner.centroids, vector);
    ...
} else {
    // Not yet trained — just track the ID (cluster 0 as placeholder).
    // Real assignment will happen on next rebuild().
    if inner.cluster_members.is_empty() {
        inner.cluster_members.push(HashSet::new());
    }
    inner.cluster_members[0].insert(id.to_string());
    inner.id_to_cluster.insert(id.to_string(), 0);
}
```
- Código (ivf.rs:292-301, rama `search` sin entrenar):
```rust
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
```
- Problema: `IVFIndex` sólo construye centroides dentro de `rebuild()` (`ivf.rs:334`). `VectorDB` **nunca** llama `self.index.rebuild()`: `grep` de `.rebuild(` en `src/` muestra como únicos callers `db.rs:865` (que opera sobre un `PartialIndex`, no sobre `self.index`) y los tests en `ivf.rs`. Por tanto, usando `VectorDB` con `IndexType::IVF{...}`, `inner.trained` queda en `false` para siempre y toda búsqueda cae al fallback brute-force sobre todos los IDs. El clustering K-means y el multi-probe (`num_clusters`/`num_probes`) —toda la razón de ser de IVF— son inactivos; el índice es funcionalmente un `Flat` sin que el usuario lo sepa.
- Escenario de fallo: `VectorDB::new(Config::new(8).with_index(IndexType::ivf_with_params(100,10)))`, insertar N vectores, buscar. Nunca se particiona; `search` recorre todos los IDs. No hay error, sólo comportamiento silenciosamente degradado (misma complejidad que flat, pero el usuario cree que tiene IVF).
- Verificación: `grep -n "\.rebuild\("` → `db.rs:865` (PartialIndex), `ivf.rs` tests, `hnsw.rs:613`/`flat.rs:120`/`mod.rs:98` (definiciones). Ningún path de `VectorDB::{new,with_fulltext,open,open_with_fulltext,insert,insert_document,search}` invoca `self.index.rebuild()`. `HybridSearch` (vía `search.rs`) también llama sólo a `index.search`.

---

## [HIGH] NaN/Inf en vectores corrompen resultados silenciosamente en todos los índices

- Archivo: `src/index/hnsw.rs:100-108` y `:264-283`; `src/index/flat.rs:33-40` y `:100-109`; `src/index/ivf.rs:304-326`; `src/distance/simd.rs:172-190` (cosine_scalar) / `:449-484` (cosine_avx2)
- Código (distancia coseno produce NaN — simd.rs:184-189):
```rust
let denom = (norm_a * norm_b).sqrt();
if denom == 0.0 {
    return 1.0;
}
1.0 - (dot / denom)
```
- Código (heap de FlatIndex con comparación NaN — flat.rs:33-39):
```rust
impl Ord for MaxSearchResult {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.distance.partial_cmp(&other.0.distance).unwrap_or(Ordering::Equal)
    }
}
```
- Código (puerta de eviction en HNSW — hnsw.rs:264-270):
```rust
let should_add = result.len() < ef || {
    if let Some(worst) = result.peek() {
        dist < worst.0.distance
    } else {
        true
    }
};
```
- Problema: Si un vector almacenado contiene `NaN`/`Inf`, `Distance::calculate` devuelve `NaN` (coseno: si una norma es NaN, `denom` es NaN; `denom == 0.0` es false → retorna `1.0 - (dot/NaN)` = NaN. Euclidean/Manhattan con NaN también propagan NaN). Ningún índice valida NaN. Los `Ord`/comparadores usan `partial_cmp(...).unwrap_or(Ordering::Equal)`, así que **no hay panic**, pero:
  - En `FlatIndex.search`, un resultado con distancia NaN entra al heap cuando `heap.len() < k` (línea 94). Una vez dentro, ningún candidato real lo puede evictar porque `dist < worst` con `worst = NaN` es siempre `false` (línea 101). El NaN ocupa un slot del top-k permanentemente y bloquea mejores vecinos.
  - En `HNSWIndex.search_layer`, idem: `dist < worst.0.distance` es false para NaN, el candidato NaN no se reemplaza y además impide que otros evicten al peor (líneas 264-270, 232-233).
  - El `sort_by` final (flat.rs:115, hnsw.rs:603, ivf.rs:330) con comparador no-total sobre NaN produce orden no especificado (sin panic, sin UB, pero resultados corruptos).
- Escenario de fallo: `db.insert("a", &[f32::NAN, 0.0, 0.0], None)` (aceptado: `db.rs:353` sólo valida `len`, no finitud). Luego `db.search(&[1.0,0.0,0.0], 5)` → resultados con distancias NaN mezclados, orden arbitrario, y un slot del top-k atrapado por el NaN.
- Verificación: `VectorDB::insert` (`db.rs:345-389`) no inspecciona valores; sólo `vector.len()`. Los comparadores citados usan `unwrap_or(Ordering::Equal)` (no panic). `f32::NAN < x` y `x < f32::NAN` son `false` por IEEE-754 → la lógica de eviction citada queda inerte. Alcanzable trivialmente desde la API pública.

---

## [MEDIUM] Panic por indexación directa sobre estado serializado hostil (HNSW)

- Archivo: `src/index/hnsw.rs:215` (y `:249` vía `.get`, seguro); entrada en `:213-216`
- Código:
```rust
for &ep in entry_points {
    if visited.insert(ep) {
        let id = &inner.idx_to_id[ep];
        if let Ok(Some(stored)) = storage.get(id) {
```
- Problema: `inner.idx_to_id[ep]` indexa sin bounds-check. `entry_points` proviene de `inner.entry_point` (hnsw.rs:555 `inner.entry_point.unwrap()`, validado sólo `is_none`, no rango) o de vecinos ya en el grafo. Si el `HNSWInner` fue cargado vía `load_index` desde bytes hostiles (bincode deserialize sin validación), `entry_point` puede ser `>= idx_to_id.len()` → **panic** (index out of bounds). Lo mismo aplica a `inner.idx_to_id[neighbor_idx]` en `connect_neighbors:349` si los vecinos serializados referencian índices inexistentes (aunque en flujo normal los vecinos son válidos).
- Escenario de fallo: Un archivo `.mmdb` crafteado donde el bloque `hnsw` serializa `entry_point = 9999` con `idx_to_id` de longitud 2. `VectorDB::open` → `index.load_index(&data)` (db.rs:167-168) acepta los bytes → próxima `db.search` → `search_layer` → `inner.idx_to_id[9999]` → panic.
- Verificación: `load_index` (hnsw.rs:669-674) hace `bincode::deserialize(data)?` y asigna `*inner = loaded` sin validar invariantes (entry_point en rango, vecinos en rango). `VectorDB::open` (db.rs:135-186) carga el bloque y, si `load_index` no falla, NO reconstruye (`need_rebuild = false`). `search` (hnsw.rs:542) toma `inner.entry_point.unwrap()` y lo pasa a `search_layer` que indexa `idx_to_id[ep]`. Alcanzable desde API pública con input no confiable.

---

## [MEDIUM] Panic por indexación sobre estado serializado hostil (IVF)

- Archivo: `src/index/ivf.rs:296` (search) y `:252` (add)
- Código (search — ivf.rs:292-297):
```rust
let candidate_ids: Vec<&String> = if inner.trained && !inner.centroids.is_empty() {
    let probe_clusters = nearest_n_centroids(&inner.centroids, query, self.num_probes);
    probe_clusters
        .iter()
        .flat_map(|&c| inner.cluster_members[c].iter())
        .collect()
```
- Código (add — ivf.rs:247-253):
```rust
let cluster = nearest_centroid(&inner.centroids, vector);
if let Some(&old) = inner.id_to_cluster.get(id) {
    inner.cluster_members[old].remove(id);
}
inner.cluster_members[cluster].insert(id.to_string());
inner.id_to_cluster.insert(id.to_string(), cluster);
```
- Problema: `inner.cluster_members[c]` y `inner.cluster_members[cluster]` indexan sin verificar que el índice `< cluster_members.len()`. En datos normales `cluster_members.len() == centroids.len()` y los índices vienen de `nearest_centroid`/`nearest_n_centroids` (en rango `0..centroids.len()`). Pero tras `load_index` con bytes hostiles, `id_to_cluster` puede mapear a índices `>= cluster_members.len()`, o `cluster_members` puede ser más corto que `centroids` → panic en el primer `add` o `search` posterior.
- Escenario de fallo: `.mmdb` crafteado con IVF serializado donde `id_to_cluster = {"x": 50}` pero `cluster_members` tiene 2 entradas. `VectorDB::open` (ivf no tiene bloque propio en `IndexBlocks` según db.rs:700-703, así que el path IVF-hostile directo es vía `load_index` expuesto por el trait o por HNSW-block-fail fallback que reconstruye; en `open` IVF siempre se reconstruye porque `IndexBlocks.hnsw` es None o falla). **Matiz:** por db.rs:167-186, IVF sólo carga estado serializado si existiera un bloque dedicado —actualmente `IndexBlocks` sólo expone `hnsw` y `bm25`—, así que en la práctica `open` reconstruye IVF y el panic no se da por esa vía. El panic sí es alcanzable llamando `IVFIndex::load_index` directamente (trait público vía `Index::load_index`) o si en el futuro se persiste el bloque IVF.
- Verificación: `load_index` (ivf.rs:390-395) deserialize sin validar. Indexación sin bounds-check en `:252` y `:296`. `nearest_centroid`/`nearest_n_centroids` recorren `centroids` y devuelven índices en `0..centroids.len()`, pero eso no acota `cluster_members` cuando el estado es hostil. Marcado MEDIUM (no CRITICAL) porque vía `VectorDB::open` actual no se persiste bloque IVF; alcanza con uso directo del trait o cambio futuro del formato.

---

## [MEDIUM] `rebuild` de HNSW ignora la métrica configurada y construye el grafo siempre con `Cosine`

- Archivo: `src/index/hnsw.rs:641-643`
- Código:
```rust
// Re-insertar todos los vectores
for (id, vector) in entries {
    self.add(&id, &vector, storage, Distance::Cosine)?;
}
```
- Problema: `HNSWIndex::rebuild` hardcodea `Distance::Cosine` al reinsertar, independientemente de la métrica con la que se construyó/originalmente se usaría. Si el índice se usaba con `Euclidean`/`Manhattan`/`DotProduct`, tras `rebuild` el grafo se reconstruye con distancias coseno, mientras que `search` (hnsw.rs:547) usa la `Distance` que el caller pase (la configurada en `VectorDB`). Esto produce un grafo inconsistente con la métrica de consulta → degradación de recall / resultados incorrectos.
- Escenario de fallo: Llamar `index.rebuild(storage)` sobre un `HNSWIndex` usado con `Distance::Euclidean` y luego buscar con Euclidean. El grafo se armó con coseno, la búsqueda navega con euclidiana → vecinos seleccionados durante la construcción no son los óptimos para la métrica de query.
- Verificación: `HNSWIndex::rebuild` es el único `rebuild` que hardcodea la métrica (`FlatIndex::rebuild` no calcula distancias; `IVFIndex::rebuild` usa `sq_euclidean` interna por diseño). **Alcanzabilidad:** `grep` confirma que `HNSWIndex::rebuild` (el método del trait) **no** es invocado por `VectorDB` ni por `PartialIndex` (el `rebuild` de `partial_index.rs:237-258` llama a `try_add`→`index.add`, no al `rebuild` del trait). Sólo tests lo llaman. Por eso MEDIUM (bug latente, no disparable por la API pública actual, pero sí si se invoca `rebuild` directamente o se integra en el futuro).

---

## [MEDIUM] Detección de features AVX2/AVX-512 recortada por `#[cfg(target_feature = ...)]` — la "selección automática en runtime" es parcial

- Archivo: `src/distance/simd.rs:27-48` (euclidean), `:64-86` (cosine), `:101-123` (dot)
- Código:
```rust
#[cfg(target_arch = "x86_64")]
{
    // AVX-512 (16 floats por operación)
    #[cfg(target_feature = "avx512f")]
    {
        if is_x86_feature_detected!("avx512f") && a.len() >= 16 {
            return unsafe { euclidean_avx512(a, b) };
        }
    }
    // AVX2 (8 floats por operación)
    #[cfg(target_feature = "avx2")]
    {
        if is_x86_feature_detected!("avx2") && a.len() >= 8 {
            return unsafe { euclidean_avx2(a, b) };
        }
    }
    ...
```
- Problema: Las ramas AVX2/AVX-512 están tras `#[cfg(target_feature = "avx2")]`/`#[cfg(target_feature = "avx512f")]`, que son **compile-time**. Salvo que el crate se compile con `target-cpu=native` o `target-feature=+avx2,+avx512f`, esas ramas no se compilan y la detección runtime (`is_x86_feature_detected!`) es código muerto. La doc del módulo afirma "selecciona automáticamente la mejor implementación disponible en tiempo de ejecución" (simd.rs:7-9), pero en un build default sólo SSE (baseline x86_64) está disponible; AVX2/AVX-512 no se usan por banyakas CPUs modernas. No es un bug de corrección (SSE da resultados correctos), pero la promesa de runtime-selection y el rendimiento anunciado no se cumplen en builds estándar.
- Escenario de fallo: Build normal (`cargo build`), CPU con AVX2. `euclidean_distance` con len≥8 → entra SSE (4-wide) en vez de AVX2 (8-wide). Rendimiento ~la mitad del esperado; sin error.
- Verificación: `cargo rustc -- --print cfg` muestra sólo `target_feature="sse"` (no avx2/avx512f). La guardia `is_x86_feature_detected!("avx2")` está dentro del `#[cfg(target_feature = "avx2")]` que no se compila → la función avx2 ni siquiera existe en el binario (está marcada `#[allow(dead_code)]` precisamente para no alertar cuando se compila fuera). SSE sí se compila y ejecuta porque `target_feature="sse"` es default en x86_64.

---

## [LOW] `random_level` puede retornar un nivel enorme ante `r == 0.0` → OOM/panic en inserción HNSW

- Archivo: `src/index/hnsw.rs:188-192`
- Código:
```rust
fn random_level(&self) -> usize {
    let mut rng = rand::thread_rng();
    let r: f64 = rng.gen();
    (-r.ln() * self.ml).floor() as usize
}
```
- Problema: `rand::Rng::gen::<f64>()` (distribución `Standard`) devuelve un valor en `[0, 1)` y **puede** retornar exactamente `0.0` (cuando el generador produce bits todos cero en la mantissa; probabilidad ~2^-53). Si `r == 0.0`, `r.ln() == -f64::INFINITY`, `(-r.ln() * self.ml).floor() == f64::INFINITY`, y `f64::INFINITY as usize` satura a `usize::MAX` (cast float→int saturante desde Rust 1.45, definido — no UB). Luego en `add` (hnsw.rs:412-421 / 427-438) `while inner.levels.len() <= node_level` intenta crear `usize::MAX` niveles → alloc error → `handle_alloc_error` → abort/panic. Igualmente, `r` subnormal muy pequeño produce un nivel grande pero finito que puede llevar a reservar muchísimos niveles.
- Escenario de fallo: `VectorDB::new(Config::new(d).with_index(IndexType::hnsw()))` y muchas `db.insert(...)`. Con probabilidad ~1/2^53 por inserción, una de ellas entra en el caso `r==0` y el proceso aborta por OOM al asignar niveles.
- Verificación: `rand 0.8` `Standard` para f64 produce `f64_from_bits` en `[0,1)`, incluyendo el 0. El cast `INFINITY as usize` satura (no UB) → el bucle `while inner.levels.len() <= usize::MAX` fuerza la asignación hasta OOM. LOW por la probabilidad extremadamente baja, pero alcanzable teóricamente desde `VectorDB::insert`.

---

## [LOW] `IVFIndex::rebuild` con `num_clusters == 0` y `n > 0` hace `assert!` (panic) dentro de `kmeans_pp_init`

- Archivo: `src/index/ivf.rs:131-133` y `:352-354`
- Código (ivf.rs:131-133):
```rust
fn kmeans_pp_init(vectors: &[Vec<f32>], k: usize) -> Vec<Vec<f32>> {
    let n = vectors.len();
    assert!(k > 0 && n > 0);
```
- Código (ivf.rs:351-354):
```rust
let k = self.num_clusters.min(vectors.len());
let (centroids, assignments) = kmeans(&vectors, k, 20);
```
- Problema: `IVFIndex::new(num_clusters, num_probes)` no acota `num_clusters` a `>= 1` (sólo `num_probes = num_probes.min(num_clusters).max(1)`, ivf.rs:104). Si `num_clusters == 0` y se llama `rebuild` con vectores, `k = 0.min(n) = 0` y `kmeans_pp_init` hace `assert!(k > 0 ...)` → panic. `add`/`search` con `num_clusters==0` están protegidos (la rama no-entrenada y los `.is_empty()`), así que el panic vive sólo en `rebuild`.
- Escenario de fallo: `IVFIndex::new(0, 1)`, añadir vectores, llamar `rebuild(&storage)` con datos → panic por assert.
- Verificación: `IVFIndex::new` (ivf.rs:103-110) no valida `num_clusters >= 1`. `kmeans_pp_init` assertion en `:132`. **Alcanzabilidad:** como se documenta en el hallazgo HIGH de IVF, `VectorDB` no llama `self.index.rebuild()`, y `PartialIndex::rebuild` usa `try_add` (no `kmeans`). Por eso LOW: el panic no se dispara desde la API pública actual; sólo llamando `rebuild` directamente o si se integra persistencia/trigger de rebuild IVF.

---

## [LOW] `FlatIndex::search` ignora su propio set `ids` y recorre todo el storage; `len()` puede divergir del conjunto buscable

- Archivo: `src/index/flat.rs:79-118` (search) y `:129-131` (len)
- Código (flat.rs:90-92):
```rust
for stored in storage.iter_with_vectors() {
    if let Some(vec) = stored.vector.as_ref() {
        let dist = distance.calculate(query, vec);
```
- Código (flat.rs:129-131):
```rust
fn len(&self) -> usize {
    self.ids.read().len()
}
```
- Problema: `FlatIndex::search` itera `storage.iter_with_vectors()` y nunca consulta `self.ids`. Así, el resultado de búsqueda se basa exclusivamente en el storage, no en el set de IDs indexados. Si el storage y el set `ids` se desincronizan (p.ej. alguien usa el trait `Index` directamente y borra del storage sin `index.remove`, o inserta en storage sin `index.add`), la búsqueda encuentra vectores no presentes en `ids` y `len()` reporta un número distinto del conjunto realmente buscable. vía `VectorDB` esto no se da (add/remove están emparejados en `db.rs`), por lo que es deuda de diseño del trait, no un bug funcional vía la API principal.
- Escenario de fallo: Uso directo del trait: `storage.insert("x", Some(v), None)` sin `index.add("x",...)`. `index.search` encuentra "x"; `index.len()` no lo cuenta. Inversamente, `index.remove("x")` sin `storage.delete` → `index.len()` baja pero `search` sigue retornando "x".
- Verificación: `FlatIndex::search` (flat.rs:79-118) no referencia `self.ids`. `len()` (flat.rs:129) sí usa `ids`. `VectorDB::insert/delete/update` mantienen ambos en sync, así que no hay divergencia observables por la API pública → LOW.

---

## [LOW] Tokenizador BM25 filtra tokens de 1 byte y tokeniza CJK de forma no idiomática

- Archivo: `src/index/bm25.rs:282-288`
- Código:
```rust
fn tokenize(&self, text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty() && s.len() > 1)
        .map(|s| s.to_string())
        .collect()
}
```
- Problema: El filtro `s.len() > 1` usa **longitud en bytes**, no número de caracteres. Un token ASCII de 1 carácter ('a', '1') se descarta (len 1), pero un carácter Unicode multi-byte ('é' = 2 bytes, un CJK = 3 bytes) se acepta aunque sea un único carácter. Para texto CJK, cada Hanzi se convierte en su propio token (no hay segmentación de palabras), lo que degrada la calidad del ranking BM25. Para ASCII, descartar tokens de 1 char es una decisión de diseño (probablemente intencionada para ruido), pero es inconsistente con el caso Unicode. No es crash ni corrupción: sólo calidad de búsqueda.
- Escenario de fallo: `index.add("d", Some(&meta con content="我")` ) → token "我" (3 bytes) indexado. Query "我" lo encuentra. Pero `content="a"` → token "a" descartado, query "a" no encuentra nada.
- Verificación: `is_alphanumeric()` es unicode-aware (`char::is_alphanumeric`). `s.len()` es byte length para `&str`. `to_lowercase()` es unicode-aware (no panic con entradas raras; strings vacíos se filtran por `!s.is_empty()`). LOW (calidad, no corrección).

---

## Cobertura

Leídos COMPLETOS (en este orden):
- `src/index/mod.rs` (1–121)
- `src/index/flat.rs` (1–201)
- `src/index/hnsw.rs` (1–1131)
- `src/index/ivf.rs` (1–638)
- `src/index/bm25.rs` (1–488)
- `src/distance/mod.rs` (1–106)
- `src/distance/simd.rs` (1–766)

Leídos para contexto / alcanzabilidad:
- `src/db.rs` (1–2038, foco en API pública `VectorDB`: insert/insert_document/search/delete/update/open/open_with_fulltext/save, y ausencia de `self.index.rebuild`)
- `src/storage/mod.rs` (trait `Storage`)
- `src/types.rs` (`SearchResult`, `StoredVector`, `Config`)
- `src/lib.rs` (re-export de `Distance`)
- `src/partial_index.rs` (para verificar si `rebuild` del trait `Index` se invoca vía índices parciales: no, usa `try_add`)

Comandos ejecutados (read-only):
- `cargo check --message-format=short` → OK (1 warning ajeno: `storage/disk.rs:40 none` never used)
- `cargo rustc -- --print cfg | grep target_feature` → confirma `sse` on, `avx2`/`avx512f` off en build default
- `grep -n "\.rebuild\(" src/` → callers del trait `rebuild`

---

## Sólido (verificado correcto)

- **HNSW remove**: reusa `free_indices` (hnsw.rs:534), limpia `level.neighbors[idx]` Y remueve `idx` de las listas ajenas con `retain` (hnsw.rs:506-511) → **no deja aristas colgantes hacia nodos borrados**. Repara `entry_point` seleccionando el nodo de mayor nivel vía `node_levels` (hnsw.rs:519-530) y baja `max_level` en consecuencia. `test_hnsw_delete_entry_point` y `test_hnsw_index_reuse` pasan y reflejan estos invariantes. Re-add tras delete reutiliza el idx y reasigna `idx_to_id`/`id_to_idx`/`node_levels` correctamente.
- **HNSW entry_point.unwrap()** seguro: tanto `add` (hnsw.rs:406) como `search` (hnsw.rs:551) validan `is_none()` antes del `unwrap`.
- **HNSW `select_neighbors` / pruning con `select_nth_unstable_by`**: el índice `m-1`/`m_max-1` está siempre guardado por `if sorted.len() > m` con `m >= 2` (`m = m.max(2)`, hnsw.rs:144; `m_max0 = m*2`). No hay panic por índice fuera de rango.
- **HNSW serialización round-trip**: `serialize_index`/`load_index` (hnsw.rs:663-674) bincode sobre `HNSWInner` (todos los campos derivan `Serialize/Deserialize`); `test_hnsw_serialization_roundtrip` verifica misma longitud y mismos resultados de búsqueda. Sin datos hostiles es fiel.
- **ef_search atómico**: `AtomicUsize` con `Relaxed` (hnsw.rs:179, 184); `set_ef_search` acota a `>= 1`. Aceptable para un counter que se lee una vez por búsqueda.
- **IVF k-means++**: maneja `k > n` con `k = k.min(n)` (ivf.rs:133, 180); clusters vacíos conservan el centroide viejo (`if counts[c] == 0 { continue }`, ivf.rs:220-222); todos los puntos idénticos → `total <= 0.0` empuja un centroide aleatorio (ivf.rs:152-156) y converge sin panic. `test_ivf_fewer_vectors_than_clusters` cubre k>n.
- **IVF nprobe**: `num_probes = num_probes.min(num_clusters).max(1)` (ivf.rs:104); `nearest_n_centroids` usa `take(n)` que no panic si `n > centroids.len()` (ivf.rs:424).
- **IVF rebuild tras deletes**: `rebuild` (ivf.rs:334-373) releé todo el storage y reentrena desde cero, por lo que las estadísticas quedan consistentes tras borrados.
- **BM25 IDF no negativo**: la fórmula `((n - df + 0.5) / (df + 0.5) + 1.0).ln()` (bm25.rs:231) es la variante BM25+/Lucene; el `+1.0` dentro del `ln` garantiza `idf >= 0` incluso cuando `df > n/2` o `df == n`. Verificado: para `df == n`, `idf = ln(1 + 0.5/(n+0.5)) > 0`. No hay IDF negativo.
- **BM25 división por `avgdl == 0`**: `avgdl = total_doc_length / n` (bm25.rs:219). Si `avgdl == 0` entonces todo `total_doc_length == 0`, lo que implica que todos los docs tienen `length == 0` y por tanto `term_frequencies` vacíos (bm25.rs:107-119) → `doc_frequencies`/`inverted_index` vacíos → para cada query token `df == 0.0` y se hace `continue` (bm25.rs:227-229) **antes** de llegar a la división. La división por cero no es alcanzable.
- **BM25 consistencia tras add/update/delete**: `add` (bm25.rs:135-153) y `remove` (bm25.rs:176-194) restan `old_doc.length` de `total_doc_length` (`saturating_sub`), eliminan el doc del `inverted_index` y decrementan `doc_frequencies` (eliminando términos con count 0). `update` = `remove`+`add` en `db.rs:608-609`. Stats `df`/`tf`/`total_doc_length` quedan consistentes. `test_remove` lo cubre.
- **BM25 strings vacíos / unicode**: `tokenize` filtra vacíos (`!s.is_empty()`); `to_lowercase` y `is_alphanumeric` son unicode-aware, no panic con entradas raras.
- **SIMD tail handling (longitudes no múltiplo del ancho)**: para longitudes iguales, cada ruta SIMD procesa `chunks = len / W` y luego el resto con un loop escalar sobre `remainder_start..a.len()` (e.g. simd.rs:338-345, 548-555). Correcto para `a.len() == b.len()`.
- **SIMD consistencia scalar vs SIMD (longitudes iguales)**: las rutas SSE/AVX2/NEON usan FMA; las escalares usan `d*d + sum`. Diferencias numéricas dentro de la tolerancia de los tests (1e-4). `test_euclidean_simd`, `test_cosine_simd`, `test_dot_product_simd` comparan ambas rutas y pasan.
- **Concurrencia**: cada índice usa `parking_lot::RwLock` (`FlatIndex.ids`, `HNSWIndex.inner`, `IVFIndex.inner`, `BM25Index.inner`). `add`/`remove` toman write, `search` toma read. `HNSWIndex::rebuild` libera el write antes de llamar a `self.add` (`drop(inner)`, hnsw.rs:638) → no reentrancia/deadlock. `test_hnsw_concurrent_access` ejercita lecturas y escrituras concurrentes.
- **`MaxSearchResult`/`Candidate`/`MaxCandidate`** con `partial_cmp().unwrap_or(Ordering::Equal)`: no panic ante NaN/Inf (a costa de orden no total — ver hallazgo HIGH de NaN).