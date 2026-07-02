//! # Open Knowledge Format (OKF) v0.1
//!
//! Ingesta y búsqueda de "bundles" OKF sobre [`VectorDB`].
//!
//! OKF v0.1 (spec de Google Cloud, junio 2026) define un "bundle" como un árbol
//! de directorios con archivos `.md`. Cada `.md` no reservado es un "concepto":
//! frontmatter YAML (delimitado por líneas `---`) con un campo REQUERIDO `type`,
//! seguido de un cuerpo markdown. El Concept ID es la ruta relativa del archivo
//! dentro del bundle sin el sufijo `.md` (ej: `tables/users.md` →
//! `tables/users`).
//!
//! Archivos reservados que NO son conceptos: `index.md` y `log.md` (en
//! cualquier directorio) — se saltan al ingerir.
//!
//! ## Consumo permisivo (obligatorio por spec)
//!
//! Se toleran tipos desconocidos, claves frontmatter extra, links rotos e
//! `index.md` ausente. No se rechaza el bundle por nada de eso. Un archivo se
//! salta (y se reporta) sólo si: no tiene frontmatter, el frontmatter no se
//! puede parsear (falta el delimitador de cierre), o no tiene el campo `type`.
//!
//! ## Parser YAML propio
//!
//! El parser de frontmatter es mínimo y propio — el crate presume "zero deps" y
//! `serde_yaml` está prohibido. Soporta:
//! - escalares `clave: valor` (con o sin comillas `"..."` / `'...'`);
//! - listas en las dos formas YAML comunes: `tags: [a, b]` (inline) y bloque con
//!   `- item`;
//! - comentarios `#` (línea completa).
//!
//! No soporta (limitación documentada, se ignora sin error): mapas anidados,
//! listas de mapas, anchors/aliases, multiline scalars (`|`/`>`), y cualquier
//! estructura más allá de escalares y listas de escalares. Las claves extra
//! escalares se conservan con prefijo `x_`.
//!
//! ## Ejemplo
//!
//! ```rust,ignore
//! use minimemory::okf::{OkfIndex, OkfConfig};
//! use minimemory::chunking::ChunkConfig;
//!
//! let index = OkfIndex::new(OkfConfig::new(ChunkConfig::default())).unwrap();
//! index.ingest_concept("tables/users",
//!     "---\ntype: table\ntitle: Users\n---\n# Users\nColumna id, name.").unwrap();
//!
//! let hits = index.search("users", 5, Some("table")).unwrap();
//! for h in &hits {
//!     println!("{} ({}): {}", h.concept_id, h.chunk_id, h.snippet);
//! }
//! ```

use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};

use crate::chunking::{chunk_markdown, ChunkConfig};
use crate::error::{Error, Result};
use crate::query::Filter;
use crate::search::HybridSearchParams;
use crate::types::{Metadata, MetadataValue};
use crate::VectorDB;

/// Campos de metadata indexados para BM25. El contenido del chunk (`content`)
/// el `title`, `description`, `heading` y `tags_text` (tags space-joined, ya que
/// las `List` no son indexables por BM25).
const FULLTEXT_FIELDS: &[&str] = &["content", "title", "description", "heading", "tags_text"];

/// Campo de metadata con el tipo OKF (filtro principal).
const META_TYPE: &str = "okf_type";
/// Campo de metadata con el Concept ID OKF.
const META_CONCEPT: &str = "okf_concept";

// ============================================================================
// Configuración
// ============================================================================

/// Configuración para construir un [`OkfIndex`].
///
/// `dimensions = 0` (o sin `embed_fn`) → modo solo-BM25: ingesta sin vectores,
/// búsqueda por keywords y filtros. Con `embed_fn` → además búsqueda semántica
/// e híbrida; en ese caso `dimensions` debe coincidir con la salida del embedder.
pub struct OkfConfig {
    /// Dimensiones del vector. `0` = sin vectores (solo BM25/filtros).
    pub dimensions: usize,
    /// Configuración de chunking para el cuerpo markdown de cada concepto.
    pub chunk_config: ChunkConfig,
    /// Función de embedding opcional. Si está presente, los chunks se insertan
    /// con vector y la búsqueda puede ser semántica/híbrida.
    pub embed_fn: Option<Box<dyn Fn(&str) -> Vec<f32> + Send + Sync>>,
}

impl OkfConfig {
    /// Crea una configuración en modo solo-BM25 (sin vectores).
    pub fn new(chunk_config: ChunkConfig) -> Self {
        Self {
            dimensions: 0,
            chunk_config,
            embed_fn: None,
        }
    }

    /// Fija las dimensiones del vector (necesario cuando se usa `embed_fn`).
    pub fn with_dimensions(mut self, dimensions: usize) -> Self {
        self.dimensions = dimensions;
        self
    }

    /// Fija la función de embedding. Implica que la búsqueda será semántica/híbrida.
    pub fn with_embed_fn<F>(mut self, f: F) -> Self
    where
        F: Fn(&str) -> Vec<f32> + Send + Sync + 'static,
    {
        self.embed_fn = Some(Box::new(f));
        self
    }
}

impl Default for OkfConfig {
    fn default() -> Self {
        Self::new(ChunkConfig::default())
    }
}

// ============================================================================
// Resultados
// ============================================================================

/// Estadísticas de una ingesta de bundle.
#[derive(Debug, Clone, Default)]
pub struct IngestStats {
    /// Cantidad de conceptos ingeridos con éxito.
    pub ingested: usize,
    /// Archivos saltados: `(ruta_relativa, razón)`.
    pub skipped: Vec<(String, String)>,
}

impl std::fmt::Display for IngestStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ingested={} skipped={}",
            self.ingested,
            self.skipped.len()
        )
    }
}

/// Un hit de búsqueda OKF.
#[derive(Debug, Clone)]
pub struct OkfHit {
    /// Concept ID del concepto al que pertenece el chunk.
    pub concept_id: String,
    /// ID del documento (`<concept_id>#<n>`).
    pub chunk_id: String,
    /// Score unificado (menor = mejor, consistente con `VectorDB`).
    pub score: f32,
    /// `title` del frontmatter, si estaba presente.
    pub title: Option<String>,
    /// Fragmento del chunk (su contenido, posiblemente truncado).
    pub snippet: String,
}

// ============================================================================
// Índice OKF
// ============================================================================

/// Wrapper sobre [`VectorDB`] que ingiere bundles OKF v0.1 y los busca por
/// keywords (BM25), filtros de metadata y —si se configuró `embed_fn`—
/// búsqueda semántica/híbrida.
pub struct OkfIndex {
    db: VectorDB,
    chunk_config: ChunkConfig,
    embed_fn: Option<Box<dyn Fn(&str) -> Vec<f32> + Send + Sync>>,
}

impl OkfIndex {
    /// Crea un índice OKF. Registra un metadata index sobre `okf_type` (retroactivo).
    pub fn new(config: OkfConfig) -> Result<Self> {
        let db_config = crate::Config::new(config.dimensions);
        let indexed_fields = FULLTEXT_FIELDS.iter().map(|s| s.to_string()).collect();
        let db = VectorDB::with_fulltext(db_config, indexed_fields)?;
        // Índice de metadata retroactivo sobre el tipo: acelera el filtro $eq
        // por `okf_type` en `search`. Si ya existiera (re-construcción sobre la
        // misma DB) se ignora el error de "ya existe"; cualquier otro error se
        // propaga.
        match db.create_metadata_index(META_TYPE) {
            Ok(()) => {}
            Err(Error::AlreadyExists(_)) => {}
            Err(e) => return Err(e),
        }
        Ok(Self {
            db,
            chunk_config: config.chunk_config,
            embed_fn: config.embed_fn,
        })
    }

    /// Referencia a la [`VectorDB`] subyacente.
    pub fn db(&self) -> &VectorDB {
        &self.db
    }

    /// Ingerea un bundle OKF desde un directorio (nativo; no disponible en wasm).
    ///
    /// Recorre el árbol, saltea `index.md`, `log.md` y archivos no-`.md`, parsea
    /// cada concepto, chunkea el cuerpo e inserta docs con id `<concept_id>#<n>`.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn ingest_bundle(&self, root: &Path) -> Result<IngestStats> {
        let mut stats = IngestStats::default();
        if !root.exists() {
            return Err(Error::InvalidConfig(format!(
                "OKF bundle root does not exist: {}",
                root.display()
            )));
        }
        let mut files: Vec<PathBuf> = Vec::new();
        collect_md_files(root, root, &mut files)?;
        for file in files {
            // Ruta relativa normalizada a separador "/".
            let rel = match file.strip_prefix(root) {
                Ok(p) => p,
                Err(_) => continue,
            };
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            // Reservados (en cualquier directorio): index.md y log.md.
            let fname = rel
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if fname == "index.md" || fname == "log.md" {
                continue;
            }
            // Concept ID = ruta relativa sin el sufijo .md (spec OKF v0.1).
            let concept_id = strip_md_suffix(&rel_str);
            let content = std::fs::read_to_string(&file)?;
            match self.ingest_one(concept_id, &content)? {
                IngestOne::Inserted(_) => stats.ingested += 1,
                IngestOne::Skipped(reason) => stats.skipped.push((rel_str, reason)),
            }
        }
        Ok(stats)
    }

    /// Ingerea un único concepto desde string (portable; funciona en wasm).
    ///
    /// Reemplaza los chunks previos del mismo `concept_id` (upsert idempotente):
    /// útil para re-ingestar un concepto actualizado. Devuelve la cantidad de
    /// chunks insertados (`0` si el concepto se salta por falta de `type` o
    /// frontmatter roto).
    pub fn ingest_concept(&self, concept_id: &str, content: &str) -> Result<usize> {
        // Re-ingesta idempotente: borra chunks previos del mismo concepto.
        let _ = self.remove_concept(concept_id)?;
        match self.ingest_one(concept_id, content)? {
            IngestOne::Inserted(n) => Ok(n),
            IngestOne::Skipped(reason) => {
                // La razón no se expone por esta API portable (Result<usize>).
                drop(reason);
                Ok(0)
            }
        }
    }

    /// Núcleo de ingesta compartido entre la vía bundle y la portable.
    fn ingest_one(&self, concept_id: &str, content: &str) -> Result<IngestOne> {
        let (yaml, body) = match split_frontmatter(content) {
            Some(parts) => parts,
            None => {
                let reason = if content.lines().next().map(|l| l.trim()) == Some("---")
                {
                    "broken frontmatter (no closing delimiter)".to_string()
                } else {
                    "no frontmatter".to_string()
                };
                return Ok(IngestOne::Skipped(reason));
            }
        };
        let fm = parse_yaml(&yaml);
        let type_ = match fm.get("type") {
            Some(FValue::Scalar(s)) => s.clone(),
            _ => {
                return Ok(IngestOne::Skipped("missing 'type' field".to_string()));
            }
        };

        let title = scalar(&fm, "title");
        let description = scalar(&fm, "description");
        let resource = scalar(&fm, "resource");
        let timestamp = scalar(&fm, "timestamp");
        let tags = tags_of(&fm, "tags");

        // Claves extra escalares → prefijo `x_`. Listas/mapas extra se descartan.
        let reserved: &[&str] = &["type", "title", "description", "resource", "tags", "timestamp"];
        let mut extras: Vec<(String, String)> = Vec::new();
        for (k, v) in &fm {
            if reserved.contains(&k.as_str()) {
                continue;
            }
            if let FValue::Scalar(s) = v {
                extras.push((format!("x_{k}"), s.clone()));
            }
        }

        let chunk_result = chunk_markdown(&body, &self.chunk_config)?;
        let mut count = 0usize;
        for (n, chunk) in chunk_result.chunks.iter().enumerate() {
            let doc_id = format!("{concept_id}#{n}");
            let vector = self.embed_fn.as_ref().map(|f| f(&chunk.content));

            let mut meta = Metadata::new();
            meta.insert(META_TYPE, type_.as_str());
            meta.insert(META_CONCEPT, concept_id);
            if let Some(ref t) = title {
                meta.insert("title", t.as_str());
            }
            if let Some(ref d) = description {
                meta.insert("description", d.as_str());
            }
            if let Some(ref r) = resource {
                meta.insert("resource", r.as_str());
            }
            if let Some(ref ts) = timestamp {
                meta.insert("timestamp", ts.as_str());
            }
            if !tags.is_empty() {
                let list: Vec<MetadataValue> =
                    tags.iter().map(|t| MetadataValue::String(t.clone())).collect();
                meta.insert("tags", MetadataValue::List(list));
                meta.insert("tags_text", tags.join(" "));
            }
            if let Some(ref h) = chunk.metadata.heading {
                meta.insert("heading", h.as_str());
            }
            for (k, v) in &extras {
                meta.insert(k.as_str(), v.as_str());
            }
            meta.insert("content", chunk.content.as_str());

            self.db.insert_document(&doc_id, vector.as_deref(), Some(meta))?;
            count += 1;
        }
        Ok(IngestOne::Inserted(count))
    }

    /// Busca conceptos por keywords (BM25) o híbrida (si hay `embed_fn`).
    ///
    /// `type_filter` restringe a un `type` OKF concreto (filtro `$eq` sobre
    /// `okf_type`, acelerado por el metadata index creado en [`new`](Self::new)).
    pub fn search(
        &self,
        query: &str,
        k: usize,
        type_filter: Option<&str>,
    ) -> Result<Vec<OkfHit>> {
        let filter = type_filter.map(|t| Filter::eq(META_TYPE, t));
        let results = if let Some(ref f) = self.embed_fn {
            let vec = f(query);
            let mut params = HybridSearchParams::hybrid(vec, query, k);
            if let Some(filter) = filter {
                params = params.with_filter(filter);
            }
            self.db.hybrid_search(params)?
        } else {
            let mut params = HybridSearchParams::keyword(query, k);
            if let Some(filter) = filter {
                params = params.with_filter(filter);
            }
            self.db.hybrid_search(params)?
        };

        let hits = results
            .into_iter()
            .map(|r| {
                let meta = r.metadata.clone();
                let concept_id = meta
                    .as_ref()
                    .and_then(|m| m.get(META_CONCEPT))
                    .and_then(metadata_value::as_string)
                    .unwrap_or_default();
                let title = meta
                    .as_ref()
                    .and_then(|m| m.get("title"))
                    .and_then(metadata_value::as_string);
                let snippet = meta
                    .as_ref()
                    .and_then(|m| m.get("content"))
                    .and_then(metadata_value::as_string)
                    .map(|s| truncate(&s, 200))
                    .unwrap_or_default();
                OkfHit {
                    concept_id,
                    chunk_id: r.id,
                    score: r.score,
                    title,
                    snippet,
                }
            })
            .collect();
        Ok(hits)
    }

    /// Lista los Concept IDs únicos ingeridos.
    pub fn concepts(&self) -> Vec<String> {
        let page = match self
            .db
            .list_documents(None, None, 1_000_000, 0)
        {
            Ok(p) => p,
            Err(_) => return Vec::new(),
        };
        let mut set: Vec<String> = Vec::new();
        for item in page.items {
            if let Some(m) = item.metadata {
                if let Some(MetadataValue::String(c)) = m.get(META_CONCEPT) {
                    if !set.contains(c) {
                        set.push(c.clone());
                    }
                }
            }
        }
        set
    }

    /// Borra todos los chunks de un concepto. Devuelve la cantidad borrada.
    pub fn remove_concept(&self, concept_id: &str) -> Result<usize> {
        let page = self
            .db
            .list_documents(Some(Filter::eq(META_CONCEPT, concept_id)), None, 1_000_000, 0)?;
        let mut count = 0;
        for item in page.items {
            if self.db.delete(&item.id)? {
                count += 1;
            }
        }
        Ok(count)
    }
}

// Helper para distinguir el resultado de ingesta de un único concepto.
enum IngestOne {
    Inserted(usize),
    Skipped(String),
}

// ============================================================================
// Frontmatter: split + parser YAML mínimo
// ============================================================================

/// Valor parseado de una clave de frontmatter.
#[derive(Debug, Clone)]
enum FValue {
    /// Escalar string.
    Scalar(String),
    /// Lista de strings.
    List(Vec<String>),
    /// Estructura anidada (mapa o lista de mapas): ignorada sin error.
    Ignored,
}

/// Parte un documento en `(yaml, body)` si tiene frontmatter válido
/// (delimitado por líneas `---`). Devuelve `None` si no hay frontmatter o si el
/// delimitador de cierre falta (frontmatter roto).
fn split_frontmatter(content: &str) -> Option<(String, String)> {
    let mut lines = content
        .lines()
        .map(|l| l.strip_suffix('\r').unwrap_or(l))
        .peekable();
    let first = lines.next()?;
    if first.trim() != "---" {
        return None;
    }
    let mut yaml: Vec<&str> = Vec::new();
    let mut body: Vec<&str> = Vec::new();
    let mut closed = false;
    for line in lines {
        if !closed {
            let t = line.trim();
            if t == "---" || t == "..." {
                closed = true;
            } else {
                yaml.push(line);
            }
        } else {
            body.push(line);
        }
    }
    if !closed {
        return None;
    }
    Some((yaml.join("\n"), body.join("\n")))
}

/// Parser YAML mínimo (ver doc del módulo). Devuelve un mapa clave → [`FValue`].
fn parse_yaml(text: &str) -> HashMap<String, FValue> {
    let mut result: HashMap<String, FValue> = HashMap::new();

    // Líneas con (indent, contenido trim_start), descartando líneas vacías y
    // comentarios de línea completa.
    let lines: Vec<(usize, String)> = text
        .lines()
        .map(|l| l.strip_suffix('\r').unwrap_or(l))
        .filter_map(|l| {
            let trimmed = l.trim_start();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                return None;
            }
            let indent = l.len() - trimmed.len();
            Some((indent, trimmed.to_string()))
        })
        .collect();

    let mut i = 0;
    while i < lines.len() {
        let (indent, content) = &lines[i];
        if *indent != 0 {
            // Resto de un bloque anidado que decidimos ignorar: saltar.
            i += 1;
            continue;
        }
        let (key, rest) = match split_kv(content) {
            Some(kv) => kv,
            None => {
                i += 1;
                continue;
            }
        };
        let rest_trim = rest.trim();

        // Lista inline: `key: [a, b, c]`.
        if rest_trim.starts_with('[') && rest_trim.ends_with(']') {
            result.insert(key, FValue::List(parse_inline_list(rest_trim)));
            i += 1;
            continue;
        }
        // Escalar no vacío.
        if !rest_trim.is_empty() {
            result.insert(key, FValue::Scalar(strip_quotes(rest_trim)));
            i += 1;
            continue;
        }
        // Valor vacío: puede ser lista en bloque o mapa anidado. Recoger el
        // bloque indentado que sigue.
        let mut j = i + 1;
        let mut block: Vec<usize> = Vec::new();
        while j < lines.len() && lines[j].0 > 0 {
            block.push(j);
            j += 1;
        }
        if block.is_empty() {
            result.insert(key, FValue::Scalar(String::new()));
            i += 1;
            continue;
        }
        let first_content = &lines[block[0]].1;
        if first_content.starts_with("- ") || first_content == "-" {
            // Lista en bloque. Si algún item es un mapa (`- key: value`), se
            // considera estructura anidada y se ignora.
            let is_map_list = block.iter().any(|&bi| {
                let after = lines[bi].1.strip_prefix("- ").unwrap_or(&lines[bi].1);
                split_kv(after.trim_start()).is_some()
            });
            if is_map_list {
                result.insert(key, FValue::Ignored);
            } else {
                let vals: Vec<String> = block
                    .iter()
                    .map(|&bi| {
                        let after = lines[bi].1.strip_prefix("- ").unwrap_or(&lines[bi].1);
                        strip_quotes(after.trim())
                    })
                    .collect();
                result.insert(key, FValue::List(vals));
            }
        } else {
            // Mapa anidado: ignorar.
            result.insert(key, FValue::Ignored);
        }
        i = j;
    }
    result
}

/// Divide `key: value` en la primera clave de mapeo válida (dos puntos seguidos
/// de espacio, o dos puntos finales). Devuelve `(key, value_sin_trim_start)`.
fn split_kv(s: &str) -> Option<(String, String)> {
    let bytes = s.as_bytes();
    let mut idx = None;
    for i in 0..bytes.len() {
        if bytes[i] == b':' {
            let after_space = i + 1 < bytes.len()
                && (bytes[i + 1] == b' ' || bytes[i + 1] == b'\t');
            let is_last = i + 1 == bytes.len();
            if after_space || is_last {
                idx = Some(i);
                break;
            }
        }
    }
    let idx = idx?;
    let key = s[..idx].trim().to_string();
    if key.is_empty() {
        return None;
    }
    let val = s[idx + 1..].trim_start().to_string();
    Some((key, val))
}

/// Parsea una lista inline `[a, b, 'c']`.
fn parse_inline_list(s: &str) -> Vec<String> {
    let inner = s.trim().trim_start_matches('[').trim_end_matches(']');
    if inner.trim().is_empty() {
        return Vec::new();
    }
    split_csv(inner)
}

/// Separa por comas respetando comillas.
fn split_csv(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_dq = false;
    let mut in_sq = false;
    for c in s.chars() {
        match c {
            '"' if !in_sq => in_dq = !in_dq,
            '\'' if !in_dq => in_sq = !in_sq,
            ',' if !in_dq && !in_sq => {
                let t = strip_quotes(cur.trim());
                if !t.is_empty() {
                    out.push(t);
                }
                cur.clear();
            }
            _ => cur.push(c),
        }
    }
    let t = strip_quotes(cur.trim());
    if !t.is_empty() {
        out.push(t);
    }
    out
}

/// Quita comillas envolventes (dobles o simples).
fn strip_quotes(s: &str) -> String {
    let s = s.trim();
    let b = s.as_bytes();
    if b.len() >= 2 {
        let (fst, lst) = (b[0], b[b.len() - 1]);
        if (fst == b'"' && lst == b'"') || (fst == b'\'' && lst == b'\'') {
            return s[1..s.len() - 1].to_string();
        }
    }
    s.to_string()
}

fn scalar(fm: &HashMap<String, FValue>, key: &str) -> Option<String> {
    match fm.get(key)? {
        FValue::Scalar(s) => Some(s.clone()),
        _ => None,
    }
}

fn tags_of(fm: &HashMap<String, FValue>, key: &str) -> Vec<String> {
    match fm.get(key) {
        Some(FValue::List(v)) => v.clone(),
        Some(FValue::Scalar(s)) if !s.is_empty() => vec![s.clone()],
        _ => Vec::new(),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(max).collect();
        t.push('…');
        t
    }
}

/// Quita el sufijo `.md` (insensible a mayúsculas) de una ruta relativa
/// normalizada con separador `/`. Es el último paso para obtener el Concept ID.
#[cfg(not(target_arch = "wasm32"))]
fn strip_md_suffix(rel: &str) -> &str {
    if rel.len() >= 3 && rel[rel.len() - 3..].eq_ignore_ascii_case(".md") {
        &rel[..rel.len() - 3]
    } else {
        rel
    }
}

/// Recorre `root` recursivamente y recolecta archivos `.md` (paths absolutos).
#[cfg(not(target_arch = "wasm32"))]
fn collect_md_files(root: &Path, base: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    let entries = std::fs::read_dir(root)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_md_files(&path, base, out)?;
        } else if path.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("md")) == Some(true) {
            out.push(path);
        }
    }
    let _ = base;
    Ok(())
}

// Pequeño ayudante para leer MetadataValue como String sin traer el módulo de tipos.
mod metadata_value {
    use crate::types::MetadataValue;
    pub fn as_string(v: &MetadataValue) -> Option<String> {
        match v {
            MetadataValue::String(s) => Some(s.clone()),
            _ => None,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn cfg() -> OkfConfig {
        OkfConfig::new(ChunkConfig::default())
    }

    fn write(dir: &Path, rel: &str, content: &str) -> PathBuf {
        let p = dir.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&p, content).unwrap();
        p
    }

    // ---------- Frontmatter ----------

    #[test]
    fn fm_scalars_quoted_and_unquoted() {
        let fm = parse_yaml(
            "type: table\ntitle: \"My Title\"\ndescription: 'a desc'\nresource: https://x.com\n",
        );
        assert_eq!(scalar(&fm, "type").as_deref(), Some("table"));
        assert_eq!(scalar(&fm, "title").as_deref(), Some("My Title"));
        assert_eq!(scalar(&fm, "description").as_deref(), Some("a desc"));
        assert_eq!(
            scalar(&fm, "resource").as_deref(),
            Some("https://x.com")
        );
    }

    #[test]
    fn fm_tags_inline_and_block() {
        let a = parse_yaml("tags: [rust, db, \"vector\"]\n");
        assert_eq!(tags_of(&a, "tags"), vec!["rust", "db", "vector"]);

        let b = parse_yaml("tags:\n  - rust\n  - db\n  - vector\n");
        assert_eq!(tags_of(&b, "tags"), vec!["rust", "db", "vector"]);
    }

    #[test]
    fn fm_comments_and_extras_preserved() {
        let fm = parse_yaml(
            "# header comment\ntype: note\n# mid comment\nauthor: jane\nseverity: high\n",
        );
        assert_eq!(scalar(&fm, "type").as_deref(), Some("note"));
        assert_eq!(scalar(&fm, "author").as_deref(), Some("jane"));
        assert_eq!(scalar(&fm, "severity").as_deref(), Some("high"));
    }

    #[test]
    fn fm_nested_ignored_without_error() {
        let fm = parse_yaml(
            "type: note\nauthor:\n  name: jane\n  team: core\nmetrics:\n  - errors: 2\n  - warns: 1\n",
        );
        assert!(matches!(fm.get("author"), Some(FValue::Ignored)));
        assert!(matches!(fm.get("metrics"), Some(FValue::Ignored)));
        assert_eq!(scalar(&fm, "type").as_deref(), Some("note"));
    }

    #[test]
    fn fm_split_frontmatter_valid_and_broken() {
        let (yaml, body) = split_frontmatter("---\ntype: x\n---\n# Body\n").unwrap();
        assert_eq!(yaml, "type: x");
        assert_eq!(body, "# Body");

        assert!(split_frontmatter("# no frontmatter\nbody").is_none());
        // Apertura sin cierre → None (roto).
        assert!(split_frontmatter("---\ntype: x\nbody sin cierre").is_none());
    }

    // ---------- Permisividad ----------

    #[test]
    fn ingest_no_type_is_skipped() {
        let idx = OkfIndex::new(cfg()).unwrap();
        let r = idx
            .ingest_concept("c1", "---\ntitle: no type\n---\nbody")
            .unwrap();
        assert_eq!(r, 0);
        assert!(idx.concepts().is_empty());
    }

    #[test]
    fn ingest_unknown_type_is_ingested() {
        let idx = OkfIndex::new(cfg()).unwrap();
        let n = idx
            .ingest_concept(
                "c1",
                "---\ntype: WeirdType\ntitle: t\n---\n# H\nhello world\n",
            )
            .unwrap();
        assert!(n >= 1);
        assert!(idx.concepts().contains(&"c1".to_string()));
    }

    #[test]
    fn ingest_broken_frontmatter_skipped() {
        let idx = OkfIndex::new(cfg()).unwrap();
        let n = idx
            .ingest_concept("c1", "---\ntype: x\nbody no cierre\n")
            .unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn bundle_without_index_md_is_ok() {
        let dir = tempdir();
        write(
            &dir,
            "a.md",
            "---\ntype: doc\ntitle: A\n---\nalpha content\n",
        );
        write(
            &dir,
            "b.md",
            "---\ntype: doc\ntitle: B\n---\nbeta content\n",
        );
        let idx = OkfIndex::new(cfg()).unwrap();
        let stats = idx.ingest_bundle(&dir).unwrap();
        assert_eq!(stats.ingested, 2);
        assert!(stats.skipped.is_empty());
    }

    // ---------- Reservados ----------

    #[test]
    fn index_and_log_md_excluded_in_subdirs() {
        let dir = tempdir();
        write(
            &dir,
            "index.md",
            "---\ntype: index\n---\nshould be skipped\n",
        );
        write(
            &dir,
            "log.md",
            "---\ntype: log\n---\nshould be skipped\n",
        );
        write(
            &dir,
            "sub/index.md",
            "---\ntype: index\n---\nsub index skipped\n",
        );
        write(
            &dir,
            "sub/real.md",
            "---\ntype: doc\ntitle: Real\n---\nreal content here\n",
        );
        let idx = OkfIndex::new(cfg()).unwrap();
        let stats = idx.ingest_bundle(&dir).unwrap();
        assert_eq!(stats.ingested, 1);
        assert_eq!(idx.concepts(), vec!["sub/real".to_string()]);
    }

    // ---------- Concept id ----------

    #[test]
    fn concept_id_with_subdirs_and_windows_sep() {
        let dir = tempdir();
        write(
            &dir,
            "tables/users.md",
            "---\ntype: table\ntitle: Users\n---\nusers table\n",
        );
        let idx = OkfIndex::new(cfg()).unwrap();
        idx.ingest_bundle(&dir).unwrap();
        assert_eq!(idx.concepts(), vec!["tables/users".to_string()]);
    }

    // ---------- Búsqueda ----------

    #[test]
    fn search_type_filter_and_mixed() {
        let idx = OkfIndex::new(cfg()).unwrap();
        idx.ingest_concept(
            "a",
            "---\ntype: doc\ntitle: Alpha\n---\nrust programming language\n",
        )
        .unwrap();
        idx.ingest_concept(
            "b",
            "---\ntype: note\ntitle: Beta\n---\nrust memory safety notes\n",
        )
        .unwrap();

        let only_doc = idx.search("rust", 10, Some("doc")).unwrap();
        assert!(!only_doc.is_empty());
        assert!(only_doc.iter().all(|h| h.concept_id == "a"));

        let all = idx.search("rust", 10, None).unwrap();
        let concepts: Vec<_> = all.iter().map(|h| h.concept_id.clone()).collect();
        assert!(concepts.contains(&"a".to_string()));
        assert!(concepts.contains(&"b".to_string()));
    }

    // ---------- remove + re-ingesta ----------

    #[test]
    fn remove_and_reingest_updates_content() {
        let idx = OkfIndex::new(cfg()).unwrap();
        idx.ingest_concept(
            "c",
            "---\ntype: doc\ntitle: C\n---\nold content alpha\n",
        )
        .unwrap();
        let hits_old = idx.search("alpha", 10, None).unwrap();
        assert!(!hits_old.is_empty());

        let removed = idx.remove_concept("c").unwrap();
        assert!(removed >= 1);
        let hits_none = idx.search("alpha", 10, None).unwrap();
        assert!(hits_none.is_empty());

        idx.ingest_concept(
            "c",
            "---\ntype: doc\ntitle: C\n---\nnew content beta\n",
        )
        .unwrap();
        let hits_beta = idx.search("beta", 10, None).unwrap();
        assert!(!hits_beta.is_empty());
        let hits_alpha = idx.search("alpha", 10, None).unwrap();
        assert!(hits_alpha.is_empty());
    }

    // ---------- Portable vs bundle ----------

    #[test]
    fn ingest_concept_matches_bundle() {
        let dir = tempdir();
        let content = "---\ntype: doc\ntitle: T\n---\n# H\nshared body text xyz\n";
        write(&dir, "shared.md", content);

        let idx_bundle = OkfIndex::new(cfg()).unwrap();
        let stats = idx_bundle.ingest_bundle(&dir).unwrap();
        assert_eq!(stats.ingested, 1);

        let idx_portable = OkfIndex::new(cfg()).unwrap();
        let n = idx_portable.ingest_concept("shared", content).unwrap();

        // Mismo número de chunks (mismo contenido → mismo chunking).
        let page_b = idx_bundle
            .db
            .list_documents(Some(Filter::eq(META_CONCEPT, "shared")), None, 1_000_000, 0)
            .unwrap();
        assert_eq!(n, page_b.items.len());

        let h1 = idx_bundle.search("xyz", 10, None).unwrap();
        let h2 = idx_portable.search("xyz", 10, None).unwrap();
        assert!(!h1.is_empty());
        assert!(!h2.is_empty());
        assert_eq!(h1.len(), h2.len());
    }

    // ---------- Unicode ----------

    #[test]
    fn unicode_body_and_frontmatter() {
        let idx = OkfIndex::new(cfg()).unwrap();
        let n = idx
            .ingest_concept(
                "u",
                "---\ntype: nota\ntitle: \"café — ñandú\"\ntags: [café, ñandú]\n---\n# Título ñandú\nContenido con emojis 🦀 y acentos.\n",
            )
            .unwrap();
        assert!(n >= 1);

        let hits = idx.search("ñandú", 10, Some("nota")).unwrap();
        assert!(!hits.is_empty());
        assert_eq!(hits[0].concept_id, "u");
        assert_eq!(hits[0].title.as_deref(), Some("café — ñandú"));
    }

    // ---------- embed_fn (semántica) ----------

    #[test]
    fn search_with_embed_fn_is_hybrid() {
        // Embedding trivial determinista: vector de dimensión 1 con valor 1.0.
        let cfg2 = OkfConfig::new(ChunkConfig::default())
            .with_dimensions(1)
            .with_embed_fn(|_s| vec![1.0f32]);
        let idx = OkfIndex::new(cfg2).unwrap();
        idx.ingest_concept(
            "e",
            "---\ntype: doc\ntitle: E\n---\nhello rust world\n",
        )
        .unwrap();
        let hits = idx.search("rust", 10, Some("doc")).unwrap();
        assert!(!hits.is_empty());
        assert_eq!(hits[0].concept_id, "e");
    }

    // ---------- helper tempdir (sin dependencias) ----------

    fn tempdir() -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "okf_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&p).unwrap();
        p
    }
}