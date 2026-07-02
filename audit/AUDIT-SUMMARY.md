# Auditoría profunda de minimemory — Informe consolidado

**Fecha:** 2026-07-01 · **Método:** 5 auditores GLM-5.2 efímeros (uno por área) + verificación independiente de hallazgos CRITICAL/HIGH por el PM (Claude) contra el código.
**Total:** 66 hallazgos — 6 CRITICAL · 15 HIGH · 24 MEDIUM · 21 LOW.
**Informes por área:** [A core+storage](audit-A-core-storage.md) · [B índices+SIMD](audit-B-indexes-simd.md) · [C search/query/quant](audit-C-search-query-quant.md) · [D memoria+replicación](audit-D-memory-replication.md) · [E bindings+embeddings](audit-E-bindings-embeddings.md)

## CRITICAL — verificados por el PM contra el código

1. **`update` no valida dimensiones ni es transaccional** (`src/db.rs:581-618`). No chequea `vector.len() != dimensions` (a diferencia de `insert`, db.rs:353) y borra del storage ANTES de cuantizar/insertar. Un update con dimensión errónea mete un vector de longitud incorrecta en storage e índice. **VERIFICADO.**

2. **Lectura fuera de límites (UB) en SIMD** (`src/distance/simd.rs` + `src/distance/mod.rs:35`). `Distance::calculate` es pública y no valida `a.len() == b.len()`; todas las rutas SIMD (SSE/AVX2/AVX-512/NEON) iteran sobre `a.len()` indexando `b` en los mismos offsets → lectura OOB si `b` es más corto. **VERIFICADO.** Se COMPONE con el hallazgo 1: un `update` con dimensión errónea deja un vector corto en el índice y la siguiente búsqueda puede disparar el OOB desde API segura.

3. **Panic por slicing de bytes en texto multibyte** (`src/chunking.rs:438-442`). `chunk_by_size` corta con `(start + target_size).min(len)` sin respetar char boundaries → `&content[start..end]` panica con texto UTF-8 multibyte (español, CJK, emoji). Alcanzable vía `VectorDB::ingest_markdown` (db.rs:1017, API pública sin feature flag). **VERIFICADO.**

4-6. **Carga de `.mmdb` hostil → OOM/abort** (`src/storage/disk.rs:193,208` + bincode sin límite). `Vec::with_capacity(header.num_vectors)` y `data.resize(len,0)` con valores controlados por el archivo; bincode deserializa `Vec<f32>` con longitud hostil sin límite. **VERIFICADO en código** (aplica solo si se cargan archivos no confiables).

## HIGH — los más relevantes (muestra)

- **IVF nunca se entrena desde `VectorDB`**: `self.index.rebuild()` no tiene ningún caller (db.rs:865 reconstruye índices *parciales*, no el principal) → IVF degrada silenciosamente a fuerza bruta; clustering y nprobe inactivos. **VERIFICADO por el PM.**
- **Los bindings `ffi` y `nodejs` NO COMPILAN** (`cargo check --features ffi|nodejs` falla con 2 errores cada uno: `Option<Vec<f32>>` mal manejado en `get`, brazo `Map` ausente en conversión de metadata). `python` tiene los mismos errores por lectura. **VERIFICADO por el PM ejecutando cargo check.** El CI no cubre estas features.
- **insert/delete/update no transaccionales** entre storage / índice vectorial / BM25 / parciales — sin lock global ni rollback: un fallo a mitad deja estado inconsistente (A).
- **Carga tolera archivos truncados** (`break` silencioso) y **CRC32 evitable** (checksum 0 u archivo sin footer se aceptan sin validar) (A).
- **Paginación y filtros devuelven menos de `k`**: offset se aplica después de truncar; filtro y soft-delete se aplican después de RRF/truncado (C, 3 hallazgos).
- **NaN/Inf corrompen el top-k silenciosamente** en todos los índices (B).
- **`clear()` no limpia índices parciales** → `search_partial` devuelve ids borrados; `create_partial_index` no indexa retroactivamente (D).
- **Replicación: `ConflictResolution` es decorativo** (nunca compara timestamps, `conflicts` siempre vacío) y `maybe_compact` puede descartar cambios aún no replicados → pérdida silenciosa (D).

## MEDIUM/LOW — patrones recurrentes

- Errores silenciados: `let _ =` en índices parciales (db.rs:384,461,567,612-615), `Filter::regex` traga patrones inválidos, `import_snapshot` WASM hace `clear()` antes de validar (pérdida de datos sin rollback), embeddings devuelven vector de ceros ante error.
- Overflows aritméticos `k*10`/`k*3`/`offset+limit` (panic en debug).
- Metadata lossy en WASM (List/Map se serializan pero se descartan al leer) y en Python (bool→int).
- FFI sin `catch_unwind` (panic cruza frontera C = UB) y sin contratos `# Safety`.
- Campo mágico `metadata["deleted"]` inconsistente y sin documentar.

## Qué está sólido (verificado por los auditores, spot-checks del PM)

- HNSW: remove correcto (reuso de slots, sin aristas colgantes, repara entry point), serialización round-trip.
- BM25: IDF no-negativo, stats consistentes tras update/delete, avgdl=0 inalcanzable.
- Persistencia: escritura atómica tmp+rename con cleanup; CRC32 sí se verifica cuando está presente y ≠0.
- k-means++ maneja clusters vacíos/k>n; tail handling SIMD correcto con longitudes iguales.
- `sync()` de replicación NO avanza el checkpoint ante aplicación parcial (propaga el error antes).
- WASM compila y maneja `Option` correctamente; FFI chequea NULL y sus pares into_raw/from_raw son coherentes.

## Prioridad de corrección sugerida

1. Validar dimensiones en `update` (cierra también la vía práctica al UB SIMD) — fix de 3 líneas.
2. `debug_assert!`/check de longitudes en `Distance::calculate`.
3. Char boundaries en `chunk_by_size` (usar `char_indices`/`floor_char_boundary`).
4. Arreglar compilación de features `ffi`/`nodejs`/`python` y añadirlas al CI.
5. Conectar `rebuild()` de IVF a la API pública (o documentar que IVF requiere rebuild manual… que hoy no existe).
6. Límites de sanidad al cargar `.mmdb` (num_vectors/len vs tamaño real del archivo).
