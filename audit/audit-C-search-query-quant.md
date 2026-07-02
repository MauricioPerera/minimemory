# Auditoría READ-ONLY — minimemory (search / query / quantization / reranker / chunking)

Fecha: 2026-07-01
Alcance: `src/search/mod.rs`, `src/search/hybrid.rs`, `src/search/rrf.rs`, `src/query/mod.rs`, `src/query/filter.rs`, `src/query/operators.rs`, `src/quantization.rs`, `src/reranker.rs`, `src/chunking.rs`
Modo: READ-ONLY. Ningún archivo del repo fue modificado. Verificación de panic de slicing confirmada con `rustc` fuera del repo.

> **Nota previa sobre la dimensión "filtros JSON estilo MongoDB ($eq, $ne, $gt, $and, $or)":** NO existe en el repo ningún parser JSON de filtros al estilo MongoDB. Se buscó (`$eq|$ne|$gt|$and|$or|parse_filter|Filter::from`) en todo `src/` y el único hit es `replication.rs`, sin relación con filtros. Los `FilterOp` se construyen exclusivamente vía constructores Rust tipados (`Filter::eq`, `Filter::regex`, etc.). Por tanto los problemas "operador desconocido ¿error o silencio?" y "tipos mixtos desde JSON" **no son alcanzables**: el tipo `MetadataValue` ya impone el tipo en compile-time. Esta dimensión queda descartada; abajo se reportan los problemas que sí aplican a los operadores tipados.

---

## [CRITICAL] `chunk_by_size` hace slicing por byte index y panic en medio de un char multibyte

- Archivo: `src/chunking.rs:437-454`
- Código:
```rust
while start < content.len() {
    let end = (start + target_size).min(content.len());

    // Buscar un buen punto de corte (fin de párrafo o oración)
    let actual_end = if end < content.len() {
        let slice = &content[start..end];          // <-- panic si `end` cae dentro de un char multibyte
        if let Some(pos) = slice.rfind("\n\n") {
            start + pos + 2
        } else if let Some(pos) = slice.rfind(". ") {
            start + pos + 2
        } else {
            end
        }
    } else {
        end
    };

    let chunk_content = content[start..actual_end].trim().to_string();
```
- Problema: `target_size`, `overlap`, `start` y `end` se tratan como índices de bytes sobre un `&str`, pero nunca se verifican contra fronteras de char (UTF-8). Si `start + target_size` cae dentro de un char multibyte (emoji, acentos, CJK), `&content[start..end]` panic con `byte index ... is not a char boundary`. Lo mismo ocurre en la iteración siguiente si `overlap` deja `start` en medio de un char (`new_start = actual_end - overlap`).
- Escenario de fallo: `content = "a😀b"` (bytes: `0='a'`, `1..5='😀'`, `5='b'`), `ChunkStrategy::BySize { target_size: 2, overlap: 0 }`. Primera iteración: `start=0`, `end=min(0+2,6)=2` → `&content[0..2]` cae dentro de `😀` → panic inmediato. Cualquier texto con caracteres multibyte y un `target_size` que no sea múltiplo alineado produce el panic.
- Verificación: (1) confirmé con un binario aparte compilado con `rustc -O` que `&"a😀b"[0..2]` panic exactamente con `end byte index 2 is not a char boundary; it is inside '😀' (bytes 1..5)`. (2) Alcance API pública: `chunk_markdown` (`src/chunking.rs:775`) → `BasicMarkdownParser::chunk` → `chunk_by_size`; y `VectorDB::ingest_markdown` (`src/db.rs:1017`) llama a `crate::chunking::chunk_markdown(content, config)` pasando contenido y config del usuario directamente. Alcanzable trivialmente desde la API pública sin inputs patológicos (texto Unicode normal + target_size cualquiera).

---

## [HIGH] Paginación (`with_offset`) devuelve menos resultados que `k` en modos Vector/Keyword/Hybrid

- Archivo: `src/search/hybrid.rs:195-207` (offset/truncate central) y `src/search/hybrid.rs:212-229` (vector_search no compensa offset)
- Código:
```rust
// Apply OFFSET (pagination)
if params.offset > 0 {
    if params.offset >= results.len() {
        return Ok(vec![]);
    }
    results = results.into_iter().skip(params.offset).collect();
}

// Apply LIMIT (k) — sub-methods already apply k, but after re-sorting
// the order may differ, so re-apply
if let Some(ref _order) = params.order_by {
    results.truncate(params.k);
}
```
```rust
fn vector_search(...) -> Result<Vec<HybridSearchResult>> {
    ...
    let search_k = if params.filter.is_some() {
        params.k * 10
    } else {
        params.k
    };
    let results = index.search(query, search_k, storage, distance)?;
    ...
        .take(params.k)   // devuelve a lo sumo k, SIN sumar offset
```
- Problema: los sub-métodos `vector_search`, `keyword_search` y `hybrid_search` piden internamente `k` (o `k*10`/`k*3` con filtro) resultados y `.take(params.k)`. Nunca piden `k + offset`. El método central luego aplica `skip(offset)`. Resultado: se devuelven `k - offset` documentos en vez de `k`. Sólo `filter_only_search` (`need_all = order_by.is_some() || offset > 0`, línea 425) recolecta todo cuando hay offset; los demás modos no.
- Escenario de fallo: `HybridSearchParams::vector(q, 10).with_offset(5)` en modo `Vector` sin `order_by` → `vector_search` devuelve 10 → `skip(5)` → **se devuelven 5**, no 10. Página 2 (`offset=10`) → `offset >= len` → `vec![]` aunque existan muchos más documentos. Con `order_by` tampoco se corrige en Vector/Keyword/Hybrid (los sub-métodos siguen sin pedir `k+offset`).
- Verificación: lectura completa de `HybridSearch::search` (líneas 148-210), `vector_search` (212-255), `keyword_search` (257-305), `hybrid_search` (307-409), `filter_only_search` (411-448). Ninguno suma `params.offset` al `search_k`/`fetch_k`. `filter_only_search` es el único que tiene en cuenta `offset`. Contraste: `VectorDB::search_paged` (`src/db.rs:1328`, fuera de scope) sí calcula `fetch_k = offset + limit`, lo que confirma que la biblioteca sabe que hace falta pero `HybridSearch` no lo aplica. Alcanzable vía `VectorDB::hybrid_search` (`src/db.rs:1085`).

---

## [HIGH] Filtro + soft-delete se aplican DESPUÉS de truncar a `k`, devolviendo menos de `k` resultados

- Archivo: `src/search/hybrid.rs:173-179` (soft-delete) y `src/search/hybrid.rs:232-241` / `279-301` (filtro post-fetch)
- Código:
```rust
// Filter out soft-deleted documents (metadata "deleted" = true)
results.retain(|r| {
    !matches!(
        r.metadata.as_ref().and_then(|m| m.get("deleted")),
        Some(crate::types::MetadataValue::Bool(true))
    )
});
```
- Problema: el filtrado de soft-deletes ocurre en `search()` **después** de que los sub-métodos ya truncaron a `k` (`.take(params.k)` en vector_search; `if hybrid_results.len() >= params.k { break }` en keyword_search/hybrid_search). Si entre los primeros `k` hay documentos marcados `deleted=true`, el `retain` los elimina y el resultado final tiene menos de `k` elementos aunque existan documentos válimos con peor ranking que no fueron traídos.
- Escenario de fallo: 10 documentos, 4 de ellos con `metadata.deleted=true` entre los top-k por similitud. `vector(q, 10)` trae 10, `retain` elimina 4 → se devuelven 6, no 10, pese a existir 6+ documentos válidos.
- Verificación: orden de operaciones en `search()` (líneas 155-210): match modo → `results` (ya truncados a k) → `retain` deleted → offset → truncate. El `retain` está después del truncamiento de los sub-métodos. Alcanzable desde `VectorDB::hybrid_search`/`search_with_filter` con documentos soft-deleted.

---

## [HIGH] Filtro se aplica DESPUÉS de RRF en modo Hybrid → resultados < `k` con filtros selectivos

- Archivo: `src/search/hybrid.rs:362-406`
- Código:
```rust
// Aplicar RRF con pesos
let rrf_results = weighted_reciprocal_rank_fusion(
    vec![
        (vector_results, vector_weight),
        (keyword_results, keyword_weight),
    ],
    DEFAULT_RRF_K,
);

// Construir resultados finales
let mut final_results = Vec::new();
for (id, rrf_score) in rrf_results {
    if let Ok(Some(doc)) = storage.get(&id) {
        // Aplicar filtro
        if let Some(filter) = &params.filter {
            if !FilterEvaluator::evaluate(filter, doc.metadata.as_ref()) {
                continue;
            }
        }
        ...
        final_results.push(...);
        if final_results.len() >= params.k {
            break;
        }
    }
}
```
- Problema: en modo `Hybrid`, `fetch_k = params.k * 3` se pide **sin filtrar**; el filtro se aplica **después** de fusionar con RRF y al iterar. Si el filtro es selectivo, gran parte de los `k*3` candidatos se descartan y `final_results` puede quedar con muchos menos de `k` (o vacío) aunque existan k+ documentos que cumplen el filtro con ranking algo peor. Comparar con `vector_search`/`keyword_search`, que al menos piden `k*10` cuando hay filtro; `hybrid_search` sólo pide `k*3` y encima filtra post-fusión.
- Escenario de fallo: 1000 documentos, filtro `category="rare"` que cumple el 2 %. `hybrid(v, "text", 10).with_filter(...)` trae 30 sin filtrar, RRF los ordena, filtra → típicamente 0–1 coinciden → se devuelven <10 aunque >10 documentos cumplen.
- Verificación: lectura de `hybrid_search` (307-409). `fetch_k` = `params.k * 3` (línea 317), sin `*10` cuando hay filtro. El filtro se evalúa dentro del loop sobre `rrf_results` (376-380). Alcanzable vía `VectorDB::hybrid_search` con `HybridSearchParams::hybrid(...).with_filter(...)`.

---

## [MEDIUM] `Filter::regex` con patrón inválido se traga silenciosamente (devuelve "no match" en vez de error)

- Archivo: `src/query/operators.rs:103-108`
- Código:
```rust
FilterOp::Regex(pattern) => match value {
    Some(MetadataValue::String(s)) => regex_lite::Regex::new(pattern)
        .map(|re| re.is_match(s))
        .unwrap_or(false),
    _ => false,
},
```
- Problema: si `pattern` no compila, `Regex::new` devuelve `Err` y `unwrap_or(false)` lo convierte en "no coincide". El error NO se propaga: el filtro evalúa a `false` para todos los documentos, produciendo un resultado vacío sin señal de que la regex era inválida. El usuario no puede distinguir "patrón inválido" de "nadie coincide".
- Escenario de fallo: `Filter::regex("title", "[unclosed")` → todas las evaluaciones `false` → búsqueda/filtro devuelve 0 resultados silenciosamente.
- Verificación: `evaluate` devuelve `bool`, no `Result` (línea 48), por diseño no puede propagar el error. `regex_lite::Regex::new` retorna `Result` estándar. Alcanzable vía `Filter::regex` + `VectorDB::filter_search`/`search_with_filter`/`hybrid_search`.
- Sobre ReDoS: `regex_lite` es un motor NFA sin backreferences ni backtracking catastrófico (misma familia que `regex`), por lo que el riesgo de ReDoS exponencial es bajo; no se reporta como hallazgo separado.

---

## [MEDIUM] Overflow aritmético (`k * 10` / `k * 3`) con `k` grande desde la API pública

- Archivo: `src/search/hybrid.rs:223-227` y `src/search/hybrid.rs:317`
- Código:
```rust
// vector_search
let search_k = if params.filter.is_some() {
    params.k * 10 // Buscar 10x más para compensar filtrado
} else {
    params.k
};
```
```rust
// hybrid_search
let fetch_k = params.k * 3; // Fetch más para RRF
```
- Problema: `params.k` es `usize` público y controlado por el usuario (`HybridSearchParams::vector(query, k)`, `keyword`, `hybrid`, `filter_only`). `k * 10` y `k * 3` no están guardados contra overflow. En build debug → panic por overflow aritmético; en release → wrapping a un número chico → `index.search` trae muy pocos resultados. `k = usize::MAX` (o cualquier `k > usize::MAX/10`) lo dispara.
- Escenario de fallo: `HybridSearchParams::vector(vec, usize::MAX).with_filter(filter)` → `vector_search` → `usize::MAX * 10` → panic en debug / resultados incorrectos en release.
- Verificación: `k` viene sin validar del usuario (`HybridSearchParams` campos públicos, `vector()` línea 54). `VectorDB::search_with_filter(query, filter, k)` (`src/db.rs:1180`) pasa `k` del usuario directo a `HybridSearchParams::vector(query, k)`. Mismo camino en `hybrid_search` (`src/db.rs:1085`). Requiere input patológico, por eso MEDIUM y no CRITICAL.

---

## [MEDIUM] Funciones de distancia Int3/Binary indexan `data` sin verificar consistencia con `dimensions` (panic OOB)

- Archivo: `src/quantization.rs:698-737` (y patrón análogo en `euclidean_distance_int3` 741-768, `dot_product_distance_int3` 772-797, `manhattan_distance_int3` 827-848) y `src/quantization.rs:233-240` (`to_f32`)
- Código:
```rust
pub fn cosine_distance_int3(a: &[u64], b: &[u64], dimensions: usize) -> f32 {
    debug_assert_eq!(a.len(), b.len());
    ...
    let full_words = dimensions / INT3_VALUES_PER_WORD;
    let remainder = dimensions % INT3_VALUES_PER_WORD;

    for i in 0..full_words {
        let wa = a[i];
        ...
    if remainder > 0 {
        let wa = a[full_words];     // <-- OOB si dimensions > a.len()*21
        let wb = b[full_words];
```
```rust
// to_f32 (Int3)
for i in 0..*dimensions {
    let word_idx = i / INT3_VALUES_PER_WORD;
    ...
    let val = ((data[word_idx] >> shift) & 0x7) as u8;   // <-- OOB si dimensions > data.len()*21
```
- Problema: las funciones de distancia toman `dimensions` como parámetro independiente de la longitud real de `data`. Sólo se `debug_assert` que `a.len()==b.len()`, NO que `dimensions <= data.len()*21`. Si `dimensions` afirma más valores de los que `data` contiene, `a[full_words]` / `data[word_idx]` indexan fuera de rango → panic. `QuantizedVector` deriva `Deserialize`, así que un vector cargado de fuente serializada corrupta/truncada (con `dimensions` mayor que `data.len()*21`) hace panic en `to_f32()` y en las distancias.
- Escenario de fallo: deserializar un `QuantizedVector::Int3 { data: vec![0u64; 1], dimensions: 100, ... }` (1 palabra = 21 valores, pero `dimensions=100`) → `to_f32()` itera `i in 0..100`, `word_idx=4` (>0) → `data[4]` OOB → panic. Mismo para las funciones de distancia públicas llamadas con `dimensions` inconsistente.
- Verificación: en el flujo normal (`Quantizer::quantize` → `quantized_distance`) `dimensions` y `data.len()` son consistentes (`num_words = div_ceil(21)`), por lo que **no se confirma vía flujo normal**. El panic es alcanzable vía (a) llamada directa a las `pub fn cosine_distance_int3`/etc. con argumentos inconsistentes, o (b) deserialización de datos no confiables. Marcado **PLAUSIBLE (no confirmado por flujo normal)**.

---

## [MEDIUM] `cosine_distance_polar_symmetric` con `dimensions == 0` devuelve NaN (división `0/0`)

- Archivo: `src/quantization.rs:414-433`
- Código:
```rust
pub fn cosine_distance_polar_symmetric(a: &[u64], b: &[u64], dimensions: usize) -> f32 {
    let pairs = dimensions / 2;
    let mut dot = 0.0f32;
    for p in 0..pairs { ... }
    // Both vectors are unit by construction (cos^2+sin^2=1 per pair)
    1.0 - (dot / pairs as f32)   // pairs=0 → 0.0/0.0 = NaN
}
```
- Problema: con `dimensions == 0`, `pairs == 0`, el loop no ejecuta, `dot = 0.0`, y se retorna `1.0 - (0.0 / 0.0) = NaN`. Un NaN como distancia rompe ordenamientos (`partial_cmp` → `None` → `Equal`) y se propaga como "score" sin error.
- Escenario de fallo: `Quantizer::polar(0)` (`0 % 2 == 0`, pasa el guard de paridad) → `quantize(&[])` acepta (len 0 == dimensions 0) → devuelve `Polar { data: vec![], dimensions: 0, ... }`. Dos de estos vectores → `cosine_distance_polar_symmetric` → NaN.
- Verificación: `Quantizer::polar` (línea 502) no valida `dimensions > 0`. `quantize` Polar (566-599) sólo valida `dimensions % 2 == 0`. `l2_normalize([])` → norm 0 → devuelve `vec![]`. El loop de pares no ejecuta. `cosine_distance_polar_asymmetric` con dim 0 tiene guard `if denom == 0.0 { 1.0 }` y NO produce NaN; la simétrica sí. Alcanzable desde API pública (`Quantizer::polar(0)` es `pub`). Caso borde, MEDIUM.

---

## [MEDIUM] `chunk_by_heading` no avanza `start_pos` tras `clear()` → metadata `start_position` incorrecta

- Archivo: `src/chunking.rs:376-391`
- Código:
```rust
if level <= max_level && !current_content.trim().is_empty() {
    let chunk = Self::create_chunk(
        &config.id_prefix,
        chunk_index,
        current_content.trim().to_string(),
        current_heading.clone(),
        current_level,
        ChunkType::Text,
        start_pos,
        start_pos + current_content.len(),
    );
    chunks.push(chunk);
    chunk_index += 1;
    current_content.clear();
    start_pos += current_content.len();   // <-- current_content ya está vacío: suma 0
}
```
- Problema: `start_pos += current_content.len()` se ejecuta **después** de `current_content.clear()`, por lo que siempre suma 0. `start_position` nunca avanza entre chunks: todos los chunks (salvo el último, que usa `content.len()`) reportan el mismo `start_position`. La metadata de posición es incorrecta.
- Escenario de fallo: cualquier documento con >1 heading al nivel configurado. Los chunks resultantes tienen `start_position` repetido/erróneo.
- Verificación: lectura de `chunk_by_heading` (360-425). El `clear()` está en la línea 389, inmediatamente antes del `start_pos += current_content.len()` (390). Sólo afecta al campo metadata `start_position` (no al contenido del chunk). MEDIUM (metadatos incorrectos, no corrupción de contenido).

---

## [MEDIUM] `chunk_by_paragraph` calcula `start_pos`/`end_pos` sobre longitudes recortadas, no sobre offsets reales

- Archivo: `src/chunking.rs:496-534`
- Código:
```rust
let paragraphs: Vec<&str> = content
    .split("\n\n")
    .map(|p| p.trim())              // trim cambia la longitud
    .filter(|p| !p.is_empty())
    .collect();
...
for para in paragraphs {
    current_paragraphs.push(para);
    if current_paragraphs.len() >= max_paragraphs {
        let chunk_content = current_paragraphs.join("\n\n");
        let end_pos = start_pos + chunk_content.len();   // longitud post-trim/post-join
        ...
        start_pos = end_pos;
```
- Problema: `start_pos`/`end_pos` se acumulan con `chunk_content.len()`, que es la longitud del texto ya recortado (`trim`) y re-unido con `"\n\n"`, no los offsets originales en `content`. Las separaciones `"\n\n"` originales y el whitespace eliminado por `trim` no se contabilizan, por lo que `start_position`/`end_position` se desincronizan progresivamente del texto fuente. No hay panic (no se usa para slicing), pero la metadata de posición es incorrecta.
- Escenario de fallo: documento con párrafos separados por `"\n\n"` y/o whitespace alrededor. A partir del 2º chunk, las posiciones reportadas ya no coinciden con el contenido real.
- Verificación: lectura de `chunk_by_paragraph` (496-569). `end_pos`/`start_pos` sólo se pasan a `create_chunk` como metadata; no se usan para indexar `content`. MEDIUM (metadata incorrecta).

---

## [LOW] `values_equal` para `Float` compara con `f64::EPSILON` absoluto (semántica frágil fuera de rango cercano a 0)

- Archivo: `src/query/operators.rs:114-125`
- Código:
```rust
(MetadataValue::Float(f1), MetadataValue::Float(f2)) => (f1 - f2).abs() < f64::EPSILON,
...
(MetadataValue::Int(i), MetadataValue::Float(f)) => (*i as f64 - f).abs() < f64::EPSILON,
(MetadataValue::Float(f), MetadataValue::Int(i)) => (f - *i as f64).abs() < f64::EPSILON,
```
- Problema: `f64::EPSILON` (~2.2e-16) es un umbral absoluto. Para valores grandes (p. ej. ~1e6), dos floats a 1 ULP de distancia difieren en ~2.3e-10 > EPSILON, así que `Eq` los considera desiguales aunque sean "el mismo número" en sentido relativo. Para valores pequeños la tolerancia es excesiva. Es una igualdad "bitwise casi exacta" con una banda arbitraria sólo útil cerca de 0.
- Escenario de fallo: `Filter::eq("score", 1000000.5f64)` sobre un documento con `Float(1000000.499999999)` (1 ULP) → `false` inesperado si el usuario esperaba igualdad numérica.
- Verificación: lectura de `values_equal`. No es panic ni corrupción; sólo semántica de igualdad. LOW.

---

## [LOW] `compare_metadata_values`: el doc comenta "Equal if either is None" pero el código ordena `None` al final

- Archivo: `src/query/mod.rs:85-110`
- Código:
```rust
/// Compare two optional MetadataValues for ordering.
/// Returns Ordering::Equal if either is None or types are incompatible.
pub fn compare_metadata_values(
    a: Option<&MetadataValue>,
    b: Option<&MetadataValue>,
) -> Ordering {
    match (a, b) {
        ...
        // None or incompatible → equal (stable sort preserves original order)
        (None, Some(_)) => Ordering::Greater, // None sorts last
        (Some(_), None) => Ordering::Less,
        _ => Ordering::Equal,
    }
}
```
- Problema: el comentario de la función y el comentario inline dicen `Ordering::Equal` para `None`, pero el código devuelve `Greater`/`Less` (`None` va al final en orden ascendente). El comportamiento del código es razonable (nulos al final); la documentación es la que miente.
- Escenario de fallo: cualquier `ORDER BY` sobre un campo donde algunos documentos no lo tienen → orden inesperado vs. lo que dice el doc.
- Verificación: lectura de `compare_metadata_values`. LOW (doc/code mismatch).

---

## [LOW] `Reranker::rerank_search` fija `distance = 1.0 - score` sin acotar `score` (puede dar negativo o >1)

- Archivo: `src/reranker.rs:104-112`
- Código:
```rust
for rr in &ranked {
    if rr.index < text_indices.len() {
        let orig_idx = text_indices[rr.index];
        let mut result = results[orig_idx].clone();
        result.distance = 1.0 - rr.score; // Convert score to distance
        reranked.push(result);
    }
}
```
- Problema: `rr.score` proviene de la función de reranking del usuario, que puede devolver cualquier `f32` (negativo, >1, NaN). `1.0 - score` puede salirse de `[0,1]` o ser NaN. El `SearchResult.distance` deja de ser una distancia válida y los posteriores ordenamientos/truncados que asuman rango se comportan de forma incorrecta. No hay panic (el `sort_by` en `rank` usa `partial_cmp(...).unwrap_or(Equal)`).
- Escenario de fallo: rerank fn que devuelve `score > 1.0` (p. ej. scores de probabilidad sin normalizar, o un modelo que escupe logits) → `distance` negativo.
- Verificación: `RerankResult.score: f32` sin invariante documentada (línea 30). `rank` ordena con `partial_cmp` guardado (línea 59). LOW (calidad/semántica, no panic).

---

## [LOW] Convención de signo de `score` inconsistente entre modos de búsqueda

- Archivo: `src/search/hybrid.rs:245`, `src/search/hybrid.rs:289-290`, `src/search/hybrid.rs:392-394`
- Código:
```rust
// vector_search
score: r.distance,              // menor = mejor
// keyword_search
score: -result.score,           // negativo para que menor = mejor
// hybrid_search
score: -rrf_score,              // negativo para que menor = mejor
```
- Problema: el campo `HybridSearchResult.score` significa cosas distintas según el modo (distancia cruda en Vector, BM25 negado en Keyword, RRF negado en Hybrid). Para un consumidor que lea `score` sin mirar `vector_distance`/`bm25_score`, el valor es ambiguo y no comparable entre modos.
- Verificación: lectura de los tres sub-métodos. No es bug funcional dentro de cada modo (cada uno mantiene "menor = mejor"), sólo inconsistencia de API. LOW.

---

# Cobertura

Leídos COMPLETOS (start→EOF):
- `src/search/mod.rs` (34 líneas)
- `src/search/hybrid.rs` (621 líneas, incluidos tests)
- `src/search/rrf.rs` (234 líneas, incluidos tests)
- `src/query/mod.rs` (110 líneas)
- `src/query/filter.rs` (527 líneas, incluidos tests)
- `src/query/operators.rs` (225 líneas, incluidos tests)
- `src/quantization.rs` (1327 líneas, incluidos tests)
- `src/reranker.rs` (200 líneas, incluidos tests)
- `src/chunking.rs` (946 líneas, incluidos tests)

Leídos parcialmente para alcanzabilidad (fuera de scope, sólo verificación de callers):
- `src/db.rs` (búsqueda selectiva de `hybrid_search`, `search_with_filter`, `filter_search`, `search_paged`, `ingest_markdown`, `insert_chunk`).
- `src/replication.rs` (descarte del parser JSON de filtros).

Verificación ejecutada: `rustc -O` sobre un snippet externo para confirmar el panic de slicing en char multibyte (no se modificó ni compiló el repo). No se ejecutaron `cargo check`/`cargo test` sobre el repo para no riesgo de mutación; los hallazgos se basan en lectura estática + semántica del lenguaje.

# Sólido

Partes verificadas y correctas:
- **RRF (`rrf.rs`):** fórmula `1/(k + rank + 1)` con `+1` por indexación 0-correcta; `k=0` no divide por cero (`rank+1 ≥ 1`); listas vacías y listas con un único elemento tratadas; el ordenamiento usa `partial_cmp(...).unwrap_or(Equal)` y no panic con NaN/inf. Tests cubren basic, single-list, weighted, empty.
- **`Filter` lógica booleana (`filter.rs`):** `And`/`Or`/`Not` correctos; `Or` vacío = `true` (vacuously true) y `And` vacío = `true` (`.all` sobre iterador vacío); `Not` negación correcta; `range` construye `Gte`+`Lte` y `And(vec![])` cuando min/max son `None`.
- **`get_nested_value` (dot notation):** navegación `Map`/`List` correcta; índice de array vía `parse::<usize>()` con `.ok()` (no panic); campos inexistentes → `None` consistentemente.
- **`FilterOp` operadores:** `Exists` correcto; `Eq`/`Ne` con campo ausente (`Ne` → true, `Eq` → false, consistente con MongoDB); comparaciones cross-type Int/Float en `compare_values`; `Contains`/`StartsWith`/`EndsWith` case-insensitive y seguras sobre no-string (`_ => false`).
- **Quantization Int8 (`quantize_value`/`dequantize_value`):** mapeo `[-128,127]` correcto para min/max dados; `min==max` → `scale=1.0`, no divide por cero; clamping con `f32::clamp`; NaN → `as i8` satura a 0 (no panic).
- **Quantization Int3 (`quantize_value_3bit`/`dequantize_value_3bit`):** guarda `range <= 0.0` devolviendo el nivel medio (3); packing 21 valores/u63 consistente entre `quantize`, `to_f32` y las distancias; `div_ceil` asegura tamaño de `data` suficiente para `dimensions` en el flujo normal.
- **Polar (`quantize`):** valida `dimensions % 2 == 0`; rotación determinista (`xorshift32` + Fisher-Yates); packing/unpacking 3-bit consistente; `cosine_distance_polar_asymmetric` tiene guard `denom == 0.0 → 1.0`; tablas cos/sin coherentes con los 8 bins.
- **`Quantizer::quantize` general:** valida `vector.len() == self.dimensions` (`Error::DimensionMismatch`); `quantized_distance` valida `dim_a == dim_b` para Int3/Binary/Polar.
- **`Reranker`:** `Send+Sync+'static` bien acotado; `rerank_search` indexa con guard `rr.index < text_indices.len()` (no OOB aunque la fn devuelva índices fuera de rango); resultados sin `text_field` se reubican al final; `is_empty` short-circuit correcto.
- **Chunking `chunk_by_code_blocks`:** detección de fences ``` ``` ``` correcta, lenguaje extraído con `line[3..]` (byte 3 siempre char boundary porque ``` ``` ``` son 3 ASCII), posiciones sólo se usan como metadata (no slicing), `total_chunks` actualizado al final.
- **`chunk_by_size`:** tiene guard anti-loop-infinito (`start = new_start.max(start + 1)` y `if actual_end >= content.len() { break }`); no hay underflow en `actual_end - overlap` (guard `if actual_end > overlap`); texto vacío → sin iteraciones → resultado vacío.
- **`FilterOp` derive `Serialize/Deserialize`** y `Filter` derive `Clone/Debug`: coherentes para persistencia.