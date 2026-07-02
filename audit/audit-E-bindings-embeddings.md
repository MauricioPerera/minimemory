# Auditoría READ-ONLY — Bindings & Embeddings (minimemory v3.0.0)

Fecha: 2026-07-01
Auditor: agente senior Rust
Alcance: `src/lib.rs`, `src/bindings/{mod,wasm,ffi,nodejs,python}.rs`, `npm-src/index.ts`, `src/embeddings/{mod,bert,gemma}.rs`

## Resumen ejecutivo

El hallazgo más grave es que **tres de los cuatro bindings tras feature-flags no compilan** (`ffi`, `nodejs`; `python` no se pudo compilar en este entorno por pyo3 0.20.3 vs Python 3.14, pero por lectura de código tiene el mismo patrón roto). La causa raíz común es que `VectorDB::get` retorna `Option<(Option<Vec<f32>>, Option<Metadata>)>` (el vector es `Option` porque existen documentos *metadata-only*), y los bindings ffi/nodejs/python tratan el vector como `Vec<f32>` siempre. El binding `wasm` sí maneja correctamente el `Option` y compila.

Esto indica que estos features no se compilan en CI: son código muerto publicado.

---

## [HIGH] `ffi` no compila: `mmdb_get` trata `Option<Vec<f32>>` como `Vec<f32>`
- Archivo: `src/bindings/ffi.rs:268-274`
- Código:
```rust
match db.inner.get(id_str) {
    Ok(Some((vector, _))) => {
        unsafe { *len = vector.len() as u32 };
        let mut boxed = vector.into_boxed_slice();
        let ptr = boxed.as_mut_ptr();
        std::mem::forget(boxed);
        ptr
    }
    _ => {
        unsafe { *len = 0 };
        ptr::null_mut()
    }
}
```
- Problema: `VectorDB::get` retorna `Result<Option<(Option<Vec<f32>>, Option<Metadata>)>>` (`src/db.rs:540`). Aquí `vector` es `Option<Vec<f32>>`, no `Vec<f32>`. `Option` no tiene `.len()` ni `.into_boxed_slice()`. Adicionalmente, aunque compilara, un documento *metadata-only* (vector `None`) no se maneja: se devolvería basura o pánico en lugar de NULL.
- Escenario de fallo: cualquier intento de construir el crate con `--features ffi`.
- Verificación: `cargo check --features ffi --no-default-features` → `error[E0624]: method 'len' is private` (linha 270) y `error[E0599]: no method named 'into_boxed_slice'` (linha 271). **Confirmado: el feature `ffi` no compila.**

---

## [HIGH] `nodejs` no compila: mismo `Option<Vec>` en `get` + brazo `Map` ausente
- Archivo: `src/bindings/nodejs.rs:164-173` y `src/bindings/nodejs.rs:248-261`
- Código (get):
```rust
pub fn get(&self, id: String) -> Result<Option<Vec<f64>>> {
    match self
        .inner
        .get(&id)
        .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))?
    {
        Some((vector, _)) => Ok(Some(vector.iter().map(|&x| x as f64).collect())),
        None => Ok(None),
    }
}
```
- Código (metadata_to_hashmap):
```rust
let str_value = match value {
    RustMetadataValue::String(s) => s,
    RustMetadataValue::Int(i) => i.to_string(),
    RustMetadataValue::Float(f) => f.to_string(),
    RustMetadataValue::Bool(b) => b.to_string(),
    RustMetadataValue::List(_) => "[list]".to_string(),
};
```
- Problema: (1) `vector` es `Option<Vec<f32>>`; `vector.iter()` itera la `Option` (un elemento `&Vec<f32>`), y `|&x| x as f64` intenta castear `Vec<f32>` a `f64` — type error. (2) `MetadataValue` tiene variante `Map(HashMap<String, MetadataValue>)` (`src/types.rs:84`); el `match` no es exhaustivo (falta `Map` y no hay wildcard).
- Escenario de fallo: construir con `--features nodejs`.
- Verificación: `cargo check --features nodejs --no-default-features` → `error[E0605]: non-primitive cast: 'Vec<f32>' as 'f64'` (linha 170) y `error[E0004]: non-exhaustive patterns: 'MetadataValue::Map(_)' not covered` (linha 251). **Confirmado: el feature `nodejs` no compila.**

---

## [HIGH] `python` no compila: mismo `Option<Vec>` en `get` + brazo `Map` ausente en `metadata_to_dict`
- Archivo: `src/bindings/python.rs:180-192` y `src/bindings/python.rs:327-344`
- Código (get):
```rust
fn get(&self, id: &str) -> PyResult<Option<(Vec<f32>, Option<HashMap<String, PyObject>>)>> {
    match self
        .inner
        .get(id)
        .map_err(|e| PyValueError::new_err(e.to_string()))?
    {
        Some((vector, metadata)) => {
            let meta = metadata.map(metadata_to_dict);
            Ok(Some((vector, meta)))
        }
        None => Ok(None),
    }
}
```
- Código (metadata_to_dict):
```rust
let py_value: PyObject = match value {
    RustMetadataValue::String(s) => s.into_py(py),
    RustMetadataValue::Int(i) => i.into_py(py),
    RustMetadataValue::Float(f) => f.into_py(py),
    RustMetadataValue::Bool(b) => b.into_py(py),
    RustMetadataValue::List(_) => py.None(), // Simplificado
};
```
- Problema: (1) El tipo de retorno declara `Vec<f32>` pero `vector` es `Option<Vec<f32>>` → mismatch E0308. (2) `match` no exhaustivo: falta `RustMetadataValue::Map(_)` (`src/types.rs:84`) y no hay wildcard → E0004.
- Escenario de fallo: construir con `--features python`.
- Verificación: **No pude compilar el feature `python` en este entorno**: `pyo3-ffi` 0.20.3 aborta su build-script con *"the configured Python interpreter version (3.14) is newer than PyO3's maximum supported version (3.12)"*. No obstante, por lectura de código los dos errores anteriores son categóricos (Rust exige matches exhaustivos y tipos correctos). **Marcado como confirmado por lectura; no compilable además por incompatibilidad de versión pyo3↔Python 3.14.**

---

## [HIGH] `python`: los booleanos de metadata se almacenan como enteros (1/0)
- Archivo: `src/bindings/python.rs:301-323`
- Código:
```rust
fn dict_to_metadata(dict: &Bound<'_, PyDict>) -> PyResult<RustMetadata> {
    let mut meta = RustMetadata::new();
    for (key, value) in dict.iter() {
        let key_str: String = key.extract()?;
        if let Ok(v) = value.extract::<String>() {
            meta.insert(key_str, v);
        } else if let Ok(v) = value.extract::<i64>() {
            meta.insert(key_str, v);
        } else if let Ok(v) = value.extract::<f64>() {
            meta.insert(key_str, v);
        } else if let Ok(v) = value.extract::<bool>() {
            meta.insert(key_str, v);
        } else {
            return Err(PyValueError::new_err(...));
        }
    }
    Ok(meta)
}
```
- Problema: En Python `bool` es subclase de `int`, y pyo3 `value.extract::<i64>()` sobre `True`/`False` retorna `1`/`0` con éxito. Como el brazo `i64` se prueba **antes** que el brazo `bool`, cualquier booleano se almacena como `MetadataValue::Int(1|0)` y **nunca** como `Bool`. El round-trip `insert({active: True})` → `get()` devuelve `1` (int), no `True`.
- Escenario de fallo: `db.insert("x", vec, {"active": True})` luego `db.get("x")` → metadata `active == 1` (int), no `bool`. También rompe `Filter::eq("active", true)` si el filtro compara tipo.
- Verificación: lectura del orden de los `extract` (String → i64 → f64 → bool). El brazo `bool` es inalcanzable para cualquier `bool` de Python. No pude ejecutarlo (pyo3 no compila en este entorno), pero la semántica de pyo3 `extract::<i64>` sobre `PyBool` es conocida. **PLAUSIBLE confirmado por lectura.**

---

## [MEDIUM] `wasm`: `import_snapshot` no es atómico — limpia la DB antes de validar/importar
- Archivo: `src/bindings/wasm.rs:621-655`
- Código:
```rust
pub fn import_snapshot(&self, json: &str) -> Result<usize, JsError> {
    let entries: Vec<serde_json::Value> = serde_json::from_str(json)
        .map_err(|e| JsError::new(&format!("Invalid JSON: {}", e)))?;

    self.inner.clear();

    let mut imported = 0;
    for entry in &entries {
        let id = entry["id"].as_str()
            .ok_or_else(|| JsError::new("Missing 'id' field in snapshot entry"))?;
        ...
        if let Some(vec) = vector {
            self.inner.insert(id, &vec, Some(meta))
                .map_err(|e| JsError::new(&e.to_string()))?;
        } else {
            self.inner.insert_document(id, None, Some(meta))
                .map_err(|e| JsError::new(&e.to_string()))?;
        }
        imported += 1;
    }
    Ok(imported)
}
```
- Problema: `self.inner.clear()` se ejecuta **antes** de iterar y validar las entradas. Si una entrada es inválida (falta `id`, vector con dimensiones erróneas, metadata JSON inválido), la función retorna `Err` habiendo ya borrado toda la data previa y solo importado parcialmente.
- Escenario de fallo: `db.import_snapshot('[{"id":"a","vector":[0.1]},{"id":"b"}]')` en una DB 384-dim → `clear()`, inserta `a` (falla dim → Err) → DB queda vacía. Sin rollback ni validación previa.
- Verificación: lectura completa de la función (líneas 621-655); `clear()` en línea 625 precede al bucle de inserción. Compila (`--features wasm` OK).

---

## [MEDIUM] `wasm`: round-trip de metadata es lossy — serializa List/Map pero los descarta al parsear
- Archivo: `src/bindings/wasm.rs:704-733` (parse) vs `src/bindings/wasm.rs:849-877` (to_json)
- Código (parse_metadata_json):
```rust
if let serde_json::Value::Object(map) = value {
    for (key, val) in map {
        match val {
            serde_json::Value::String(s) => { meta.insert(&key, s); }
            serde_json::Value::Number(n) => { ... }
            serde_json::Value::Bool(b) => { meta.insert(&key, b); }
            _ => {} // Ignorar arrays y objetos anidados
        }
    }
}
```
- Código (metadata_to_json, líneas 862-870):
```rust
crate::types::MetadataValue::List(l) => {
    serde_json::Value::Array(l.iter().map(|v| metadata_value_to_json(v)).collect())
}
crate::types::MetadataValue::Map(m) => { ... serde_json::Value::Object(obj) }
```
- Problema: `metadata_to_json` serializa `List` y `Map` correctamente, pero `parse_metadata_json` ignora arrays y objetos anidados (`_ => {}`). Un snapshot exportado con metadata anidada (`{"tags": ["a","b"]}`), al reimportarse, pierde esos campos silenciosamente. Asimetría export≠import.
- Escenario de fallo: `insert_with_metadata("x", v, '{"tags":["a","b"],"nested":{"k":1}}')` → `export_snapshot()` lo serializa → `import_snapshot(json)` → los campos `tags` y `nested` se descartan (metadata vacía).
- Verificación: lectura de ambas funciones. `parse_metadata_json` no tiene brazos para `Value::Array` ni `Value::Object` anidados.

---

## [MEDIUM] `wasm`: `get` usa `.unwrap()` que puede pánico → trap de WASM
- Archivo: `src/bindings/wasm.rs:266-277`
- Código:
```rust
pub fn get(&self, id: &str) -> Result<JsValue, JsError> {
    match self.inner.get(id).map_err(|e| JsError::new(&e.to_string()))? {
        Some((vector, metadata)) => {
            let result = serde_json::json!({
                "vector": vector,
                "metadata": metadata.map(|m| metadata_to_json(&m)),
            });
            Ok(JsValue::from_str(&serde_json::to_string(&result).unwrap()))
        }
        None => Ok(JsValue::NULL),
    }
}
```
- Problema: `serde_json::to_string(&result).unwrap()`. Si la serialización fallara, el `unwrap()` pánica dentro de un `#[wasm_bindgen]` → `unreachable`/trap de WASM (aborta el módulo) en lugar de retornar un `JsError`. El `Result` de la firma queda inútil en ese path. El riesgo real es bajo porque la estructura es simple, pero es un patrón incorrecto para un límite de lenguaje.
- Escenario de fallo: difícil de forzar con datos normales (metadata ya es JSON-serializable). Sería relevante si un `MetadataValue::Float` contuviera `NaN`/`Inf` — `serde_json` retorna error al serializar `NaN` f32 → `unwrap` pánica.
- Verificación: lectura de la función; `serde_json::to_string` retorna `Result` y se ignora con `unwrap()`. `MetadataValue::Float(f)` se serializa vía `from_f64(*f)` en `metadata_value_to_json` que ya mapea NaN→Null, pero el `vector` (array de f32) se serializa directo y serde rechaza NaN/Inf en arrays → error → panic. **MEDIUM (condicional a NaN en el vector almacenado).**

---

## [MEDIUM] `ffi`: ningún `catch_unwind` — un pánico cruza la frontera C (UB)
- Archivo: `src/bindings/ffi.rs` (todas las `extern "C"`, p.ej. `mmdb_insert` 145-169, `mmdb_search` 183-226, `mmdb_get` 251-281)
- Código (ejemplo, `mmdb_search`):
```rust
#[no_mangle]
pub extern "C" fn mmdb_search(
    db: *mut MiniMemoryDB,
    query: *const c_float,
    len: u32,
    k: u32,
    result_count: *mut u32,
) -> *mut SearchResult {
    if db.is_null() || query.is_null() || result_count.is_null() {
        return ptr::null_mut();
    }
    let db = unsafe { &*db };
    let query_vec: Vec<f32> = unsafe { std::slice::from_raw_parts(query, len as usize).to_vec() };
    match db.inner.search(&query_vec, k as usize) { ... }
}
```
- Problema: Ninguna función FFI envuelve su cuerpo en `std::panic::catch_unwind`. Un pánico iniciado en código Rust (index out of bounds, overflow aritmético en debug, `unwrap`/`expect` dentro de `VectorDB`) que se propigue a través de `extern "C"` es **comportamiento indefinido** (Rust no define el comportamiento de unwinding a través de FFI; por defecto aborta o corrompe el stack del caller C).
- Escenario de fallo: cualquier pánico no previsto en `db.inner.search`/`insert`/`get` durante una llamada desde C/PHP/Ruby.
- Verificación: `rg "catch_unwind" src/bindings/ffi.rs` → 0 coincidencias. Las funciones `extern "C"` no tienen `#[panic_handler]` ni `catch_unwind`. Condicional a que un pánico ocurra; los métodos internos mayormente retornan `Result`, pero no todos los paths están libres de pánico (p.ej. conversión de `usize` a `u32` en `mmdb_len` podría overflow en release? no, `as u32` trunca sin pánico). **MEDIUM (higiene FFI estándar).**

---

## [MEDIUM] `npm-src/index.ts`: ruta `manhattan` + `hnsw` choca — `new_hnsw` rechaza manhattan
- Archivo: `npm-src/index.ts:87-101` y `src/bindings/wasm.rs:85-91`
- Código (TS):
```ts
if (quant !== "none" || config.hnsw_m || config.hnsw_ef) {
  db = WasmVectorDB.new_with_config(...);
} else if (idx === "hnsw") {
  db = WasmVectorDB.new_hnsw(config.dimensions, dist, 16, 200);
} else {
  db = new WasmVectorDB(config.dimensions, dist, idx);
}
```
- Código (wasm `new_hnsw`):
```rust
pub fn new_hnsw(dimensions: usize, distance: &str, m: usize, ef_construction: usize) -> Result<WasmVectorDB, JsError> {
    let dist = match distance {
        "cosine" | "cos" => RustDistance::Cosine,
        "euclidean" | "l2" => RustDistance::Euclidean,
        "dot" | "dot_product" => RustDistance::DotProduct,
        d => return Err(JsError::new(&format!("Unknown distance: {}", d))),
    };
```
- Problema: El tipo TS `Distance` incluye `"manhattan"` (línea 45). El constructor general `new` y `new_with_config` (vía `parse_distance`) aceptan `manhattan`, pero `new_hnsw` **no**. El wrapper enruta `index:"hnsw"` sin quant/params extra a `new_hnsw`, que lanza `JsError("Unknown distance: manhattan")`. Inconsistencia: mismo `distance` válido falla según el `index`.
- Escenario de fallo: `MiniMemory.create({ dimensions: 384, distance: "manhattan", index: "hnsw" })` → throw en runtime, aunque los tipos lo permiten.
- Verificación: comparación de los tres `match` en `wasm.rs` (`new` línea 56 acepta manhattan; `new_hnsw` línea 86 no; `parse_distance` línea 676 sí). Compila (wasm OK).

---

## [MEDIUM] `embeddings`: `into_embed_fn` devuelve un vector de ceros silencioso ante error
- Archivo: `src/embeddings/mod.rs:241-249`
- Código:
```rust
pub fn into_embed_fn(self) -> impl Fn(&str) -> Vec<f32> + Send + Sync + 'static {
    use std::sync::Arc;
    let embedder = Arc::new(self);
    move |text: &str| -> Vec<f32> {
        embedder
            .embed(text)
            .unwrap_or_else(|_| vec![0.0; embedder.dimensions()])
    }
}
```
- Problema: Si `embed` falla (tokenización, forward pass, OOM), la closure devuelve `vec![0.0; dim]` sin registrar ni señalar error. Un vector de ceros insertado en una DB con distancia coseno es problemático (norma 0 → similitud indefinida / comportamiento degenerado) y silencioso: el `AgentMemory` que use esta función nunca sabrá que el embedding falló.
- Escenario de fallo: texto que rompa el tokenizer, o modelo no cargado correctamente, en un pipeline `set_embed_fn` → se insertan vectores cero que contaminan la búsqueda sin error.
- Verificación: lectura de la función; `unwrap_or_else` devuelve ceros sin log ni panic. Compila (feature `embeddings`).

---

## [LOW] `wasm`: `parse_filter_json` descarta silenciosamente filtros malformados
- Archivo: `src/bindings/wasm.rs:744-805`
- Código:
```rust
if key == "$and" {
    if let serde_json::Value::Array(arr) = val {
        let sub: Result<Vec<Filter>, _> = arr.iter().map(parse_filter_value).collect();
        filters.push(Filter::all(sub?));
    }
} else if key == "$or" {
    if let serde_json::Value::Array(arr) = val {
        ...
    }
} else if let serde_json::Value::Object(ops) = val {
    for (op, target) in ops {
        let f = match op.as_str() {
            "$eq" => ...,
            ...
            "$contains" => { if let Some(s) = target.as_str() { ... } else { continue; } }
            "$regex" => { if let Some(s) = target.as_str() { ... } else { continue; } }
            _ => continue,
        };
        filters.push(f);
    }
} else {
    filters.push(Filter::eq(key.as_str(), json_to_metadata_value(val)));
}
```
- Problema: Si `$and`/`$or` no son arrays, el `if let` no entra y **no se pusha nada ni se retorna error** — el operador se ignora. Operadores desconocidos (`$in`, `$nin`, etc.) → `continue` silencioso. `$contains`/`$regex` con target no-string → `continue`. Un filtro como `{"$and": {"x": 1}}` se convierte en filtro vacío, y luego `if filters.is_empty()` retorna `Err("Empty filter")` — pero un filtro `{"$and": [...], "x": 1}` donde `$and` no es array solo descarta `$and` y queda `x==1`, cambiando la semántica sin avisar.
- Escenario de fallo: `db.filterSearch({"$and": {"x": 1}})` → ignora `$and`, evalúa como `{"x":1}` (o error "Empty filter" si solo estaba `$and`).
- Verificación: lectura de los tres `if let` sin rama `else` de error.

---

## [LOW] `wasm`: inconsistencia de distancias aceptadas entre constructores
- Archivo: `src/bindings/wasm.rs:55-62` (`new`), `85-91` (`new_hnsw`), `676-687` (`parse_distance`)
- Código:
```rust
// new (linha 56): acepta manhattan
"manhattan" | "l1" => RustDistance::Manhattan,
// new_hnsw (linha 86): NO acepta manhattan
// parse_distance (linha 681): acepta manhattan
```
- Problema: `manhattan` es válido en `new`, `new_with_config`, `new_int8/int3/binary` (vía `parse_distance`), pero no en `new_hnsw`. La docstring de `new` dice "cosine, euclidean, dot" pero acepta manhattan. Superficie inconsistente.
- Escenario de fallo: `new_hnsw(384, "manhattan", 16, 200)` → `JsError("Unknown distance: manhattan")`.
- Verificación: comparación de los tres `match`. Compila.

---

## [LOW] `embeddings`: `unsafe` mmap de safetensors sin contrato de safety documentado
- Archivo: `src/embeddings/bert.rs:71-76` y `src/embeddings/gemma.rs:387-392`
- Código:
```rust
let vb = unsafe {
    VarBuilder::from_mmaped_safetensors(&[weights_path.clone()], DType::F32, &device)
        .map_err(|e| {
            Error::InvalidConfig(format!("Failed to load model weights: {}", e))
        })?
};
```
- Problema: Bloque `unsafe` (mmap de archivo) sin comentario `// SAFETY:` que documente el contrato (archivo no modificado/concurrente mientras está mapeado, path controlado). Si el `.safetensors` se modifica durante la inferencia, lectura desde el mmap es UB. Es el patrón estándar de candle, pero el `unsafe` debería justificarse.
- Escenario de fallo: otro proceso trunca/reemplaza `model.safetensors` cacheado mientras corre `embed_batch`.
- Verificación: lectura; ausencia de comentario `SAFETY:` en ambos bloques.

---

## [LOW] `ffi`: contratos `# Safety` no documentados en funciones `extern "C"`
- Archivo: `src/bindings/ffi.rs` (todas las funciones)
- Problema: Ninguna función `#[no_mangle] pub extern "C"` documenta un `# Safety` section. Los callers C/PHP/Ruby deben saber: `db`/`id`/`vector`/`query`/`result_count` deben ser no-NULL (algunos se checkean, otros requieren contrato), `len`/`count` deben coincidir exactamente con la longitud real, `mmdb_free_results` exige el `count` exacto retornado por `mmdb_search`, y los punteros devueltos deben liberarse con su `free_*` específico. Sin doc, el contrato es implícito.
- Escenario de fallo: caller pasa `count` erróneo a `mmdb_free_results` → `Vec::from_raw_parts(results, count, count)` con layout incorrecto → UB (dealloc con capacity equivocada).
- Verificación: `rg "# Safety" src/bindings/ffi.rs` → 0. Las funciones sí checkean NULLs internamente (bueno), pero el contrato de `len`/`count` no se defiende ni documenta.

---

## Cobertura (archivos leídos COMPLETOS)

- `src/lib.rs` ✓
- `src/bindings/mod.rs` ✓
- `src/bindings/wasm.rs` ✓ (878 líneas)
- `src/bindings/ffi.rs` ✓ (419 líneas)
- `src/bindings/nodejs.rs` ✓ (261 líneas)
- `src/bindings/python.rs` ✓ (362 líneas)
- `npm-src/index.ts` ✓ (263 líneas)
- `src/embeddings/mod.rs` ✓ (300 líneas)
- `src/embeddings/bert.rs` ✓ (199 líneas)
- `src/embeddings/gemma.rs` ✓ (612 líneas)

Lecturas de contexto (parciales, para verificar alcanzabilidad):
- `src/db.rs` (firmas `get`, `insert_document`, `list_ids`, `save`, `open`, `search_paged`, etc.)
- `src/types.rs` (enum `MetadataValue`, variante `Map`)

Verificaciones ejecutadas (read-only):
- `cargo check --features ffi --no-default-features` → **FAIL** (2 errores, `mmdb_get`)
- `cargo check --features nodejs --no-default-features` → **FAIL** (2 errores, `get` + `Map` arm)
- `cargo check --features python --no-default-features` → **FAIL** build-script pyo3 (Python 3.14 > 3.12); errores de código no alcanzables a compilar pero visibles por lectura
- `cargo check --features wasm --no-default-features` → **OK** (1 warning ajeno al alcance, `disk.rs:40`)

## Sólido (verificado y bien hecho)

- **`wasm.rs` maneja `Option<Vec<f32>>` correctamente** en `get` (`if let Some(vec) = vector`, línea 604) y `export_snapshot` — a diferencia de los otros tres bindings. Compila.
- **`wasm.rs` validación de entrada**: JSON inválido retorna `JsError` limpio (`parse_metadata_json`, `parse_filter_json`, `import_snapshot` línea 623), no pánico.
- **`wasm.rs` conversión de errores**: todos los métodos mapean `Error` a `JsError` vía `map_err(|e| JsError::new(&e.to_string()))`; consistente.
- **`ffi.rs` checks de NULL**: todas las funciones verifican `db.is_null()` (y los punteros relevantes) antes de dereferenciar — `mmdb_insert:151`, `mmdb_search:190`, `mmdb_get:256`, `mmdb_delete:299`, `mmdb_contains:324`, `mmdb_save:349`, `mmdb_load:373`, `mmdb_len:395`, `mmdb_dimensions:405`, `mmdb_clear:415`. No hay deref ciego.
- **`ffi.rs` propiedad de memoria coherente** (asumiendo compilara): `mmdb_new` usa `Box::into_raw`, `mmdb_free` usa `Box::from_raw` — par correcto, no hay doble free. `mmdb_search`/`mmdb_get` usan `as_mut_ptr` + `mem::forget` y los `free_results`/`free_vector` reconstruyen con `Vec::from_raw_parts(ptr, count, count)` (capacity==len, correcto porque `collect()`/`into_boxed_slice` reservan exacto). Los `CString::into_raw`/`from_raw` por id también están emparejados en `mmdb_free_results:239`.
- **`ffi.rs` `CString::new(r.id).unwrap_or_default()`** (línea 209): en lugar de pánico ante ids con byte nulo, cae a CString vacío. Defensivo.
- **`wasm.rs` `truncate_and_normalize`** (línea 660): guarda `norm > 1e-10` antes de dividir → no divide por cero.
- **`embeddings/mod.rs` `l2_normalize`** (línea 295): guarda `norm > 1e-12` → no divide por cero.
- **`bert.rs`/`gemma.rs` `mean_pooling`**: `count.clamp(1e-9, f64::MAX)` evita división por cero cuando todos los tokens son padding.
- **`bert.rs`/`gemma.rs` batch vacío**: `embed_batch` retorna `Ok(Vec::new())` temprano (líneas 98, 480); `embed` usa `unwrap()` sobre `next()` pero es seguro porque se llama con `&[text]` (no vacío).
- **`lib.rs` superficie pública**: no expone internals peligrosos. `mod storage`, `mod db`, `mod distance`, `mod error`, `mod types` son privados; solo se reexportan los tipos públicos via `pub use`. `pub mod embeddings` está detrás de `#[cfg(feature = "embeddings")]` y `pub mod bindings` tras `#[cfg(any(...))]`.
- **`npm-src/index.ts`**: el wrapper coincide con la API WASM real en tipos y aridad para los métodos que compilan (`insert`, `search`, `insertDocument`, `list`, `searchPaged`, `export`/`import`, `dispose`). La conversión `Float32Array | number[]` es correcta.

---

Nota final: el patrón de los tres bindings rotos (`ffi`, `nodejs`, `python`) sugiere que `VectorDB::get` cambió su firma a `Option<Vec<f32>>` (para soportar documentos *metadata-only* del document-store) después de que los bindings se escribieran, y nadie recompiló esos features. Recomiendo agregar matriz de CI que compile cada feature flag individualmente.