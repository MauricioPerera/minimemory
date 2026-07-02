# Auditoría READ-ONLY — Core Storage (minimemory)

Alcance: `src/db.rs`, `src/storage/disk.rs`, `src/storage/format.rs`, `src/storage/memory.rs`, `src/storage/mod.rs`, `src/error.rs`, `src/types.rs`.
Fecha: 2026-07-01. Auditor: senior Rust, modo read-only. No se modificó ningún archivo del repo.

Convención de severidad usada: CRITICAL = corrupción de datos / UB / panic trivial desde API pública; HIGH = bug funcional real; MEDIUM = manejo de errores débil o riesgo condicional; LOW = calidad/deuda.

---

## [CRITICAL] `update` no valida dimensiones del vector → panic por index out of bounds en `search`

- Archivo: `src/db.rs:581-604`
- Código:
```rust
pub fn update(
    &self,
    id: impl Into<VectorId>,
    vector: &[f32],
    metadata: Option<Metadata>,
) -> Result<()> {
    let id = id.into();

    // Step 1: Update storage in-place (overwrite, no gap)
    // First delete old entry, then immediately insert new one
    self.storage.delete(&id)?;
    if let Some(ref quantizer) = self.quantizer {
        let qvec = quantizer.quantize(vector)?;
        self.storage
            .insert_quantized(id.clone(), qvec, metadata.clone())?;
    } else {
        self.storage
            .insert(id.clone(), Some(vector.to_vec()), metadata.clone())?;
    }

    // Step 2: Update index (remove old, add new)
    self.index.remove(&id)?;
    self.index
        .add(&id, vector, &*self.storage, self.config.distance)?;
```
- Problema: `insert` (db.rs:353), `insert_document` (db.rs:433) y `insert_chunk` (db.rs:911) validan `vector.len() != self.config.dimensions` y devuelven `DimensionMismatch`. `update` **no**. En la rama no-cuantizada almacena el vector de dimensión errónea tal cual (`MemoryStorage::insert` no valida, ver `src/storage/memory.rs:30-44`). El vector erróneo queda en storage y se pasa a `index.add`. En el siguiente `search`, `FlatIndex::search` itera `storage.iter_with_vectors()` y llama `distance.calculate(query, vec)` con `query.len() == config.dimensions` y `vec.len()` distinto.
- Escenario de fallo:
  ```rust
  let db = VectorDB::new(Config::new(3).with_distance(Distance::Cosine).with_index(IndexType::Flat)).unwrap();
  db.insert("a", &[1.0, 0.0, 0.0], None).unwrap();
  db.update("a", &[1.0], None).unwrap();          // NO valida dimensión; almacena len=1
  db.search(&[1.0, 0.0, 0.0], 1).unwrap();        // PANIC
  ```
- Verificación: `cosine_scalar`/`cosine_avx2` iteran usando `a.len()` y indexan `b[i]` / `b.as_ptr().add(offset)` sin comprobar `b.len() >= a.len()`:
  - `src/distance/simd.rs:472-475`: `for i in remainder_start..a.len() { dot += a[i] * b[i]; ... }`
  - `src/distance/simd.rs:458-459`: `_mm256_loadu_ps(b.as_ptr().add(offset))` (lectura OOB = UB en ruta SIMD).
  Con `a = query` (len 3) y `b = vec` (len 1), el escalar hace `b[2]` → index out of bounds (panic); la ruta AVX2/SSE lee 4–8 floats fuera del slice `b` (UB). `Euclidean`/`Manhattan`/`DotProduct` tienen el mismo patrón (`simd.rs:148-170`, `449-484`, `490-508`). Alcance: API pública `VectorDB::update` + `VectorDB::search`.

## [CRITICAL] `load_vectors` reserva memoria controlada por el header del archivo → OOM/abort con `.mmdb` hostil

- Archivo: `src/storage/disk.rs:193`
- Código:
```rust
    // Leer vectores
    let mut vectors = Vec::with_capacity(header.num_vectors as usize);
```
- Problema: `header.num_vectors` es un `u64` leído del archivo (`format.rs:163`). En target 64-bit, `as usize` preserva valores enormes. `Vec::with_capacity` con un valor gigante dispara `handle_alloc_error` → abort del proceso (no `Result`, no unwind). Un archivo `.mmdb` craftado con `num_vectors = 0xFFFF_FFFF_FFFF_FFFF` aborta el cargador.
- Escenario de fallo: archivo hostil con header `MMDB` + `version=3` + `num_vectors = u64::MAX`. `VectorDB::open(path)` → `disk::load_vectors` → abort.
- Verificación: `format.rs:162-163` lee `num_vectors` sin validación de rango; `disk.rs:193` lo usa directo como capacidad. `VectorDB::open` (db.rs:136) y `open_with_fulltext` (db.rs:211) llaman a `load_vectors` sin filtrar el header. API pública, archivo de entrada no confiable.

## [CRITICAL] `load_vectors`: `data.resize(len, 0)` con `len` hasta 4 GiB por entrada → OOM/abort

- Archivo: `src/storage/disk.rs:203-209`
- Código:
```rust
        let len = u32::from_le_bytes(buf4) as usize;

        hasher.update(&buf4);

        // Leer datos (reuse buffer, only grows if needed)
        data.resize(len, 0);
        reader.read_exact(&mut data[..len])?;

        hasher.update(&data[..len]);
```
- Problema: el prefijo de longitud de cada entrada es un `u32` little-endian tomado literalmente del archivo. `data.resize(len, 0)` intenta asignar hasta 4 GiB por entrada. Con `num_vectors` pequeño pero `len = 0xFFFF_FFFF`, una sola entrada aborta por OOM. El buffer `data` se reutiliza pero solo crece (`only grows if needed`), así que el primer `len` grande fija un asignación enorme.
- Escenario de fallo: `.mmdb` con `num_vectors = 1`, prefijo de longitud `0xFFFF_FFFF` → `data.resize(4_294_967_295, 0)` → abort.
- Verificación: `u32::from_le_bytes` sin cota; `disk.rs:196` inicializa `data` con capacidad 4096 pero el `resize` la eleva a `len`. Mismo patrón en el bloque de índice: `disk.rs:245-247` hace `let mut block_data = vec![0u8; block_len];` con `block_len: u32` → misma clase de OOM por bloque HNSW/BM25 hostil. API pública vía `open`/`open_with_fulltext`.

## [CRITICAL] `bincode::deserialize` de `VectorEntry` sin límite → OOM por campo `Vec<f32>` con longitud u64 hostil

- Archivo: `src/storage/disk.rs:214`
- Código:
```rust
        // Deserializar
        let entry: VectorEntry = bincode::deserialize(&data)?;

        vectors.push(StoredVector {
```
- Problema: `VectorEntry.vector: Option<Vec<f32>>` (`format.rs:249`) se serializa con bincode 1.3 en configuración por defecto (`Cargo.toml:25` `bincode = "1.3"`), que usa longitudes `u64` para colecciones y **no** impone límite de asignación. El buffer `data` está acotado por el prefijo `u32` (≤4 GiB), pero el `u64` interno del `Vec<f32>` puede reclamar, p.ej., `10_000_000_000` elementos → bincode intenta `Vec::with_capacity(10_000_000_000)` antes de leer → abort por OOM. Lo mismo aplica a `entry.metadata` (HashMap) y `entry.id` (String) con su `u64` de longitud.
- Escenario de fallo: `.mmdb` con una entrada cuyo payload bincode codifica `vector = Some(Vec<f32>)` con longitud `u64 = 0x0000_0002_0000_0000` (≈8.6e9) → `deserialize` aborta aunque el prefijo `u32` de la entrada sea pequeño.
- Verificación: bincode 1.3 default no aplica `limit()`; `VecVisitor` asigna según el `u64` leído. `VectorDB::open` expone esto a entrada no confiable.

## [HIGH] `update` borra del storage antes de cuantizar; si la cuantización falla, deja storage/índice/bm25 inconsistentes

- Archivo: `src/db.rs:591-599`
- Código:
```rust
    self.storage.delete(&id)?;
    if let Some(ref quantizer) = self.quantizer {
        let qvec = quantizer.quantize(vector)?;
        self.storage
            .insert_quantized(id.clone(), qvec, metadata.clone())?;
    } else {
        self.storage
            .insert(id.clone(), Some(vector.to_vec()), metadata.clone())?;
    }
```
- Problema: en DB cuantizada, `quantizer.quantize(vector)?` valida dimensión (`src/quantization.rs:511-515`) y devuelve `DimensionMismatch` si el vector viene con dimensión errónea. Pero la ejecución ya pasó `self.storage.delete(&id)?` (línea 591): el documento viejo fue eliminado del storage, mientras el índice vectorial, BM25 y los índices parciales aún referencian el `id`. La operación aborta sin rollback.
- Escenario de fallo:
  ```rust
  // DB Int8, dim 64
  db.insert("a", &v64, None).unwrap();
  db.update("a", &[1.0, 2.0], None).unwrap_err();  // quantize -> DimensionMismatch
  // storage ya no tiene "a"; index/bm25 sí. Documento perdido + referencias colgantes.
  db.search(&query64, 10)  // el índice puede devolver "a"; storage.get("a") -> None
  ```
- Verificación: `Quantizer::quantize` devuelve `Err(DimensionMismatch)` (`quantization.rs:512-515`). `update` no hace dimension-check previo y no revierte el `delete`. `search_partial` (db.rs:819) y `search` terminan haciendo `storage.get(&id)` sobre ids que el índice aún referencia → `None` inesperado / resultados huérfanos. API pública.

## [HIGH] Carrera TOCTOU en `insert`: el chequeo `contains` y la inserción no son atómicos (sin lock global)

- Archivo: `src/db.rs:360-376`
- Código:
```rust
        if self.storage.contains(&id) {
            return Err(Error::AlreadyExists(id));
        }

        // Store quantized or full vector
        if let Some(ref quantizer) = self.quantizer {
            let qvec = quantizer.quantize(vector)?;
            self.storage
                .insert_quantized(id.clone(), qvec, metadata.clone())?;
        } else {
            self.storage
                .insert(id.clone(), Some(vector.to_vec()), metadata.clone())?;
        }

        // Index always uses f32 for graph construction (HNSW needs precise distances)
        self.index
            .add(&id, vector, &*self.storage, self.config.distance)?;
```
- Problema: `VectorDB` (db.rs:42-54) **no** tiene un lock global; `storage`, `index`, `bm25_index` y `partial_indexes` son `Arc<dyn ...>` con locks internos independientes (parking_lot). `contains` toma y libera el read-lock del storage; luego `insert` toma el write-lock por separado. Dos hilos insertando el mismo `id` concurrentemente pueden pasar ambos el `contains` (ambos ven `false`) → ambos proceden → ninguno recibe `AlreadyExists`. `MemoryStorage::insert` sobrescribe silenciosamente (`memory.rs:42`), y `index.add` se llama dos veces con el mismo `id` (en HNSW/IVF puede crear nodos duplicados).
- Escenario de fallo: dos threads llaman `db.insert("x", &v, None)` simultáneamente → ambos `contains("x") == false` → ambos insertan → `db.len() == 1` pero `index.len()` puede ser 2; el contrato `AlreadyExists` se viola bajo concurrencia.
- Verificación: la struct `VectorDB` no envuelve nada en un `Mutex`/`RwLock` (db.rs:42-54); cada componente se lockea por separado (`memory.rs:12`, `flat.rs:45`). `insert_batch` (db.rs:481-489) itera llamando a `insert` sin atomicidad entre lotes. API pública diseñada como `Send + Sync`.

## [HIGH] `insert` no es transaccional: si `index.add` o `bm25.add` fallan tras `storage.insert`, el estado queda inconsistente (sin rollback)

- Archivo: `src/db.rs:365-386`
- Código:
```rust
        if let Some(ref quantizer) = self.quantizer {
            let qvec = quantizer.quantize(vector)?;
            self.storage
                .insert_quantized(id.clone(), qvec, metadata.clone())?;
        } else {
            self.storage
                .insert(id.clone(), Some(vector.to_vec()), metadata.clone())?;
        }

        // Index always uses f32 for graph construction (HNSW needs precise distances)
        self.index
            .add(&id, vector, &*self.storage, self.config.distance)?;

        // Indexar en BM25 si está habilitado
        if let Some(ref bm25) = self.bm25_index {
            bm25.add(&id, metadata.as_ref())?;
        }

        // Añadir a índices parciales que coincidan
        let _ = self
            .partial_indexes
            .on_insert(&id, vector, metadata.as_ref());
```
- Problema: la inserción muta cuatro componentes separados sin compensación. Si `index.add` devuelve `Err` después de que `storage.insert` tuvo éxito, el documento queda en storage pero no en el índice (para HNSW/IVF significa que `search` nunca lo encuentra). Si `bm25.add` falla después de storage+index, `keyword_search`/`hybrid_search` no lo encuentran pese a estar almacenado. Ningún paso revierte los anteriores.
- Escenario de fallo: `bm25.add` puede devolver error (p.ej. tokenización / límites internos del BM25); `insert` retorna `Err` pero el documento ya está en storage e index → `db.get("x")` funciona, `db.keyword_search(...)` no lo devuelve; estado silenciosamente inconsistente.
- Verificación: `insert_batch` documenta "Si alguna inserción falla, las anteriores no se revierten" (db.rs:480) pero el problema existe por inserción individual. `delete` (db.rs:558-570) tiene el mismo patrón: `storage.delete` ok → `index.remove` err → documento fuera de storage pero aún en índice. `update`/`insert_document`/`insert_chunk` repiten la estructura multi-componente sin rollback.

## [HIGH] `load_vectors` ignora silenciosamente truncamiento: archivo cortado se carga como una DB más chica sin error

- Archivo: `src/storage/disk.rs:198-222`
- Código:
```rust
    for _ in 0..header.num_vectors {
        // Leer longitud
        if reader.read_exact(&mut buf4).is_err() {
            break;
        }
        let len = u32::from_le_bytes(buf4) as usize;
        ...
    }
```
- Problema: el loop de lectura usa `break` silencioso cuando `read_exact` del prefijo de longitud falla (línea 200). Si el archivo está truncado exactamente en el límite entre entradas, `load_vectors` devuelve menos vectores que `header.num_vectors` **sin error**: pérdida silenciosa de datos. Combinado con la verificación de CRC32 (ver hallazgo MEDIUM siguiente), el checksum tampoco se verifica porque el footer no está → la carga truncada pasa completamente sin validar.
- Escenario de fallo: `db.save()` produce un `.mmdb` con N vectores; un `truncate` del SO / transferencia interrumpida deja el archivo con N-k entranas. `VectorDB::open` carga N-k vectores y devuelve `Ok`, perdiendo k documentos sin señal.
- Verificación: la rama de error del `read_exact` del prefijo es `break` (no `?`); la del payload `data[..len]` sí usa `?` (línea 209), así que la truncación en medio de un payload sí reporta error, pero la truncación en el límite de entrada no. `disk.rs:263-264` luego salta el CRC si no quedan ≥8 bytes para el footer.

## [HIGH] Verificación CRC32 evitable: checksum `0` se omite, y archivo sin footer se acepta sin validar

- Archivo: `src/storage/disk.rs:263-283`
- Código:
```rust
    let current_pos = reader.stream_position().unwrap_or(0);
    if current_pos + 8 <= file_len {
        let mut checksum_buf = [0u8; 4];
        let mut end_marker = [0u8; 4];

        if reader.read_exact(&mut checksum_buf).is_ok()
            && reader.read_exact(&mut end_marker).is_ok()
            && &end_marker == b"END!"
        {
            let stored_checksum = u32::from_le_bytes(checksum_buf);
            // Only verify if checksum is non-zero (v1 files wrote 0)
            if stored_checksum != 0 {
                let computed = hasher.finalize();
                if computed != stored_checksum {
                    return Err(Error::InvalidConfig(format!(
                        "CRC32 checksum mismatch: expected {:08x}, got {:08x}. File may be corrupted.",
                        stored_checksum, computed
                    )));
                }
            }
        }
    }
```
- Problema: (1) Si `current_pos + 8 > file_len` (archivo truncado sin footer) la verificación se salta por completo → carga sin validar integridad. (2) `if stored_checksum != 0` permite a un archivo hostil escribir `checksum = 0` para bypassear el CRC aunque sea v3 — la intención era compatibilidad con v1 pero abre un bypass: cualquier payload corrupto/craftado con checksum 0 se acepta. (3) Si `end_marker != "END!"` la verificación se salta silenciosamente (sin error) en vez de reportar formato roto.
- Escenario de fallo: `.mmdb` v3 con `checksum = 0x00000000` y datos alterados → `load_vectors` devuelve `Ok` con datos corruptos (p.ej. floats cambiados, ids manipulados). O archivo truncado sin footer → `Ok` con datos parciales.
- Verificación: condiciones de guarda en `disk.rs:264`, `268-270` y `274`. El test `test_crc32_detects_corruption` (disk.rs:413) solo cubre el caso donde el footer está intacto y el checksum es no-cero; no cubre checksum=0 ni footer ausente. API pública vía `open`.

## [MEDIUM] Overflow aritmético en `search_with_filter`/`keyword_search`/`hybrid_search`: `k * 10` / `k * 3` puede desbordar `usize`

- Archivo: `src/db.rs:1180-1194` (caller en alcance); desbordamiento en `src/search/hybrid.rs:224`, `:271`, `:317` (fuera de alcance, verificado)
- Código:
```rust
    pub fn search_with_filter(
        &self,
        query: &[f32],
        k: usize,
        filter: Filter,
    ) -> Result<Vec<SearchResult>> {
        ...
        let params = HybridSearchParams::vector(query.to_vec(), k).with_filter(filter);
        let hybrid_results = self.hybrid_search(params)?;
```
```rust
        // Buscar más resultados si hay filtro (pre-filter approach)
        let search_k = if params.filter.is_some() {
            params.k * 10 // Buscar 10x más para compensar filtrado
        } else {
            params.k
        };
```
- Problema: `k` es `usize` sin validar proveniente de la API pública. Con `filter`, `hybrid_search`/`vector_search`/`keyword_search` calculan `params.k * 10` y `params.k * 3`. Con `k` grande (p.ej. `usize::MAX / 2`) esto desborda: panic en debug (`overflow`), wrap en release → `index.search` con `k` chico → resultados incorrectos sin error. Mismo patrón en `keyword_search` (`k * 10` cuando hay filtro) y `hybrid_search` (`k * 3` para RRF).
- Escenario de fallo: `db.search_with_filter(&q, usize::MAX, filter)` → `search_k = usize::MAX * 10` → panic en debug builds.
- Verificación: `search_with_filter` (db.rs:1193) y `keyword_search` (db.rs:1127) son públicos y pasan `k` sin cota; `hybrid.rs:224/271/317` multiplican sin `saturating_mul`. `filter_search_ordered` (db.rs:1303) usa `usize::MAX` pero cae en modo `FilterOnly`, que acota a 100_000 (`hybrid.rs:426-427`), por eso no dispara este path. PLAUSIBLE solo en builds debug / resultados silenciosamente truncados en release.

## [MEDIUM] Overflow en `search_paged`: `offset + limit` puede desbordar `usize`

- Archivo: `src/db.rs:1354`
- Código:
```rust
        // Fetch enough results for offset + limit
        let fetch_k = offset + limit;
        let all_results = self
            .index
            .search(query, fetch_k, self.storage.as_ref(), self.config.distance)?;
```
- Problema: `offset` y `limit` son `usize` de la API pública sin validación. `offset + limit` desborda en debug (panic) y envuelve en release (fetch_k pequeño → página vacía/wrong sin error).
- Escenario de fallo: `db.search_paged(&q, 1, usize::MAX)` → `fetch_k = usize::MAX + 1` → panic en debug.
- Verificación: `search_paged` (db.rs:1328-1367) es público; `offset`/`limit` no se acantan. `list_documents` (db.rs:1282) usa `skip(offset).take(limit)` que sí es seguro ante overflow (lazy iterators), así que el problema es específico de `search_paged`.

## [MEDIUM] `list_documents` y la búsqueda híbrida filtran un campo mágico `metadata["deleted"] == true` de forma inconsistente y no documentada

- Archivo: `src/db.rs:1242-1246`
- Código:
```rust
            .filter(|doc| {
                // Skip soft-deleted if metadata has deleted flag
                if let Some(ref meta) = doc.metadata {
                    if let Some(crate::types::MetadataValue::Bool(true)) = meta.get("deleted") {
                        return false;
                    }
                }
```
- Problema: `list_documents` oculta documentos cuyo metadata tiene un campo booleano `"deleted" == true`. Es un campo mágico no documentado en la API pública ni en `Metadata`. Un usuario que legítimamente guarde `metadata.insert("deleted", true)` (p.ej. un flag de negocio "marca el registro como eliminado en tu dominio") ve sus documentos desaparecer de `list_documents`. Además es inconsistente: `HybridSearch::search` aplica el mismo filtro para todos los modos híbridos (`src/search/hybrid.rs:174-179`), pero `VectorDB::search` (búsqueda vectorial pura, db.rs:518) **no** lo aplica, y `VectorDB::get` tampoco. El mismo documento es visible por `get`/`search` pero invisible por `list_documents`/`filter_search`/`keyword_search`.
- Escenario de fallo: `db.insert_document("x", None, Some(meta con "deleted"=true))` → `db.list_documents(None,None,10,0)` no lo devuelve, pero `db.get("x")` sí lo retorna. Comportamiento sorpresivo y no declarado.
- Verificación: `list_documents` db.rs:1242; `hybrid.rs:174`; contraste con `search` db.rs:518-532 (sin filtro soft-delete) y `get` db.rs:540-551.

## [MEDIUM] Errores de índices parciales silenciados con `let _ =` → índices parciales pueden desincronizarse sin señal

- Archivo: `src/db.rs:384-386` (también `:461`, `:613-616`, `:943-945`)
- Código:
```rust
        // Añadir a índices parciales que coincidan
        let _ = self
            .partial_indexes
            .on_insert(&id, vector, metadata.as_ref());
```
- Problema: `PartialIndexManager::on_insert` devuelve `Result<Vec<String>>` (`src/partial_index.rs:324-329`) que puede fallar (p.ej. un índice parcial HNSW que falla al añadir). Descartar el `Result` con `let _ =` significa que un índice parcial puede quedar sin el documento mientras storage e índice principal sí lo tienen → `search_partial` pierde resultados sin error. Mismo patrón en `on_delete` (`db.rs:567`).
- Escenario de fallo: `db.create_partial_index("tech", ...)`; un `insert` cuyo `on_insert` falla para ese índice parcial → `db.search_partial("tech", &q, 10)` no devuelve el documento recién insertado, sin error reportado al caller de `insert`.
- Verificación: `on_insert`/`on_delete` son `Result` (partial_index.rs:324,345); en db.rs se descartan en `insert`, `insert_document`, `update`, `insert_chunk`. `delete` (db.rs:567) también silencia `on_delete`.

## [MEDIUM] `delete` no es transaccional: `storage.delete` ok + `index.remove`/`bm25.remove` err → referencias colgantes

- Archivo: `src/db.rs:558-570`
- Código:
```rust
    pub fn delete(&self, id: &str) -> Result<bool> {
        let deleted = self.storage.delete(id)?;
        if deleted {
            self.index.remove(id)?;
            // Remover de BM25 si está habilitado
            if let Some(ref bm25) = self.bm25_index {
                bm25.remove(id)?;
            }
            // Remover de índices parciales
            let _ = self.partial_indexes.on_delete(id);
        }
        Ok(deleted)
    }
```
- Problema: si `storage.delete` elimina el documento y luego `index.remove` devuelve `Err`, el documento ya no está en storage pero el índice vectorial sigue referenciándolo → `search` puede devolver un `id` cuyo `storage.get` es `None`. `delete` retorna `Err` pero el storage ya mutó sin rollback.
- Escenario de fallo: raro pero posible si una implementación de `Index::remove` (HNSW/IVF) devuelve error de consistencia interna; el documento queda a medias.
- Verificación: orden de operaciones en db.rs:559-565; sin compensación. `index.remove` es `Result<bool>` (`index/mod.rs:86`); para `FlatIndex` es infalible (`flat.rs:75-77`) pero el trait permite fallo en HNSW/IVF. PLAUSIBLE, no confirmado para los índices concretos (fuera de alcance).

## [MEDIUM] `update` sobre un `id` inexistente lo crea silenciosamente en vez de error

- Archivo: `src/db.rs:591-604`
- Código:
```rust
    self.storage.delete(&id)?;
    ...
    self.index.remove(&id)?;
    self.index
        .add(&id, vector, &*self.storage, self.config.distance)?;
```
- Problema: `update` documenta "Actualiza un documento existente" (db.rs:572-575) pero no verifica existencia. `self.storage.delete(&id)?` devuelve `Ok(false)` si no existe (`memory.rs:50-52`); el `?` solo desenvuelve el `Result`, el `bool` se descarta, así que no se eleva ningún error. Luego se inserta el nuevo documento → `update` actúa como `insert`. Contrasta con `insert` que sí devuelve `AlreadyExists`, y `update_document` (db.rs:628-637) que pasa por `delete`+`insert_document` y tampoco valida existencia (comportamiento análogo, pero al menos consistente con delete-then-insert).
- Escenario de fallo: `db.update("no-existe", &[...], None)` → `Ok(())` y el documento queda creado. Violación de la postcondition implícita del docstring; puede enmascarar bugs en callers que asumen existencia.
- Verificación: `MemoryStorage::delete` devuelve `Ok(bool)` (`memory.rs:50`); `db.rs:591` descarta el bool vía `?`. `delete` (db.rs:558) retorna `bool` al caller, pero `update` no lo usa para decidir.

## [MEDIUM] `FileHeader` trunca parámetros de índice a `u16` al guardar → IVF/HNSW con parámetros grandes se corrompe al recargar

- Archivo: `src/storage/format.rs:62-69`
- Código:
```rust
        let (index_type, hnsw_m, hnsw_ef) = match index {
            IndexType::Flat => (0, 0, 0),
            IndexType::HNSW { m, ef_construction } => (1, *m as u16, *ef_construction as u16),
            IndexType::IVF {
                num_clusters,
                num_probes,
            } => (2, *num_clusters as u16, *num_probes as u16),
        };
```
- Problema: `m`, `ef_construction`, `num_clusters`, `num_probes` son `usize` (ver `IndexType` en `src/index/mod.rs:25-37`) pero se guardan como `u16` (máx 65535). Valores > 65535 se truncan silenciosamente. Al recargar, `get_index_type` reconstruye con el valor truncado (`format.rs:229-241`) → HNSW/IVF con parámetros distintos a los configurados, sin error.
- Escenario de fallo: `Config::new(384).with_index(IndexType::IVF { num_clusters: 100_000, num_probes: 10 })` → guardado con `num_clusters = 100_000 as u16 = 34_464` → recargado como IVF con 34_464 clusters. Resultados de búsqueda distintos, sin señal.
- Verificación: `as u16` en `format.rs:64,65,67,68`; reconstrucción en `format.rs:231-238` (`self.hnsw_m as usize`). `IndexType` permite `usize` arbitrario (`index/mod.rs:25-37`). Sin validación ni error en `FileHeader::new`.

## [MEDIUM] `Distance::from_u8` y `get_index_type` mapean valores desconocidos a un default silencioso → header corrupto se acepta

- Archivo: `src/storage/format.rs:229-241` y `:269-276`
- Código:
```rust
    pub fn get_index_type(&self) -> IndexType {
        match self.index_type {
            1 => IndexType::HNSW { ... },
            2 => IndexType::IVF { ... },
            _ => IndexType::Flat,
        }
    }
```
```rust
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Distance::Euclidean,
            2 => Distance::DotProduct,
            3 => Distance::Manhattan,
            _ => Distance::Cosine,
        }
    }
```
- Problema: un byte `distance_type` o `index_type` fuera del rango conocido (p.ej. por corrupción o versión futura) se convierte silenciosamente a `Cosine`/`Flat`. Un `.mmdb` corrupto que originalmente era `Euclidean` pero con el byte de distancia alterado a `7` se carga como `Cosine` sin error → distancias semánticamente incorrectas. El CRC32 debería atraparlo, pero combinado con el bypass de checksum=0 (hallazgo HIGH anterior) es reachable.
- Escenario de fallo: `.mmdb` con `distance_type = 7` (corrupto/hostil, checksum=0) → `VectorDB::open` devuelve `Ok` con `Distance::Cosine` → búsquedas con métrica equivocada.
- Verificación: `_ => IndexType::Flat` (format.rs:239) y `_ => Distance::Cosine` (format.rs:274). `read_from` valida `version` (format.rs:150-155) pero no los enum bytes.

## [LOW] Redundancia/confusión: `.as_ref().or_else(|| None)` es un no-op en `open`

- Archivo: `src/db.rs:176-181`
- Código:
```rust
                let vec_data = stored
                    .vector
                    .as_ref()
                    .or_else(|| None)
                    .cloned()
                    .or_else(|| stored.quantized.as_ref().map(|q| q.to_f32()));
```
- Problema: `.or_else(|| None)` devuelve `None` incondicionalmente, por lo que `.as_ref().or_else(|| None)` es idéntico a `.as_ref()`. Es código muerto/confuso. La versión paralela en `open_with_fulltext` (db.rs:250-254) está escrita correctamente sin ese `.or_else(|| None)`. Funcionalmente equivalente, pero indica un descuido y perjudica legibilidad/auditoría.
- Escenario de fallo: ninguno funcional; produce el mismo resultado que la versión limpia.
- Verificación: `Option::or_else` con closure que retorna `None` deja la opción intacta. Comparar con db.rs:250-254.

## [LOW] `data_offset` y `index_offset` leídos del header pero no usados para posicionar el reader

- Archivo: `src/storage/disk.rs:182-187` y `format.rs:181-186`
- Código:
```rust
    let file = File::open(path)?;
    let file_len = file.metadata()?.len();
    let mut reader = BufReader::with_capacity(256 * 1024, file);

    // Leer header
    let header = FileHeader::read_from(&mut reader)?;
```
- Problema: `load_vectors` lee secuencialmente después del header y nunca hace `seek` a `header.data_offset` ni a `header.index_offset` (este último solo se usa como guarda `if header.index_offset > 0` en disk.rs:226). Hoy `data_offset` siempre es `HEADER_SIZE` (format.rs:78), así que funciona, pero el formato declara offsets que el loader ignora: un archivo válido futuro que use otro `data_offset` se rompería silenciosamente, y el campo da falsa impresión de formato robusto.
- Escenario de fallo: ninguno con el writer actual; deuda de formato / falsa promesa de robustez.
- Verificación: `format.rs:78` fija `data_offset = HEADER_SIZE`; `disk.rs:226` solo chequea `index_offset > 0` sin usar el valor real para seek.

## [LOW] `StoredVector.quantized` y `VectorEntry.quantized` usan estrategias de serialización distintas

- Archivo: `src/types.rs:283-285` vs `src/storage/format.rs:252-254`
- Código:
```rust
    // types.rs
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantized: Option<crate::quantization::QuantizedVector>,
```
```rust
    // format.rs (VectorEntry)
    /// Note: no skip_serializing_if — bincode is positional and skipping breaks deserialization
    #[serde(default)]
    pub quantized: Option<crate::quantization::QuantizedVector>,
```
- Problema: dos structs casi idénticos con decisiones de serialización distintas y un comentario que advierte que `skip_serializing_if` rompe bincode. `StoredVector` sí usa `skip_serializing_if` (pero no se persiste directamente: `save` mapea a `VectorEntry` en `disk.rs:110-115`). Es frágil: cualquier cambio que persista `StoredVector` directo introduciría el bug que el comentario de `VectorEntry` explicitly previene. Deuda de consistencia.
- Escenario de fallo: ninguno hoy (la persistencia usa `VectorEntry`); riesgo de regresión.
- Verificación: `save` construye `VectorEntry` en `disk.rs:110-115`; `StoredVector` no se serializa a disco directamente.

---

# Cobertura

Leídos COMPLETOS (todas las líneas):
- `src/db.rs` (2039 líneas, incl. tests)
- `src/storage/disk.rs` (546 líneas, incl. tests)
- `src/storage/format.rs` (320 líneas, incl. tests)
- `src/storage/memory.rs` (206 líneas, incl. tests)
- `src/storage/mod.rs` (69 líneas)
- `src/error.rs` (41 líneas)
- `src/types.rs` (353 líneas)

Leídos para contexto / alcanzabilidad (fuera de alcance, no auditados como entregable):
- `src/index/mod.rs` (trait `Index`)
- `src/index/flat.rs` (comportamiento de `add`/`remove`/`search`)
- `src/distance.rs` y `src/distance/simd.rs` (NaN, zero-norm, index OOB)
- `src/quantization.rs` (validación de dimensión en `quantize`)
- `src/partial_index.rs` (firmas de `on_insert`/`on_delete`)
- `src/search/hybrid.rs` (overflow `k*N`, soft-delete, `filter_only_search`)
- `Cargo.toml` (versión de bincode)

Verificaciones ejecutadas (read-only): `grep`/`sed` sobre los archivos anteriores; lectura de firmas. No se ejecutó `cargo` (no se modificó el repo; los hallazgos se confirmaron por lectura estática y trazado de callers). No se modificó ningún archivo.

# Sólido

Partes verificadas y bien implementadas dentro del alcance:

- **Escritura atómica en `save_vectors`** (`disk.rs:62-88`): patrón tmp+rename correcto; limpia el `.tmp` tanto en error de escritura como en fallo de rename. Test `test_atomic_write_preserves_original_on_save` lo verifica.
- **CRC32 con `crc32fast`** sobre todo el payload post-header (vectores + bloques índice + end-marker) en escritura (`disk.rs:105-160`) y verificación en lectura (`disk.rs:190-282`) cuando el footer está intacto y checksum ≠ 0. El test `test_crc32_detects_corruption` confirma detección de corrupción de bytes en la ruta feliz.
- **Header con magic + versión + rango versionado** (`format.rs:132-155`): rechaza magic incorrecto y versiones fuera de `[MIN_VERSION, VERSION]`; compatibilidad hacia atrás leyendo `quantization_type` solo en v3+ y reservando el padding adecuado según versión.
- **`MemoryStorage`** (`memory.rs`): locks `parking_lot::RwLock` correctos (write para mutar, read para consultar); `iter`/`iter_with_vectors` toman snapshot bajo lock y liberan antes de iterar (no retienen el lock durante el yield); `iter_with_vectors` dequantiza on-the-fly solo cuando hace falta (`memory.rs:74-82`).
- **`insert` valida dimensiones y duplicados** en el camino normal (`db.rs:353-362`); lo mismo `insert_document` (`db.rs:433-439`) e `insert_chunk` (`db.rs:911-917`). El bug es específico de `update`.
- **`search` con DB vacía** devuelve `Ok(vec![])` sin alcanzar el índice (`db.rs:526-528`); `search_paged` idem (`db.rs:1341-1348`).
- **`Quantizer::quantize` valida dimensión** (`quantization.rs:511-515`), así que el camino cuantizado de `insert`/`insert_document`/`insert_chunk` no almacena vectores de dimensión errónea.
- **`Distance` coseno** cubre el caso de norma cero (`denom == 0.0 → 1.0`) en todas las rutas (escalar, NEON, AVX-512, AVX2) — verificado por grep de `denom == 0.0` en `simd.rs`. (NaN/Inf no se valida, pero zero-norm sí.)
- **`PagedResult::total_pages`/`current_page`** (`types.rs:338-351`) acota `limit == 0` para evitar división por cero.
- **`load_vectors` reutiliza buffer** `data` entre entradas (`disk.rs:196-209`) para reducir allocs por vector; optimización correcta y segura (solo crece).
- **`format.rs` header a 64 bytes** con padding calculado (`HEADER_SIZE - 43`), round-trip verificado en `test_header_roundtrip`.