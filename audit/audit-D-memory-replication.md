# Auditoría READ-ONLY — Memoria Agéntica, Replicación e Índices Parciales

**Repo:** `D:\Repo\Nueva carpeta (29)\minimemory`
**Alcance:** `src/agent_memory.rs`, `src/memory_traits.rs`, `src/transfer.rs`, `src/partial_index.rs`, `src/replication.rs` (contexto: `src/db.rs`, `src/storage/memory.rs`, `src/index/*`)
**Fecha:** 2026-07-01
**Modo:** READ-ONLY. No se modificó ningún archivo del repo. `cargo check --lib` pasa (1 warning preexistente, ajeno al alcance).

---

## [HIGH] `VectorDB::clear()` no limpia los índices parciales → `search_partial` devuelve ids ya borrados

- Archivo: `src/db.rs:660-666` (clear) y `src/db.rs:800-828` (search_partial)
- Código:
```rust
// db.rs:660
pub fn clear(&self) {
    self.storage.clear();
    self.index.clear();
    if let Some(ref bm25) = self.bm25_index {
        bm25.clear();
    }
}
```
```rust
// db.rs:818
for (id, distance) in results {
    let metadata = self.storage.get(&id)?.and_then(|sv| sv.metadata);
    search_results.push(SearchResult {
        id,
        distance,
        metadata,
    });
}
```
- Problema: `clear()` vacía `storage`, `index` y `bm25`, pero **nunca toca `partial_indexes`** (no existe `PartialIndexManager::clear`). Cada `PartialIndex` mantiene su propio `storage` (parcial) y su `index` subyacente, que conservan todos los vectores e ids insertados antes del `clear()`. `search_partial` corre la búsqueda sobre el índice parcial (que aún tiene los datos) y luego enriquece metadata desde el storage **principal** (ya vacío). Resultado: devuelve `SearchResult { id, distance, metadata: None }` para ids que **ya no existen** en la DB principal.
- Escenario de fallo:
  1. `db.create_partial_index("p", PartialIndexConfig::new(Filter::eq("k","v")))?;`
  2. `db.insert("doc1", &[...], Some(meta))?;` — `on_insert` lo añade al índice parcial.
  3. `db.clear();` — storage principal vacío, índice parcial intacto.
  4. `db.search_partial("p", &q, 10)?` → retorna `doc1` con un distance calculado, aunque `db.contains("doc1") == false`.
- Verificación: `grep` confirmó que `clear()` (db.rs:660) no referencia `partial_indexes` (ningún `partial_indexes.` en `clear`). `PartialIndexManager` no expone ningún método `clear`. `search_partial` (db.rs:814) delega a `partial_indexes.search`, que opera sobre el storage local del `PartialIndex`, no el principal. Alcance: API pública, trivialmente alcanzable.

---

## [HIGH] `create_partial_index` no indexa retroactivamente; `recall_in_project` retorna resultados incompletos sin fallback

- Archivo: `src/partial_index.rs:284-294` (create_index) y `src/agent_memory.rs:950-981` (recall_in_project)
- Código:
```rust
// partial_index.rs:284
pub fn create_index(&self, name: &str, config: PartialIndexConfig) -> Result<()> {
    let mut indexes = self.indexes.write();
    if indexes.contains_key(name) {
        return Err(Error::AlreadyExists(name.to_string()));
    }
    let index = PartialIndex::new(name, config)?;   // índice vacío
    indexes.insert(name.to_string(), Arc::new(index));
    Ok(())
}
```
```rust
// agent_memory.rs:956
if self.db().has_partial_index(&index_name) {
    let embedding = self.embed(query)?;
    let results = self.db().search_partial(&index_name, &embedding, k)?;
    return Ok(results
        .into_iter()
        .map(|r| self.to_recall_from_search(r))
        .collect());
}
```
- Problema: `create_index` crea un `PartialIndex` vacío. Los documentos **existentes** en la DB principal que cumplen el filtro **no se indexan** — solo se indexan inserciones **futuras** (vía `on_insert`). `recall_in_project` detecta que el índice parcial existe y retorna temprano **sin fallback** a la búsqueda filtrada sobre la DB principal. Si el índice está vacío (o incompleto), `recall_in_project` retorna `[]` silenciosamente aunque existan memorias matching en la DB.
- Escenario de fallo:
  1. `memory.set_embed_fn(...); memory.learn_task("auth", ...) ;` — doc `episode-...` insertado con metadata `project: None` (no hay proyecto enfocado aún).
  2. `memory.focus_project("api")?;` — crea `project_api` vacío.
  3. `memory.recall_in_project("auth", 5)?` → retorna `[]` aunque el episodio existe en la DB (y aunque cumpla el filtro si tuviera `project=api`).
  El único modo de que funcione es llamar `rebuild_partial_index` manualmente, lo cual la doc de `focus_project`/`recall_in_project` no indica.
- Verificación: `PartialIndex::new` (partial_index.rs:137) inicializa `document_ids: RwLock::new(Vec::new())` y `storage: MemoryStorage::new()` — ambos vacíos. `create_index` no itera docs existentes. `db.rs::create_partial_index` (db.rs:761) solo delega. `recall_in_project` (agent_memory.rs:956) retorna sin fallback. La doc del módulo (partial_index.rs:1-28) promete "Índices sobre subconjuntos de documentos" sin aclarar que los preexistentes quedan fuera. Alcanzable vía API pública.

---

## [HIGH] `ConflictResolution::LastWriteWins` (y las demás) nunca se aplica — `apply_changes` no compara timestamps y `conflicts` siempre queda vacío

- Archivo: `src/replication.rs:436-490` (apply_changes), `src/replication.rs:425` (estrategia por defecto), `src/replication.rs:430-433` (with_conflict_strategy)
- Código:
```rust
// replication.rs:436
pub fn apply_changes(db: &VectorDB, changes: &[ChangeEntry]) -> Result<SyncResult> {
    let mut applied = 0;
    let mut skipped = 0;
    let conflicts = Vec::new();   // ← siempre vacío
    let mut last_seq = 0u64;

    for change in changes {
        last_seq = last_seq.max(change.sequence);
        match change.operation {
            OperationType::Insert => { /* ... */ }
            OperationType::Update => {
                if let Some(ref vector) = change.vector {
                    if db.contains(&change.document_id) {
                        db.update(&change.document_id, vector, change.metadata.clone())?;
                    } else {
                        db.insert_document(&change.document_id, Some(vector), change.metadata.clone())?;
                    }
                    applied += 1;
                }
            }
            OperationType::Delete => { /* ... */ }
        }
    }
    Ok(SyncResult { applied, skipped, conflicts, new_sequence: last_seq })
}
```
```rust
// replication.rs:425
conflict_strategy: ConflictResolution::LastWriteWins,
```
- Problema: `apply_changes` es un método **estático** (`Self::apply_changes`) y nunca accede a `self.conflict_strategy`. El campo `conflict_strategy` se setea en `new()`/`with_conflict_strategy` pero **nunca se lee** en ningún lado. `conflicts` se inicializa como `Vec::new()` y nunca recibe pushes. No hay ninguna comparación de `change.timestamp` contra el timestamp del doc local. Consecuencias:
  1. Un `Update` remoto **más viejo** (timestamp menor) aplicado después **pisa** datos locales más nuevos — exactamente lo que `LastWriteWins` debería impedir.
  2. `SyncResult::conflicts` siempre es `[]` → el caller no puede detectar conflictos.
  3. `KeepLocal`/`ApplyRemote` son igualmente no-ops: la estrategia elegida no cambia el comportamiento en absoluto.
- Escenario de fallo: instancia A escribe `doc1` a `t=100`. Instancia B escribe `doc1` a `t=200` (más nuevo). Se replica el cambio de A (t=100) a B **después** del cambio local de B: `apply_changes` ve `db.contains("doc1")==true` → `db.update("doc1", vector_viejo, ...)` → el dato nuevo (t=200) es sobrescrito por el viejo (t=100). Sin aviso (conflicts vacío).
- Verificación: `grep` de `conflict_strategy` en `src/replication.rs`: aparece solo en `new` (425), `with_conflict_strategy` (430), `ReplicationConfig` (576, 607). **Cero usos** dentro de `apply_changes`/`sync`. `grep` de `change.timestamp` / `local_timestamp`: el campo `timestamp` de `ChangeEntry` (replication.rs:70) solo se setea en los constructores `insert`/`update`/`delete` y **nunca se lee** para comparación. Alcanzable: API pública (`sync`, `apply_changes`, `apply_snapshot`).

---

## [HIGH] `maybe_compact` descarta entradas del log no exportadas → pérdida silenciosa de cambios para réplicas atrasadas

- Archivo: `src/replication.rs:301-312`
- Código:
```rust
fn maybe_compact(&self) {
    let len = self.entries.read().len();
    if len > self.max_entries {
        // Mantener solo las últimas max_entries/2 entradas
        let keep = self.max_entries / 2;
        let mut entries = self.entries.write();
        if entries.len() > keep {
            let start = entries.len() - keep;
            *entries = entries[start..].to_vec();
        }
    }
}
```
- Problema: cuando el log supera `max_entries` (10000), se descartan las entradas más viejas **sin consultar `last_checkpoint` ni ningún estado de replicación**. `export_since(since)` para un `since` anterior al corte más viejo retenido ya no podrá retornar esas entradas — se perdieron del log. Una réplica que no haya sincronizado desde antes de la compactación **nunca recibirá** esos cambios, sin error: `export_since` simplemente retorna menos entradas.
- Escenario de fallo: primario hace 10001 inserciones (seq 0..10000). `maybe_compact` se dispara en la inserción 10001 y descarta las primeras 5000 (seq 0..4999). Réplica nueva llama `export_since(0)` esperando el log completo → recibe solo seq 5000..10000. Los cambios 0..4999 se perdieron para esa réplica (y para cualquier réplica con `last_synced_sequence < 5000`).
- Verificación: `maybe_compact` se llama al final de cada `track_insert`/`track_update`/`track_delete` (replication.rs:217, 236, 249). No consulta `last_checkpoint` (replication.rs:269) ni los `ReplicationState` (que ni siquiera vive en el `ChangeLog`, sino en `ReplicationManager`). `export_since` (replication.rs:254) solo filtra el `Vec` en memoria. Alcanzable: API pública, sin condición rara.

---

## [MEDIUM] `on_insert` propaga el primer error con `?` y aborta los índices restantes; `db.rs` silencia con `let _ =` → desincronización oculta

- Archivo: `src/partial_index.rs:333-337` (on_insert) y `src/db.rs:384-386, 461, 613-616, 943-945`
- Código:
```rust
// partial_index.rs:324
pub fn on_insert(&self, id: &str, vector: &[f32], metadata: Option<&Metadata>) -> Result<Vec<String>> {
    let indexes = self.indexes.read();
    let mut added_to = Vec::new();
    for (name, index) in indexes.iter() {
        if index.try_add(id, vector, metadata)? {   // ← ? aborta el bucle
            added_to.push(name.clone());
        }
    }
    Ok(added_to)
}
```
```rust
// db.rs:384
let _ = self
    .partial_indexes
    .on_insert(&id, vector, metadata.as_ref());
```
- Problema: `on_insert` usa `?` dentro del bucle. Si `try_add` falla en el índice N, `on_insert` retorna `Err` **sin intentar** los índices N+1..K — esos índices nunca reciben el documento. Además, `try_add` (partial_index.rs:168) inserta en `storage` **antes** de `index.add`; si `index.add` falla, el `PartialIndex` queda internamente inconsistente (storage con el doc, index sin él, `document_ids` sin el id). Todo esto es silenciado por `let _ =` en `db.rs` (líneas 384, 461, 613-616, 943-945). El usuario no recibe señal de que el doc quedó en la DB principal pero no en uno o más índices parciales.
- Escenario de fallo: requiere que `Index::add` devuelva `Err` para algún backend. En el estado actual del repo, `MemoryStorage::insert` siempre devuelve `Ok` (storage/memory.rs:30-44, hace `HashMap::insert`) y `HNSWIndex::add`/`FlatIndex::add`/`IVFIndex::add` retornan `Ok(())` en los caminos normales — por lo que el error es **poco probable hoy**. El hallazgo es **latente**: cualquier futuro backend de indexación que pueda fallar (persistence, disk, cuantización estricta) activará la desincronización silenciada sin cambio en `db.rs`.
- Verificación: `grep` confirmó los 5 sitios de `let _ = ...on_insert/on_delete` en `db.rs`. Lectura de `MemoryStorage::insert`/`delete` y `HNSWIndex::add`/`remove` confirma que actualmente no devuelven `Err` en condiciones normales → **PLAUSIBLE, no confirmado como bug activo hoy**; el patrón de silenciamiento es el problema real.

---

## [MEDIUM] `sync` mezcla secuencias falsas del snapshot con secuencias reales → cambios reales de secuencia baja se filtran

- Archivo: `src/replication.rs:542-561` (create_snapshot) y `src/replication.rs:514-518` (filtro de sync)
- Código:
```rust
// replication.rs:546
for (i, id) in ids.iter().enumerate() {
    if let Some((vector, metadata)) = db.get(id)? {
        if let Some(vec) = vector {
            entries.push(ChangeEntry::insert(
                i as u64,            // ← secuencia falsa 0,1,2,...
                "snapshot",
                id.clone(),
                vec,
                metadata,
            ));
        }
    }
}
```
```rust
// replication.rs:514
let new_changes: Vec<_> = remote_changes
    .iter()
    .filter(|c| c.sequence > state.last_synced_sequence)
    .cloned()
    .collect();
```
- Problema: `create_snapshot` asigna como `sequence` el índice de enumeración (`0,1,2,...`), no una secuencia real del `ChangeLog` remoto, y `origin_id = "snapshot"`. Si un flujo de replicación combina `apply_snapshot` (que internamente llama `apply_changes` y fija `new_sequence = N-1`) con `sync` posterior desde el **mismo `remote_id`**, el `state.last_synced_sequence` puede quedar avanzado a un valor basado en secuencias falsas, y `sync` filtra (`c.sequence > last_synced_sequence`) cambios **reales** del remoto cuyas secuencias caen por debajo de ese umbral. `apply_snapshot`/`apply_changes` no actualizan `ReplicationState` (son estáticos y no ven `states`), pero `sync` sí lo hace; el problema aparece cuando el caller alterna snapshot y sync manejando él mismo el `remote_id`.
- Escenario de fallo: réplica nueva recibe snapshot de 1000 docs (`new_sequence=999`) vía `apply_snapshot`; el caller luego invoca `sync` con `remote_id="snapshot"` (mismo id) y `remote_changes` reales con seq 0..50 → el filtro `> 999` los descarta todos. Si usa un `remote_id` distinto para sync real, no hay colisión (estado per-remote separado).
- Verificación: `apply_snapshot` (replication.rs:564) delega en `apply_changes` y descarta `result.new_sequence` — no toca `states`. `sync` (replication.rs:493) sí actualiza `state.last_synced_sequence = result.new_sequence`. El `origin_id` "snapshot" no se usa para discriminar en `sync` (solo se filtra por `remote_id` del caller). **PLAUSIBLE como bug de composición**; depende de cómo el caller elija `remote_id`.

---

## [MEDIUM] `cleanup_old`: sustracción de timestamps puede underflow u64 (panic en debug) con `max_age_days` grande o reloj pre-época

- Archivo: `src/agent_memory.rs:1155-1156` y `src/agent_memory.rs:1193-1198`
- Código:
```rust
// agent_memory.rs:1155
pub fn cleanup_old(&self, max_age_days: u32) -> Result<usize> {
    let cutoff = current_timestamp() - (max_age_days as u64 * 24 * 60 * 60);
```
```rust
// agent_memory.rs:1193
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
```
- Problema: `current_timestamp()` usa `unwrap_or_default()`, que devuelve `Duration::default()` (= 0) si `now() < UNIX_EPOCH`. Entonces `cutoff = 0 - (max_age_days * 86400)` underflows en u64. En **debug**, la resta con underflow **panica** (`attempt to subtract with overflow`). En release, envuelve a un número enorme y `(*ts as u64) < cutoff` es falso para todo doc → no se borra nada (silencioso). Incluso con reloj normal, `max_age_days = u32::MAX` produce `≈ 3.7e14` s, que excede el unix-time actual (`≈ 1.8e9`) → mismo underflow.
- Escenario de fallo: `memory.cleanup_old(u32::MAX)?` en build debug → panic. O sistema con reloj mal configurado (< 1970) → `current_timestamp()` = 0 → panic en debug para cualquier `max_age_days > 0`.
- Verificación: `current_timestamp` (agent_memory.rs:1193) confirmado con `unwrap_or_default`. El producto `u32::MAX * 86400` cabe en u64, pero la resta con la epoch actual underflowea. `cargo check` compila (el overflow es runtime, no detectado en check). Alcanzable: API pública `cleanup_old`.

---

## [MEDIUM] `set_score_weights(0,0,0)` produce pesos NaN → `recall` hace panic en `partial_cmp(...).unwrap()`

- Archivo: `src/memory_traits.rs:987-992` (set_score_weights), `src/memory_traits.rs:1381` y `1640` (sort)
- Código:
```rust
// memory_traits.rs:987
pub fn set_score_weights(&mut self, relevance: f32, transfer: f32, priority: f32) {
    let total = relevance + transfer + priority;
    self.relevance_weight = relevance / total;   // total=0 → NaN
    self.transfer_weight = transfer / total;
    self.priority_weight = priority / total;
}
```
```rust
// memory_traits.rs:1381
recalls.sort_by(|a, b| b.combined_score.partial_cmp(&a.combined_score).unwrap());
```
- Problema: si los tres pesos son `0.0`, `total = 0.0` y cada división produce `NaN`. Entonces `combined_score = relevance*NaN + transfer*NaN + priority*NaN = NaN` para todo recall. `f32::partial_cmp(NaN, NaN)` devuelve `None`, y `.unwrap()` **panica**. Lo mismo aplica al sort de `recall_high_priority` (línea 1640).
- Escenario de fallo: `let mut m = GenericMemory::<SoftwareDevelopment>::new(4)?; m.set_score_weights(0.0,0.0,0.0);` … luego `m.recall(&emb, 5)?` con ≥2 resultados → panic.
- Verificación: `set_score_weights` es `pub` (memory_traits.rs:987). No hay validación de `total > 0`. El sort con `.unwrap()` (sin `unwrap_or`) está en 1381 y 1640. Alcanzable vía API pública de `GenericMemory`. (Desde `AgentMemory` no se expone directamente, pues `generic_memory()` devuelve `&` no `&mut`, pero sí desde `GenericMemory`.)

---

## [MEDIUM] `UsageStats` usa `.unwrap()` en `duration_since(UNIX_EPOCH)` → panic si el reloj del sistema es pre-época

- Archivo: `src/memory_traits.rs:283-286, 298-301, 331-334, 339-344`
- Código:
```rust
// memory_traits.rs:283
pub fn new() -> Self {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()                       // ← panic si now < epoch
        .as_secs() as i64;
```
```rust
// memory_traits.rs:331
pub fn age_seconds(&self) -> i64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    now - self.created_at
}
```
- Problema: `UsageStats::new`, `record_access`, `age_seconds` y `staleness_seconds` usan `.unwrap()` (no `unwrap_or_default`). Si `SystemTime::now() < UNIX_EPOCH` (reloj del sistema desajustado, algunos entornos de CI/containers durante el boot), `duration_since` devuelve `Err` y `.unwrap()` **panica**. `age_seconds`/`staleness_seconds` se llaman en el hot path de `recall` (vía `calculate_priority_score` → `recency_score`/`calculate_decay`), así que el panic puede ocurrir durante un recall normal.
- Escenario de fallo: host con reloj seteado antes de 1970 (raro pero posible en devices embebidos, VMs recién arrancadas, clock skew extremo) → `memory.recall(...)` panic.
- Verificación: las cuatro funciones usan `.unwrap()` textualmente (memory_traits.rs:285, 300, 333, 342). A diferencia de `agent_memory::current_timestamp` y `replication::current_timestamp` que usan `unwrap_or_default` (no panican, pero devuelven 0 → ver hallazgo de `cleanup_old` y nota abajo). **PLAUSIBLE** (depende de estado del reloj).

---

## [MEDIUM] `to_recall` / `to_recall_from_search` mapean cualquier `type` desconocido o faltante a `MemoryType::Episode`

- Archivo: `src/agent_memory.rs:1025-1041` y `1062-1078`
- Código:
```rust
// agent_memory.rs:1025
fn to_recall(&self, result: crate::HybridSearchResult) -> MemoryRecall {
    let memory_type = result
        .metadata
        .as_ref()
        .and_then(|m| m.get("type"))
        .map(|v| match v {
            crate::MetadataValue::String(s) => match s.as_str() {
                "episode" => MemoryType::Episode,
                "code_snippet" => MemoryType::CodeSnippet,
                "api_knowledge" => MemoryType::ApiKnowledge,
                "error_solution" => MemoryType::ErrorSolution,
                "pattern" => MemoryType::Pattern,
                _ => MemoryType::Episode,          // ← desconocido → Episode
            },
            _ => MemoryType::Episode,              // ← no-String → Episode
        })
        .unwrap_or(MemoryType::Episode);           // ← sin "type" → Episode
```
- Problema: cualquier documento sin metadata `type`, o con un valor no-string, o con un string no reconocido (p.ej. `"documentation"`, `"project_context"`, o un doc insertado directamente vía `db().insert_document` por el usuario) se reporta como `MemoryType::Episode`. Un `recall_similar` que retorne un `code_snippet` cuyo `type` se guardó mal, o un doc ajeno a `AgentMemory` insertado en la misma DB, se etiqueta como episodio. No corrompe datos, pero degrada la semántica del `MemoryRecall` y puede confundir lógica downstream que ramifique por `memory_type`.
- Escenario de fallo: usuario reutiliza `memory.db()` para insertar un doc propio con `type="documentation"`; `recall_similar` lo retorna como `MemoryType::Episode`.
- Verificación: `to_recall` y `to_recall_from_search` tienen los tres fallbacks a `Episode` (líneas 1037-1041 y 1074-1078). `MemoryType::Documentation`/`ProjectContext` no tienen rama en el match → caen al `_`. Alcanzable: API pública.

---

## [LOW] `on_update` en `PartialIndexManager` nunca es invocado — código muerto

- Archivo: `src/partial_index.rs:358-369`
- Código:
```rust
pub fn on_update(&self, id: &str, vector: &[f32], metadata: Option<&Metadata>) -> Result<()> {
    let indexes = self.indexes.read();
    for index in indexes.values() {
        let _ = index.remove(id);
        let _ = index.try_add(id, vector, metadata)?;
    }
    Ok(())
}
```
- Problema: `on_update` no es llamado por ningún sitio. `db.rs::update` (db.rs:612-616) implementa la actualización de índices parciales como `on_delete` + `on_insert`, no como `on_update`. `grep` de `on_update` en todo `src/` devuelve solo su definición. Es código muerto.
- Verificación: `grep -n on_update` → único hit en `partial_index.rs:358`. Alcanzabilidad: ninguna.

---

## [LOW] `OperationType::Update` con `vector: None` se dropea silenciosamente

- Archivo: `src/replication.rs:459-473`
- Código:
```rust
OperationType::Update => {
    if let Some(ref vector) = change.vector {
        if db.contains(&change.document_id) {
            db.update(&change.document_id, vector, change.metadata.clone())?;
        } else {
            db.insert_document(&change.document_id, Some(vector), change.metadata.clone())?;
        }
        applied += 1;
    }
    // ← si vector es None: no hace nada, no suma applied, no error
}
```
- Problema: un `ChangeEntry` con `operation: Update` y `vector: None` (posible si el entry fue deserializado desde JSON manipulado, ya que `vector` es `Option<Vec<f32>>`) se ignora sin error ni contador. `ChangeEntry::update` siempre setea `Some(vector)`, pero la estructura es `Deserialize` y acepta JSON arbitrario. Un update malformado se pierde sin señal.
- Escenario de fallo: `serde_json::from_str` de un JSON con `{"operation":"Update","vector":null,...}` → entry con `vector=None` → `apply_changes` lo saltea.
- Verificación: `ChangeEntry` deriva `Deserialize` (replication.rs:57). El `if let Some` no tiene rama `else`. Alcanzable solo con input deserializado manualmente (no desde los constructores `ChangeEntry::update`).

---

## [LOW] `DecayConfig::calculate_decay` con `half_life_seconds == 0` → división por cero → decay `inf`

- Archivo: `src/memory_traits.rs:437-445`
- Código:
```rust
pub fn calculate_decay(&self, age_seconds: i64, priority: Priority) -> f32 {
    if !self.enabled || self.immune_priorities.contains(&priority) {
        return 1.0;
    }
    let decay = 0.5_f32.powf(age_seconds as f32 / self.half_life_seconds as f32);
    decay.max(self.min_decay)
}
```
- Problema: si el usuario construye un `DecayConfig` con `half_life_seconds: 0` (campo público, no validado), la división `age / 0.0` da `inf` y `0.5.powf(inf) = 0.0` (para age>0) — en realidad `age_seconds/0.0 = +inf`, `0.5_f32.powf(+inf) = 0.0`, `0.0.max(min_decay) = min_decay`. Así que el resultado es `min_decay`, no `inf`. **Corrección**: el caso `age_seconds == 0` da `0/0 = NaN` → `0.5.powf(NaN) = NaN` → `NaN.max(min_decay) = NaN` → `priority_score = raw * NaN = NaN` → sort panic (ver hallazgo de NaN). El escenario de panic real es `half_life_seconds: 0` **y** un doc con `age_seconds == 0` (recién creado).
- Escenario de fallo: `DecayConfig { enabled: true, half_life_seconds: 0, ... }` + recall que toca un doc creado en el mismo segundo → `age_seconds = 0` → `0.0/0.0 = NaN` → decay NaN → combined_score NaN → `partial_cmp().unwrap()` panic.
- Verificación: campos de `DecayConfig` son públicos (memory_traits.rs:384-394), sin validación. `set_decay_config` (memory_traits.rs:976) acepta cualquier config. Los presets usan `slow`/`fast`/`default` con half_life ≠ 0, así que **no es alcanzable por defecto**; solo por configuración manual. **PLAUSIBLE**.

---

## [LOW] `recall` registra `access` para todos los candidatos filtrados (hasta k*3), no solo los k retornados

- Archivo: `src/memory_traits.rs:1361-1378, 1382`
- Código:
```rust
// 1361
{
    let mut stats = self.usage_stats.write();
    for recall in &recalls {            // ← recalls aún no truncado
        let entry = stats
            .entry(recall.id.clone())
            .or_insert_with(|| UsageStats::load_from_metadata(&recall.metadata));
        entry.record_access();
        // ... persiste via db.update
    }
}
// 1381
recalls.sort_by(|a, b| b.combined_score.partial_cmp(&a.combined_score).unwrap());
recalls.truncate(k);                    // ← truncado DESPUÉS de registrar access
```
- Problema: `recall` busca `k*3` candidatos, filtra por transferibilidad, y registra `record_access()` (y persiste via `db.update`) para **todos** los que pasaron el filtro, **antes** de ordenar y truncar a `k`. Resultado: `access_count` se incrementa para memorias que **no fueron retornadas** al caller. Esto infla las estadísticas de uso y sesga futuros `frequency_score`/`usefulness_score` hacia memorias que coinciden transferiblemente pero no son relevantes. Además, realiza hasta `k*3` writes extra en la DB por recall.
- Escenario de fallo: cualquier `recall(emb, k)` con más de `k` candidatos sobre el umbral de transferibilidad → access inflado para los no retornados.
- Verificación: el bloque de grabación de acceso (1361-1378) ocurre antes de `sort_by`+`truncate` (1381-1382). `record_access` (memory_traits.rs:296) incrementa `access_count`. Alcanzable: API pública.

---

## [LOW] `MemoryType::Pattern`, `Documentation`, `ProjectContext` son inalcanzables desde la API de aprendizaje

- Archivo: `src/agent_memory.rs:78-95` (enum), `src/agent_memory.rs:723-865` (learn_*)
- Código:
```rust
// agent_memory.rs:80
pub enum MemoryType {
    Episode, CodeSnippet, ApiKnowledge, Pattern,
    ErrorSolution, Documentation, ProjectContext,
}
```
- Problema: ninguna función `learn_*` (`learn_episode`, `learn_code`, `learn_api`, `learn_error_solution`) setea `type` a `"pattern"`, `"documentation"` o `"project_context"`. Las únicas strings escritas son `episode`, `code_snippet`, `api_knowledge`, `error_solution`. Además, `to_recall`/`to_recall_from_search` no tienen ramas para `documentation`/`project_context` (caen a `Episode`). Estas tres variantes son efectivamente dead desde la API de `AgentMemory`.
- Verificación: `grep` de `MemoryType::Pattern`/`Documentation`/`ProjectContext` en `agent_memory.rs` — solo aparecen en la definición del enum y en `as_str`/`from_str`. Ningún `meta.insert("type", MemoryType::Pattern.as_str())`. Alcanzabilidad: ninguna vía `AgentMemory`.

---

## [LOW] `transfer.rs::ConceptExtractor` implementa el trait `memory_traits::ConceptExtractor` pero ningún `DomainPreset` lo usa

- Archivo: `src/transfer.rs:236-249`
- Código:
```rust
impl crate::memory_traits::ConceptExtractor for ConceptExtractor {
    fn extract(&self, description: &str, content: &str) -> Vec<String> {
        ConceptExtractor::extract(self, description, content)
    }
    fn is_universal(&self, concept: &str) -> bool {
        self.principles.iter().any(|(_, name)| *name == concept)
    }
    fn universal_concepts(&self) -> Vec<&'static str> {
        self.principles.iter().map(|(_, name)| *name).collect()
    }
}
```
- Problema: este trait impl existe para "interoperabilidad con `GenericMemory`", pero `SoftwareDevelopment` (el preset que usa `AgentMemory`) está fijado a `SoftwareConceptExtractor` (memory_traits.rs:2013). No hay ningún `DomainPreset` con `type Concepts = transfer::ConceptExtractor`. La impl es válida pero no conectada a ningún flujo: `TransferableMemory` mantiene su propio `extractor: ConceptExtractor` (campo, no vía trait) y lo usa por método inherente. El trait impl es código muerto/decorativo.
- Verificación: `grep` de `transfer::ConceptExtractor` como type alias en presets → ninguno. `SoftwareDevelopment::Concepts = SoftwareConceptExtractor` (memory_traits.rs:2013). `TransferableMemory` usa `self.extractor.extract(...)` (método inherente, transfer.rs:353), no vía trait. Alcanzabilidad: ninguna.

---

## [LOW] Doble sistema de conceptos inconsistente (transfer.rs vs memory_traits.rs)

- Archivo: `src/transfer.rs:81-233` (ConceptExtractor, 50+ patrones) y `src/memory_traits.rs:1825-1872` (SoftwareConceptExtractor, 10 patrones)
- Código:
```rust
// transfer.rs: extrae ~50 conceptos (design patterns + domain + principles), persiste en tags:
episode.tags.push(format!("concept:{}", concept.to_lowercase().replace(' ', "_")));
```
```rust
// memory_traits.rs: SoftwareConceptExtractor extrae 10 conceptos, persiste en metadata "concepts":
meta.insert("concepts", concepts.join(","));
```
- Problema: conviven dos extractores disjuntos. `TransferableMemory` usa el rico (50+) y persiste conceptos en `tags` (`concept:...`); `recall_transferable` los lee de `tags` (transfer.rs:531-551). `GenericMemory` (el que usa `AgentMemory` internamente) usa `SoftwareConceptExtractor` (10 patrones) y persiste en el campo metadata `concepts`; `recall`/`make_recall` leen de `concepts` (memory_traits.rs:1341-1345, 1505-1509). Cuando `TransferableMemory.learn_task_transferable` delega en `AgentMemory.learn_episode` → `GenericMemory.learn_raw_with_priority`, **ambos** corren: transfer.rs agrega tags, y `SoftwareConceptExtractor` agrega `concepts`. Los conjuntos difieren (p.ej. "Factory Pattern" está en transfer.rs pero no en SoftwareConceptExtractor). Así, `recall_transferable` ve conceptos que `recall` no ve, y viceversa. No corrompe datos, pero la noción de "concepto" es inconsistente entre las dos APIs que operan sobre la misma DB.
- Verificación: comparación de las tablas `patterns` en ambos archivos. `transfer.rs` lista 19 design patterns + 31 domain concepts + 13 principles; `SoftwareConceptExtractor` lista 10 conceptos. Las claves de persistencia difieren (`tags` vs `concepts`). Alcanzable: flujos públicos `learn_task_transferable` y `recall`.

---

## Cobertura

Archivos leídos **completos**:
- `src/partial_index.rs` (511 líneas)
- `src/replication.rs` (743 líneas)
- `src/agent_memory.rs` (1339 líneas)
- `src/memory_traits.rs` (2879 líneas)
- `src/transfer.rs` (795 líneas)

Archivos leídos **parcialmente** para contexto/verificación de alcanzabilidad:
- `src/db.rs` — líneas 360-470, 540-666, 735-865, 938-952 (insert/insert_document/delete/update/clear/create_partial_index/search_partial/rebuild_partial_index/chunking on_insert) + grep de `on_insert`/`on_delete`/`partial_indexes`/`clear`.
- `src/storage/memory.rs` — líneas 1-110 (confirmación de que `insert`/`delete` no devuelven `Err`).
- `src/index/hnsw.rs` — líneas 400-540 (confirmación de que `add`/`remove` retornan `Ok` en caminos normales).
- `src/index/flat.rs`, `src/index/ivf.rs` — grep de `fn remove` (confirmación de signatures).

Comandos de verificación ejecutados (read-only):
- `cargo check --lib` → pasa (1 warning preexistente en `storage/disk.rs`, ajeno al alcance).
- `grep`/`rg` selectivos sobre `src/` para `on_insert`, `on_delete`, `on_update`, `partial_indexes.`, `clear`, `conflict_strategy`, `change.timestamp`, `MemoryType::`, `transfer::ConceptExtractor`.

## Sólido

Partes verificadas y bien hechas dentro del alcance:

- **`PartialIndex::try_add`/`remove`/`rebuild`** (partial_index.rs:168-258): la lógica de filtrado, inserción conjunta en storage+index+`document_ids`, y el rebuild (limpieza completa + reindexado) son correctos. `rebuild` sí reindexa retroactivamente — es la salida prevista al gap de `create_partial_index`, aunque no se documenta.
- **`PartialIndexManager`**: locking con `parking_lot::RwLock` correcto; `create_index` valida duplicados (`AlreadyExists`), `drop_index` valida ausencia (`NotFound`), `search` valida existencia. `on_delete` (partial_index.rs:345) usa `let _ = index.remove(id)` correctamente para ignorar "no está en este índice".
- **`ChangeLog` tracking y serialización** (replication.rs:169-337): `track_*` usa `fetch_add` atómico para secuencias, `to_json`/`from_json` redondos, `export_since`/`checkpoint`/`export_since_checkpoint` correctos. `from_json` restaura `sequence` y `last_checkpoint` como `AtomicU64`.
- **`replication::sync`** (replication.rs:493-529): el filtro `c.sequence > last_synced_sequence` y la actualización `state.last_synced_sequence = result.new_sequence` **solo ocurren si `apply_changes` retorna `Ok`** (el `?` en línea 521 propaga el error antes de tocar el estado). Verifiqué que una aplicación parcial seguida de `Err` **no** avanza el checkpoint, y el re-intento es idempotente (Insert→`AlreadyExists` skipped, Delete→`false` skipped, Update→sobrescribe). Esto responde al punto del PM: `sync` **no** avanza `last_synced_sequence` ante aplicación parcial.
- **`replication::apply_changes` Insert/Delete** (replication.rs:446-481): manejo idempotente correcto (`AlreadyExists`→skipped, delete `false`→skipped, otros errores propagados).
- **`GenericMemory` sistema de prioridad** (memory_traits.rs:1388-1411, 519-534): `calculate_priority_score` combina base/frequency/usefulness/recency con pesos, aplica decay y clamp; `recency_score` y `frequency_score`/`usefulness_score` tratan correctamente el caso `access_count==0` (sin división por cero). `PriorityWeights::calculate_score` hace `.clamp(0.0,1.0)`.
- **`UsageStats::save_to_metadata`/`load_from_metadata`** (memory_traits.rs:348-380): persistencia robusta con defaults si faltan campos. `recall` (memory_traits.rs:1326-1329) carga desde caché o desde metadata — sobrevive a reinicios.
- **`TransferableMemory::recall_transferable`** (transfer.rs:432-469): el sort usa `partial_cmp(...).unwrap_or(Ordering::Equal)` (transfer.rs:462) — **no** panica con NaN, a diferencia de `memory_traits`. Buen manejo defensivo. `calculate_concept_overlap` (transfer.rs:633) guarda `/ concepts.len().max(...).max(1)` contra división por cero.
- **`AgentMemory::save`/`load`** (agent_memory.rs:549-593): persistencia de `WorkingContext` como doc metadata-only `__working_context__`, y `stats()` (agent_memory.rs:1099-1141) descuenta `internal` del total — consistente.
- **`LanguageCompatibility`/`KnowledgeDomain`** (memory_traits.rs:770-833, 606-707): tablas de familias y `related_domains` correctas y simétricas en los casos testeados.

---

*Fin del informe. 17 hallazgos: 4 HIGH, 6 MEDIUM, 7 LOW. Las severidades no se inflaron; los hallazgos marcados "PLAUSIBLE" dependen de condiciones cuya alcanzabilidad no pude confirmar totalmente dentro del alcance (clock del sistema, configuración manual, composición snapshot+sync).*