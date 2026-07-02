//! # Índices de metadata opt-in
//!
//! Índices de metadata por campo para acelerar los filtros de equality
//! (`$eq`/`$ne`) y rango (`$gt`/`$gte`/`$lt`/`$lte`) de
//! [`crate::query::Filter`], de modo que las consultas sobre campos indexados
//! sean sub-lineales en lugar de un O(n) documento-a-documento.
//!
//! Este módulo es **lógica pura de índice**: no conoce `VectorDB` ni el
//! query planner. El integrador mantiene los índices vía `on_insert` /
//! `on_delete` / `on_update` / `on_clear` y consulta candidatos vía
//! `candidates_eq` / `candidates_range`.
//!
//! ## Semántica de los candidatos (crítico para el planner)
//!
//! Tanto [`MetadataIndexManager::candidates_eq`] como
//! [`MetadataIndexManager::candidates_range`] devuelven `Option`:
//!
//! - `None` → **el campo no está indexado** (o el tipo de consulta no es
//!   indexable en v1). El caller debe hacer full-scan y evaluar el filtro
//!   directamente. Es un "no sé".
//! - `Some(set)` → **el campo está indexado y la respuesta es definitiva**:
//!   `set` es exactamente el conjunto de ids que cumple la condición. Un
//!   `Some(set vacío)` significa "indexado y con seguridad ningún documento
//!   cumple" — no es lo mismo que `None`.
//!
//! El índice está diseñado para **nunca** devolver un conjunto distinto al
//! de la evaluación directa del filtro: cuando no puede replicar la
//! semántica exacta (p. ej. igualdad de floats con tolerancia épsilon, o
//! enteros fuera del rango exacto de f64), devuelve `None` y fuerza el
//! fallback a scan.
//!
//! ## Tipos indexables en v1
//!
//! - **Igualdad**: `String`, `Int`, `Bool` con igualdad exacta (coincide 1:1
//!   con `values_equal` de `operators.rs`). `Float` se mantiene en el mapa
//!   de igualdad vía `f64::to_bits` pero `candidates_eq` devuelve `None`
//!   para floats (la igualdad del filtro es épsilon-tolerante, no
//!   replicable por hash). Si el campo tiene **algún** valor `Float`, la
//!   igualdad por `Int` también cae a `None` (cruce Int↔Float del filtro).
//!   `List`/`Map` no son indexables (ver más abajo).
//! - **Rango**: `String` (orden lexicográfico, igual a `cmp` de Rust) y
//!   numérico (`Int` + `Float` unificados en un `BTreeMap` ordenado por
//!   `f64::total_cmp`). El cruce Int↔Float se resuelve promoviendo `Int` a
//!   `f64`, exactamente como hace `compare_values`. `-0.0` se canóniza a
//!   `+0.0` para coincidir con `partial_cmp` (que los trata iguales). `NaN`
//!   no se indexa (el filtro nunca lo matchea en rango) y un umbral `NaN`
//!   produce `Some(vacío)`.
//! - `Bool`, `List`, `Map` no participan del índice de rango: el filtro
//!   nunca los compara con orden, así que un umbral de estos tipos produce
//!   `Some(vacío)`.
//!
//! ## NaN en floats
//!
//! `MetadataValue::Float(f64)` admite cualquier `f64`, incluido `NaN` (la
//! API no filtra). Política total:
//! - En **igualdad**, `NaN` ocupa un único bucket bajo `f64::to_bits(NaN)`
//!   (los bits de `NaN` son estables en Rust), pero `candidates_eq` para
//!   floats devuelve `None` siempre, así que el bucket no se consulta.
//! - En **rango**, `NaN` no se inserta en el `BTreeMap` (el filtro devuelve
//!   `None` en `partial_cmp` → ninguna comparación de rango matchea). Un
//!   umbral `NaN` devuelve `Some(vacío)`.
//!
//! ## List / Map
//!
//! `values_equal` no tiene rama `List`/`Map` → nunca son iguales a nada, y
//! `compare_values` los resuelve como `None` → nunca matchean rango. El
//! operador `Contains` es *substring sobre `String`*, no membresía en
//! lista; `In` es "el valor almacenado ∈ lista-del-query", no "el query ∈
//! lista-almacenada". Por tanto indexar elementos de `List` no beneficia a
//! ningún operador actual: en v1 `List`/`Map` se declaran **no indexables**
//! y se ignoran al mantener el índice (un doc con valor `List`/`Map` en un
//! campo indexado queda sin entrada, igual que un doc sin ese campo).

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::ops::Bound;

use parking_lot::RwLock;

use crate::error::{Error, Result};
use crate::types::{Metadata, MetadataValue, VectorId};

/// Umbral a partir del cual un `i64` no se representa exactamente como
/// `f64`. Fuera de este rango, promover `Int` a `f64` pierde precisión y el
/// índice numérico unificado dejaría de coincidir con la comparación
/// exacta `i64.cmp` del filtro; por eso se fuerza fallback.
const F64_EXACT_INT_BOUND: u64 = 1u64 << 53;

/// Operador de rango soportado por el índice.
///
/// Espejo de los operadores de comparación de [`crate::query::FilterOp`]
/// que el índice sabe resolver sub-linealmente.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeOp {
    /// Mayor que (`>`).
    Gt,
    /// Mayor o igual que (`>=`).
    Gte,
    /// Menor que (`<`).
    Lt,
    /// Menor o igual que (`<=`).
    Lte,
}

/// Clave hashable para el índice de igualdad.
///
/// `f64` no implementa `Hash`/`Eq`, así que para `Float` se usan los bits
/// (`f64::to_bits`), que dan una identidad total y determinista. Los
/// `List`/`Map` no aparecen: no son indexables en v1.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
enum IndexKey {
    Str(String),
    Int(i64),
    Bool(bool),
    /// Bits de un `f64` (`f64::to_bits`). Estable incluido para `NaN`.
    Float(u64),
}

impl IndexKey {
    /// Construye la clave de igualdad para un valor, o `None` si el tipo no
    /// es indexable (`List`/`Map`).
    fn from_value(v: &MetadataValue) -> Option<Self> {
        match v {
            MetadataValue::String(s) => Some(Self::Str(s.clone())),
            MetadataValue::Int(i) => Some(Self::Int(*i)),
            MetadataValue::Bool(b) => Some(Self::Bool(*b)),
            MetadataValue::Float(f) => Some(Self::Float(f.to_bits())),
            MetadataValue::List(_) | MetadataValue::Map(_) => None,
        }
    }
}

/// Wrapper de `f64` con orden total vía [`f64::total_cmp`].
///
/// Se usa como clave del `BTreeMap` numérico de rango. Solo se almacenan
/// valores no-`NaN` y canónizados (`-0.0` → `+0.0`), de modo que
/// `total_cmp` coincide con `partial_cmp` del filtro para todo valor
/// indexado.
#[derive(Debug, Clone, Copy)]
struct NumKey(f64);

impl PartialEq for NumKey {
    fn eq(&self, other: &Self) -> bool {
        self.0.total_cmp(&other.0) == Ordering::Equal
    }
}

impl Eq for NumKey {}

impl PartialOrd for NumKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NumKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

/// Canóniza `-0.0` a `+0.0`. `partial_cmp` trata `-0.0` y `+0.0` como
/// iguales (`Equal`); sin canónizar, `total_cmp` los ordenaría distinto y
/// las queries de rango divergerían del filtro.
fn canon_float(f: f64) -> f64 {
    if f == 0.0 {
        0.0f64
    } else {
        f
    }
}

/// `true` si un `i64` está fuera del rango exactamente representable por
/// `f64` (|i| >= 2^53). Usa `unsigned_abs` para no desbordar en
/// `i64::MIN`.
fn is_large_int(i: i64) -> bool {
    i.unsigned_abs() >= F64_EXACT_INT_BOUND
}

/// Estado de un índice sobre un campo.
#[derive(Debug, Default)]
struct FieldIndex {
    /// Igualdad: valor (hashable) → ids.
    eq: HashMap<IndexKey, HashSet<VectorId>>,
    /// Rango sobre strings (orden lexicográfico de Rust).
    string_range: BTreeMap<String, HashSet<VectorId>>,
    /// Rango numérico unificado (Int + Float promocionados a f64).
    num_range: BTreeMap<NumKey, HashSet<VectorId>>,
    /// Nº de valores `Float` actualmente indexados en este campo. Si > 0,
    /// la igualdad por `Int` debe caer a `None` (cruce Int↔Float del
    /// filtro con tolerancia épsilon).
    n_float: usize,
    /// Nº de valores `Int` con |i| >= 2^53 indexados. Si > 0, las queries
    /// de rango numérico caen a `None` (la promoción a f64 perdería
    /// precisión vs `i64.cmp` del filtro).
    n_large_int: usize,
}

impl FieldIndex {
    /// Indexa `id` bajo el valor `v` para este campo.
    fn index_value(&mut self, id: &VectorId, v: &MetadataValue) {
        // Igualdad (String/Int/Bool/Float). List/Map: no indexable.
        if let Some(key) = IndexKey::from_value(v) {
            self.eq.entry(key).or_default().insert(id.clone());
        }

        // Rango.
        match v {
            MetadataValue::String(s) => {
                self.string_range.entry(s.clone()).or_default().insert(id.clone());
            }
            MetadataValue::Int(i) => {
                let key = NumKey(canon_float(*i as f64));
                self.num_range.entry(key).or_default().insert(id.clone());
                if is_large_int(*i) {
                    self.n_large_int += 1;
                }
            }
            MetadataValue::Float(f) => {
                // NaN no se indexa en rango (el filtro nunca lo matchea).
                if !f.is_nan() {
                    let key = NumKey(canon_float(*f));
                    self.num_range.entry(key).or_default().insert(id.clone());
                }
                self.n_float += 1;
            }
            // Bool/List/Map no participan del índice de rango.
            _ => {}
        }
    }

    /// Desindexa `id` del valor `v` para este campo.
    fn remove_value(&mut self, id: &VectorId, v: &MetadataValue) {
        if let Some(key) = IndexKey::from_value(v) {
            remove_from_bucket(&mut self.eq, key, id);
        }

        match v {
            MetadataValue::String(s) => {
                remove_from_btree_bucket(&mut self.string_range, s.clone(), id);
            }
            MetadataValue::Int(i) => {
                let key = NumKey(canon_float(*i as f64));
                remove_from_btree_bucket(&mut self.num_range, key, id);
                if is_large_int(*i) {
                    self.n_large_int = self.n_large_int.saturating_sub(1);
                }
            }
            MetadataValue::Float(f) => {
                if !f.is_nan() {
                    let key = NumKey(canon_float(*f));
                    remove_from_btree_bucket(&mut self.num_range, key, id);
                }
                self.n_float = self.n_float.saturating_sub(1);
            }
            _ => {}
        }
    }

    /// Desindexa `id` de todos los buckets del campo (scan), sin
    /// conocimiento del valor previo. Se usa cuando el integrador no
    /// dispone de la metadata vieja.
    ///
    /// No ajusta `n_float`/`n_large_int` (no puede saber qué había); los
    /// deja conservadores, lo que sólo puede forzar más fallbacks `None`,
    /// nunca resultados incorrectos.
    fn remove_id_everywhere(&mut self, id: &VectorId) {
        self.eq.retain(|_, set| {
            set.remove(id);
            !set.is_empty()
        });
        self.string_range.retain(|_, set| {
            set.remove(id);
            !set.is_empty()
        });
        self.num_range.retain(|_, set| {
            set.remove(id);
            !set.is_empty()
        });
    }

    /// Vacía todos los buckets conservando los contadores de tipo a 0.
    fn clear(&mut self) {
        self.eq.clear();
        self.string_range.clear();
        self.num_range.clear();
        self.n_float = 0;
        self.n_large_int = 0;
    }
}

fn remove_from_bucket<K: std::hash::Hash + Eq>(
    map: &mut HashMap<K, HashSet<VectorId>>,
    key: K,
    id: &VectorId,
) {
    if let Some(set) = map.get_mut(&key) {
        set.remove(id);
        if set.is_empty() {
            map.remove(&key);
        }
    }
}

fn remove_from_btree_bucket<K: Ord>(
    map: &mut BTreeMap<K, HashSet<VectorId>>,
    key: K,
    id: &VectorId,
) {
    if let Some(set) = map.get_mut(&key) {
        set.remove(id);
        if set.is_empty() {
            map.remove(&key);
        }
    }
}

/// Gestor de índices de metadata opt-in, por campo.
///
/// Thread-safe vía interior mutability con `parking_lot::RwLock` (patrón
/// de [`crate::partial_index::PartialIndexManager`]).
pub struct MetadataIndexManager {
    fields: RwLock<HashMap<String, FieldIndex>>,
}

impl Default for MetadataIndexManager {
    fn default() -> Self {
        Self::new()
    }
}

impl MetadataIndexManager {
    /// Crea un gestor sin índices.
    pub fn new() -> Self {
        Self {
            fields: RwLock::new(HashMap::new()),
        }
    }

    /// Registra un índice sobre `field`.
    ///
    /// No reindexa documentos existentes; el integrador debe alimentar el
    /// índice con `on_insert` para los docs ya presentes.
    pub fn create_index(&self, field: &str) -> Result<()> {
        let mut fields = self.fields.write();
        if fields.contains_key(field) {
            return Err(Error::AlreadyExists(field.to_string()));
        }
        fields.insert(field.to_string(), FieldIndex::default());
        Ok(())
    }

    /// Elimina el índice sobre `field`. Tras esto, `candidates_*` devuelven
    /// `None` para ese campo.
    pub fn drop_index(&self, field: &str) -> Result<()> {
        let mut fields = self.fields.write();
        if fields.remove(field).is_none() {
            return Err(Error::NotFound(field.to_string()));
        }
        Ok(())
    }

    /// Lista los campos indexados, en orden lexicográfico (determinista).
    pub fn list_indexes(&self) -> Vec<String> {
        let mut names: Vec<String> = self.fields.read().keys().cloned().collect();
        names.sort();
        names
    }

    /// `true` si `field` tiene índice registrado.
    pub fn has_index(&self, field: &str) -> bool {
        self.fields.read().contains_key(field)
    }

    /// Notifica la inserción de un documento: indexa `id` bajo los campos
    /// indexados presentes en `metadata`.
    pub fn on_insert(&self, id: &VectorId, metadata: Option<&Metadata>) {
        let mut fields = self.fields.write();
        let metadata = match metadata {
            Some(m) => m,
            None => return,
        };
        for (field, state) in fields.iter_mut() {
            if let Some(v) = metadata.get(field) {
                state.index_value(id, v);
            }
        }
    }

    /// Notifica la eliminación de un documento: desindexa `id` usando la
    /// metadata **vieja**.
    ///
    /// Requiere la metadata vieja para desindexar por valor (barato y con
    /// contadores exactos). Si se pasa `None` (o el campo no estaba), hace
    /// un scan del campo para igualmente sacar el `id`, pero no ajusta los
    /// contadores de tipo — quedan conservadores, lo que sólo puede
    /// provocar más fallbacks `None`, nunca resultados incorrectos.
    pub fn on_delete(&self, id: &VectorId, old_metadata: Option<&Metadata>) {
        let mut fields = self.fields.write();
        let old = old_metadata.as_ref();
        for (field, state) in fields.iter_mut() {
            match old.and_then(|m| m.get(field)) {
                Some(v) => state.remove_value(id, v),
                None => state.remove_id_everywhere(id),
            }
        }
    }

    /// Notifica la actualización de un documento: desindexa con la metadata
    /// vieja y reindexa con la nueva.
    pub fn on_update(
        &self,
        id: &VectorId,
        old_metadata: Option<&Metadata>,
        new_metadata: Option<&Metadata>,
    ) {
        self.on_delete(id, old_metadata);
        self.on_insert(id, new_metadata);
    }

    /// Vacía todos los buckets pero **conserva los índices registrados**,
    /// de modo que inserciones futuras vuelvan a poblarlos (análogo a
    /// `PartialIndexManager::clear_all`).
    pub fn on_clear(&self) {
        let mut fields = self.fields.write();
        for state in fields.values_mut() {
            state.clear();
        }
    }

    /// Candidatos para `$eq(field, value)`.
    ///
    /// Ver la semántica `None` vs `Some(vacío)` en la documentación del
    /// módulo.
    pub fn candidates_eq(&self, field: &str, value: &MetadataValue) -> Option<HashSet<VectorId>> {
        let fields = self.fields.read();
        let state = fields.get(field)?;

        match value {
            MetadataValue::String(s) => {
                Some(state.eq.get(&IndexKey::Str(s.clone())).cloned().unwrap_or_default())
            }
            MetadataValue::Int(i) => {
                // Si hay floats en el campo, el filtro hace cruce
                // Int↔Float (épsilon); el índice no lo replica → fallback.
                if state.n_float > 0 {
                    return None;
                }
                Some(state.eq.get(&IndexKey::Int(*i)).cloned().unwrap_or_default())
            }
            MetadataValue::Bool(b) => {
                Some(state.eq.get(&IndexKey::Bool(*b)).cloned().unwrap_or_default())
            }
            MetadataValue::Float(_) => {
                // Igualdad de floats del filtro es épsilon-tolerante, no
                // replicable por hash → fallback a scan.
                None
            }
            MetadataValue::List(_) | MetadataValue::Map(_) => {
                // No indexables en v1.
                None
            }
        }
    }

    /// Candidatos para un operador de rango sobre `field`.
    ///
    /// Ver la semántica `None` vs `Some(vacío)` en la documentación del
    /// módulo.
    pub fn candidates_range(
        &self,
        field: &str,
        op: RangeOp,
        value: &MetadataValue,
    ) -> Option<HashSet<VectorId>> {
        let fields = self.fields.read();
        let state = fields.get(field)?;

        match value {
            MetadataValue::String(s) => Some(range_query_string(state, op, s)),
            MetadataValue::Int(i) => {
                if state.n_large_int > 0 {
                    return None;
                }
                let thr = canon_float(*i as f64);
                Some(range_query_num(state, op, NumKey(thr)))
            }
            MetadataValue::Float(f) => {
                if f.is_nan() {
                    // El filtro: partial_cmp(NaN) → None → ninguna
                    // comparación de rango matchea → vacío definitivo.
                    return Some(HashSet::new());
                }
                if state.n_large_int > 0 {
                    return None;
                }
                let thr = canon_float(*f);
                Some(range_query_num(state, op, NumKey(thr)))
            }
            // Bool/List/Map: compare_values → None → 0 matches definitivos.
            MetadataValue::Bool(_) | MetadataValue::List(_) | MetadataValue::Map(_) => {
                Some(HashSet::new())
            }
        }
    }
}

/// Ejecuta una query de rango sobre el `BTreeMap` de strings.
fn range_query_string(
    state: &FieldIndex,
    op: RangeOp,
    thr: &str,
) -> HashSet<VectorId> {
    let mut out = HashSet::new();
    let range = match op {
        RangeOp::Gt => state
            .string_range
            .range::<str, _>((Bound::Excluded(thr), Bound::Unbounded)),
        RangeOp::Gte => state
            .string_range
            .range::<str, _>((Bound::Included(thr), Bound::Unbounded)),
        RangeOp::Lt => state
            .string_range
            .range::<str, _>((Bound::Unbounded, Bound::Excluded(thr))),
        RangeOp::Lte => state
            .string_range
            .range::<str, _>((Bound::Unbounded, Bound::Included(thr))),
    };
    for (_k, set) in range {
        out.extend(set.iter().cloned());
    }
    out
}

/// Ejecuta una query de rango sobre el `BTreeMap` numérico unificado.
fn range_query_num(state: &FieldIndex, op: RangeOp, thr: NumKey) -> HashSet<VectorId> {
    let mut out = HashSet::new();
    let range = match op {
        RangeOp::Gt => state.num_range.range((Bound::Excluded(thr), Bound::Unbounded)),
        RangeOp::Gte => state.num_range.range((Bound::Included(thr), Bound::Unbounded)),
        RangeOp::Lt => state.num_range.range((Bound::Unbounded, Bound::Excluded(thr))),
        RangeOp::Lte => state.num_range.range((Bound::Unbounded, Bound::Included(thr))),
    };
    for (_k, set) in range {
        out.extend(set.iter().cloned());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::{Filter, FilterEvaluator};
    use crate::types::MetadataValue;

    /// Metadatos helper: pares (campo, valor).
    fn meta(pairs: &[(&str, MetadataValue)]) -> Metadata {
        let mut m = Metadata::new();
        for (k, v) in pairs {
            m.insert(*k, v.clone());
        }
        m
    }

    fn v_str(s: &str) -> MetadataValue {
        MetadataValue::String(s.to_string())
    }

    /// Oráculo: ids que cumplen `Filter::eq(field, value)` evaluado directo.
    fn oracle_eq(
        docs: &[(VectorId, Option<Metadata>)],
        field: &str,
        value: &MetadataValue,
    ) -> HashSet<VectorId> {
        let f = Filter::Condition {
            field: field.to_string(),
            op: crate::query::FilterOp::Eq(value.clone()),
        };
        docs.iter()
            .filter(|(_, m)| FilterEvaluator::evaluate(&f, m.as_ref()))
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Oráculo: ids que cumplen un operador de rango evaluado directo.
    fn oracle_range(
        docs: &[(VectorId, Option<Metadata>)],
        field: &str,
        op: RangeOp,
        value: &MetadataValue,
    ) -> HashSet<VectorId> {
        let fop = match op {
            RangeOp::Gt => crate::query::FilterOp::Gt(value.clone()),
            RangeOp::Gte => crate::query::FilterOp::Gte(value.clone()),
            RangeOp::Lt => crate::query::FilterOp::Lt(value.clone()),
            RangeOp::Lte => crate::query::FilterOp::Lte(value.clone()),
        };
        let f = Filter::Condition {
            field: field.to_string(),
            op: fop,
        };
        docs.iter()
            .filter(|(_, m)| FilterEvaluator::evaluate(&f, m.as_ref()))
            .map(|(id, _)| id.clone())
            .collect()
    }

    #[test]
    fn create_eq_string_int_bool() {
        let mgr = MetadataIndexManager::new();
        mgr.create_index("category").unwrap();
        mgr.create_index("count").unwrap();
        mgr.create_index("active").unwrap();

        let docs = vec![
            ("doc1".to_string(), Some(meta(&[("category", v_str("tech")), ("count", MetadataValue::Int(42)), ("active", MetadataValue::Bool(true))]))),
            ("doc2".to_string(), Some(meta(&[("category", v_str("tech")), ("count", MetadataValue::Int(7)), ("active", MetadataValue::Bool(false))]))),
            ("doc3".to_string(), Some(meta(&[("category", v_str("news")), ("count", MetadataValue::Int(42)), ("active", MetadataValue::Bool(true))]))),
            ("doc4".to_string(), Some(meta(&[("category", v_str("sports"))]))),
        ];
        for (id, m) in &docs {
            mgr.on_insert(id, m.as_ref());
        }

        // eq
        let got = mgr.candidates_eq("category", &v_str("tech")).unwrap();
        assert_eq!(got, HashSet::from(["doc1".to_string(), "doc2".to_string()]));

        let got = mgr.candidates_eq("count", &MetadataValue::Int(42)).unwrap();
        assert_eq!(got, HashSet::from(["doc1".to_string(), "doc3".to_string()]));

        let got = mgr.candidates_eq("active", &MetadataValue::Bool(true)).unwrap();
        assert_eq!(got, HashSet::from(["doc1".to_string(), "doc3".to_string()]));

        // Indexado, sin matches -> Some(vacío), NO None.
        let got = mgr.candidates_eq("category", &v_str("missing")).unwrap();
        assert!(got.is_empty());

        // Equivalencia con oráculo.
        assert_eq!(
            mgr.candidates_eq("category", &v_str("news")).unwrap(),
            oracle_eq(&docs, "category", &v_str("news"))
        );
    }

    #[test]
    fn none_vs_empty_distinction() {
        let mgr = MetadataIndexManager::new();
        // Sin índice -> None.
        assert!(mgr.candidates_eq("nope", &v_str("x")).is_none());
        assert!(mgr
            .candidates_range("nope", RangeOp::Gt, &v_str("x"))
            .is_none());

        mgr.create_index("f").unwrap();
        mgr.on_insert(&"d1".to_string(), Some(&meta(&[("f", v_str("a"))])));
        // Indexado, sin matches -> Some(vacío).
        assert_eq!(mgr.candidates_eq("f", &v_str("zzz")).unwrap(), HashSet::new());
        assert_eq!(
            mgr.candidates_range("f", RangeOp::Gt, &v_str("zzz")).unwrap(),
            HashSet::new()
        );
    }

    #[test]
    fn update_moves_bucket() {
        let mgr = MetadataIndexManager::new();
        mgr.create_index("cat").unwrap();
        let id = "d1".to_string();
        let old = meta(&[("cat", v_str("a"))]);
        let new = meta(&[("cat", v_str("b"))]);
        mgr.on_insert(&id, Some(&old));

        assert!(mgr.candidates_eq("cat", &v_str("a")).unwrap().contains(&id));
        mgr.on_update(&id, Some(&old), Some(&new));
        // Ya no está en bucket "a", ahora en "b".
        assert!(!mgr.candidates_eq("cat", &v_str("a")).unwrap().contains(&id));
        assert!(mgr.candidates_eq("cat", &v_str("b")).unwrap().contains(&id));
    }

    #[test]
    fn delete_removes() {
        let mgr = MetadataIndexManager::new();
        mgr.create_index("cat").unwrap();
        let id = "d1".to_string();
        let m = meta(&[("cat", v_str("a"))]);
        mgr.on_insert(&id, Some(&m));
        assert!(mgr.candidates_eq("cat", &v_str("a")).unwrap().contains(&id));
        mgr.on_delete(&id, Some(&m));
        assert!(!mgr.candidates_eq("cat", &v_str("a")).unwrap().contains(&id));
    }

    #[test]
    fn delete_without_old_metadata_scans() {
        let mgr = MetadataIndexManager::new();
        mgr.create_index("cat").unwrap();
        let id = "d1".to_string();
        mgr.on_insert(&id, Some(&meta(&[("cat", v_str("a"))])));
        // old_metadata None -> scan removal.
        mgr.on_delete(&id, None);
        assert!(!mgr.candidates_eq("cat", &v_str("a")).unwrap().contains(&id));
    }

    #[test]
    fn clear_empties_but_keeps_registrations() {
        let mgr = MetadataIndexManager::new();
        mgr.create_index("cat").unwrap();
        mgr.on_insert(&"d1".to_string(), Some(&meta(&[("cat", v_str("a"))])));
        assert!(!mgr.candidates_eq("cat", &v_str("a")).unwrap().is_empty());

        mgr.on_clear();
        assert!(mgr.candidates_eq("cat", &v_str("a")).unwrap().is_empty());
        // Sigue registrado: captura inserciones futuras.
        assert!(mgr.has_index("cat"));
        mgr.on_insert(&"d2".to_string(), Some(&meta(&[("cat", v_str("a"))])));
        assert!(mgr.candidates_eq("cat", &v_str("a")).unwrap().contains(&"d2".to_string()));
    }

    #[test]
    fn drop_index_returns_none() {
        let mgr = MetadataIndexManager::new();
        mgr.create_index("cat").unwrap();
        mgr.on_insert(&"d1".to_string(), Some(&meta(&[("cat", v_str("a"))])));
        assert!(mgr.candidates_eq("cat", &v_str("a")).is_some());

        mgr.drop_index("cat").unwrap();
        assert!(mgr.candidates_eq("cat", &v_str("a")).is_none());
        assert!(mgr.candidates_range("cat", RangeOp::Gt, &v_str("a")).is_none());
        assert!(!mgr.has_index("cat"));

        // drop inexistente -> NotFound.
        assert!(matches!(mgr.drop_index("cat"), Err(Error::NotFound(_))));
        // create duplicado -> AlreadyExists.
        mgr.create_index("x").unwrap();
        assert!(matches!(mgr.create_index("x"), Err(Error::AlreadyExists(_))));
    }

    #[test]
    fn float_range_negatives_zero_extremes_and_equivalence() {
        let mgr = MetadataIndexManager::new();
        mgr.create_index("score").unwrap();
        let docs: Vec<(VectorId, Option<Metadata>)> = vec![
            ("a".to_string(), Some(meta(&[("score", MetadataValue::Float(-10.0))]))),
            ("b".to_string(), Some(meta(&[("score", MetadataValue::Float(-0.0))]))),
            ("c".to_string(), Some(meta(&[("score", MetadataValue::Float(0.0))]))),
            ("d".to_string(), Some(meta(&[("score", MetadataValue::Float(0.5))]))),
            ("e".to_string(), Some(meta(&[("score", MetadataValue::Float(1e308))]))),
            ("f".to_string(), Some(meta(&[("score", MetadataValue::Float(-1e308))]))),
        ];
        for (id, m) in &docs {
            mgr.on_insert(id, m.as_ref());
        }

        for (op, thr) in [
            (RangeOp::Gt, MetadataValue::Float(0.0)),
            (RangeOp::Gte, MetadataValue::Float(0.0)),
            (RangeOp::Lt, MetadataValue::Float(0.0)),
            (RangeOp::Lte, MetadataValue::Float(0.0)),
            (RangeOp::Gt, MetadataValue::Float(-1e308)),
            (RangeOp::Lt, MetadataValue::Float(1e308)),
            (RangeOp::Gte, MetadataValue::Float(-10.0)),
            (RangeOp::Lte, MetadataValue::Float(-10.0)),
        ] {
            let got = mgr.candidates_range("score", op, &thr).unwrap();
            let want = oracle_range(&docs, "score", op, &thr);
            assert_eq!(got, want, "mismatch op={:?} thr={:?}", op, thr);
        }
    }

    #[test]
    fn neg_zero_canonicalized_matches_filter() {
        // El filtro trata -0.0 y +0.0 como Equal (partial_cmp).
        let mgr = MetadataIndexManager::new();
        mgr.create_index("x").unwrap();
        let docs: Vec<(VectorId, Option<Metadata>)> = vec![
            ("neg".to_string(), Some(meta(&[("x", MetadataValue::Float(-0.0))]))),
            ("pos".to_string(), Some(meta(&[("x", MetadataValue::Float(0.0))]))),
            ("half".to_string(), Some(meta(&[("x", MetadataValue::Float(0.5))]))),
        ];
        for (id, m) in &docs {
            mgr.on_insert(id, m.as_ref());
        }
        // $gte(0.0) debe incluir tanto -0.0 como +0.0.
        let got = mgr.candidates_range("x", RangeOp::Gte, &MetadataValue::Float(0.0)).unwrap();
        let want = oracle_range(&docs, "x", RangeOp::Gte, &MetadataValue::Float(0.0));
        assert_eq!(got, want);
        assert!(got.contains("neg") && got.contains("pos"));

        // $lt(0.0) excluye ambos ceros.
        let got = mgr.candidates_range("x", RangeOp::Lt, &MetadataValue::Float(0.0)).unwrap();
        let want = oracle_range(&docs, "x", RangeOp::Lt, &MetadataValue::Float(0.0));
        assert_eq!(got, want);
        assert!(!got.contains("neg") && !got.contains("pos"));
    }

    #[test]
    fn int_range_equivalence() {
        let mgr = MetadataIndexManager::new();
        mgr.create_index("n").unwrap();
        let docs: Vec<(VectorId, Option<Metadata>)> = vec![
            ("a".to_string(), Some(meta(&[("n", MetadataValue::Int(-5))]))),
            ("b".to_string(), Some(meta(&[("n", MetadataValue::Int(0))]))),
            ("c".to_string(), Some(meta(&[("n", MetadataValue::Int(10))]))),
            ("d".to_string(), Some(meta(&[("n", MetadataValue::Int(10))]))),
            ("e".to_string(), Some(meta(&[("n", MetadataValue::Int(100))]))),
        ];
        for (id, m) in &docs {
            mgr.on_insert(id, m.as_ref());
        }
        for (op, thr) in [
            (RangeOp::Gt, MetadataValue::Int(10)),
            (RangeOp::Gte, MetadataValue::Int(10)),
            (RangeOp::Lt, MetadataValue::Int(10)),
            (RangeOp::Lte, MetadataValue::Int(10)),
            (RangeOp::Gte, MetadataValue::Int(-5)),
            (RangeOp::Gt, MetadataValue::Int(-100)),
        ] {
            let got = mgr.candidates_range("n", op, &thr).unwrap();
            let want = oracle_range(&docs, "n", op, &thr);
            assert_eq!(got, want, "mismatch op={:?} thr={:?}", op, thr);
        }
    }

    #[test]
    fn string_range_equivalence() {
        let mgr = MetadataIndexManager::new();
        mgr.create_index("s").unwrap();
        let docs: Vec<(VectorId, Option<Metadata>)> = vec![
            ("a".to_string(), Some(meta(&[("s", v_str("apple"))]))),
            ("b".to_string(), Some(meta(&[("s", v_str("banana"))]))),
            ("c".to_string(), Some(meta(&[("s", v_str("cherry"))]))),
            ("d".to_string(), Some(meta(&[("s", v_str("Apple"))]))), // mayúscula < minúscula
        ];
        for (id, m) in &docs {
            mgr.on_insert(id, m.as_ref());
        }
        for (op, thr) in [
            (RangeOp::Gt, v_str("apple")),
            (RangeOp::Gte, v_str("apple")),
            (RangeOp::Lt, v_str("cherry")),
            (RangeOp::Lte, v_str("banana")),
        ] {
            let got = mgr.candidates_range("s", op, &thr).unwrap();
            let want = oracle_range(&docs, "s", op, &thr);
            assert_eq!(got, want, "mismatch op={:?} thr={:?}", op, thr);
        }
    }

    #[test]
    fn mixed_int_float_range_unified_matches_oracle() {
        // Campo con Int y Float mezclados: el cruce se resuelve promoviendo
        // Int a f64, igual que compare_values.
        let mgr = MetadataIndexManager::new();
        mgr.create_index("v").unwrap();
        let docs: Vec<(VectorId, Option<Metadata>)> = vec![
            ("i3".to_string(), Some(meta(&[("v", MetadataValue::Int(3))]))),
            ("i4".to_string(), Some(meta(&[("v", MetadataValue::Int(4))]))),
            ("f3".to_string(), Some(meta(&[("v", MetadataValue::Float(3.0))]))),
            ("f3_5".to_string(), Some(meta(&[("v", MetadataValue::Float(3.5))]))),
            ("f2".to_string(), Some(meta(&[("v", MetadataValue::Float(2.0))]))),
        ];
        for (id, m) in &docs {
            mgr.on_insert(id, m.as_ref());
        }

        // Umbral Int: debe matchear Float-stored por promoción.
        for (op, thr) in [
            (RangeOp::Gt, MetadataValue::Int(3)),
            (RangeOp::Gte, MetadataValue::Int(3)),
            (RangeOp::Lt, MetadataValue::Int(3)),
        ] {
            let got = mgr.candidates_range("v", op, &thr).unwrap();
            let want = oracle_range(&docs, "v", op, &thr);
            assert_eq!(got, want, "int-thr mismatch op={:?}", op);
        }
        // Umbral Float: debe matchear Int-stored por promoción.
        for (op, thr) in [
            (RangeOp::Gt, MetadataValue::Float(3.0)),
            (RangeOp::Gte, MetadataValue::Float(3.0)),
            (RangeOp::Lt, MetadataValue::Float(3.0)),
        ] {
            let got = mgr.candidates_range("v", op, &thr).unwrap();
            let want = oracle_range(&docs, "v", op, &thr);
            assert_eq!(got, want, "float-thr mismatch op={:?}", op);
        }
    }

    #[test]
    fn float_eq_falls_back_to_none() {
        let mgr = MetadataIndexManager::new();
        mgr.create_index("f").unwrap();
        mgr.on_insert(&"d1".to_string(), Some(&meta(&[("f", MetadataValue::Float(1.0))])));
        // Igualdad de floats -> None (épsilon del filtro no replicable).
        assert!(mgr.candidates_eq("f", &MetadataValue::Float(1.0)).is_none());
        // Pero el rango sí funciona.
        assert!(mgr
            .candidates_range("f", RangeOp::Gte, &MetadataValue::Float(1.0))
            .is_some());
    }

    #[test]
    fn int_eq_falls_back_when_field_has_floats() {
        let mgr = MetadataIndexManager::new();
        mgr.create_index("v").unwrap();
        mgr.on_insert(&"d1".to_string(), Some(&meta(&[("v", MetadataValue::Int(3))])));
        // Sin floats aún: Int eq sirve.
        assert!(mgr.candidates_eq("v", &MetadataValue::Int(3)).is_some());
        mgr.on_insert(&"d2".to_string(), Some(&meta(&[("v", MetadataValue::Float(3.0))])));
        // Ahora hay float: cruce Int↔Float -> fallback.
        assert!(mgr.candidates_eq("v", &MetadataValue::Int(3)).is_none());
    }

    #[test]
    fn nan_range_is_empty_and_not_indexed() {
        let mgr = MetadataIndexManager::new();
        mgr.create_index("f").unwrap();
        let docs: Vec<(VectorId, Option<Metadata>)> = vec![
            ("nan".to_string(), Some(meta(&[("f", MetadataValue::Float(f64::NAN))]))),
            ("one".to_string(), Some(meta(&[("f", MetadataValue::Float(1.0))]))),
            ("two".to_string(), Some(meta(&[("f", MetadataValue::Float(2.0))]))),
        ];
        for (id, m) in &docs {
            mgr.on_insert(id, m.as_ref());
        }
        // Umbral NaN -> vacío definitivo (Some(empty)), no None.
        assert_eq!(
            mgr.candidates_range("f", RangeOp::Gte, &MetadataValue::Float(f64::NAN)).unwrap(),
            HashSet::new()
        );
        // NaN almacenado no matchea ningún rango: $gte(0.0) = {one, two}, sin nan.
        let got = mgr.candidates_range("f", RangeOp::Gte, &MetadataValue::Float(0.0)).unwrap();
        let want = oracle_range(&docs, "f", RangeOp::Gte, &MetadataValue::Float(0.0));
        assert_eq!(got, want);
        assert!(!got.contains("nan"));
    }

    #[test]
    fn large_int_range_falls_back() {
        let mgr = MetadataIndexManager::new();
        mgr.create_index("n").unwrap();
        let big = (1i64 << 53) + 1;
        mgr.on_insert(&"d1".to_string(), Some(&meta(&[("n", MetadataValue::Int(big))])));
        // |i| >= 2^53 -> rango numérico cae a None.
        assert!(mgr.candidates_range("n", RangeOp::Gte, &MetadataValue::Int(0)).is_none());
        assert!(mgr.candidates_range("n", RangeOp::Gte, &MetadataValue::Float(0.0)).is_none());
        // Igualdad de Int sigue funcionando (no afectada por large_int).
        assert!(mgr.candidates_eq("n", &MetadataValue::Int(big)).is_some());
    }

    #[test]
    fn bool_and_list_map_range_are_empty() {
        let mgr = MetadataIndexManager::new();
        mgr.create_index("f").unwrap();
        mgr.on_insert(&"d1".to_string(), Some(&meta(&[("f", MetadataValue::Int(5))])));
        // Umbral Bool/List/Map -> Some(vacío) definitivo.
        assert_eq!(
            mgr.candidates_range("f", RangeOp::Gt, &MetadataValue::Bool(true)).unwrap(),
            HashSet::new()
        );
        assert_eq!(
            mgr.candidates_range(
                "f",
                RangeOp::Gt,
                &MetadataValue::List(vec![MetadataValue::Int(1)])
            )
            .unwrap(),
            HashSet::new()
        );
    }

    #[test]
    fn list_map_values_not_indexed_for_eq() {
        let mgr = MetadataIndexManager::new();
        mgr.create_index("tags").unwrap();
        let lst = MetadataValue::List(vec![v_str("rust"), v_str("db")]);
        let mp = MetadataValue::Map(std::collections::HashMap::from([(
            "k".to_string(),
            MetadataValue::Int(1),
        )]));
        mgr.on_insert(&"d1".to_string(), Some(&meta(&[("tags", lst.clone())])));
        mgr.on_insert(&"d2".to_string(), Some(&meta(&[("tags", mp.clone())])));
        // eq sobre List/Map umbral -> None (no indexable).
        assert!(mgr.candidates_eq("tags", &lst).is_none());
        assert!(mgr.candidates_eq("tags", &mp).is_none());
    }

    #[test]
    fn unicode_ids_and_missing_fields() {
        let mgr = MetadataIndexManager::new();
        mgr.create_index("cat").unwrap();
        let docs: Vec<(VectorId, Option<Metadata>)> = vec![
            ("café-1".to_string(), Some(meta(&[("cat", v_str("técnología"))]))),
            ("日本-2".to_string(), Some(meta(&[("cat", v_str("técnología"))]))),
            ("plain".to_string(), Some(meta(&[("other", v_str("x"))]))), // sin "cat"
            ("nometa".to_string(), None),
        ];
        for (id, m) in &docs {
            mgr.on_insert(id, m.as_ref());
        }
        let got = mgr.candidates_eq("cat", &v_str("técnología")).unwrap();
        assert_eq!(got, HashSet::from(["café-1".to_string(), "日本-2".to_string()]));
        // docs sin el campo "cat" no aparecen.
        assert!(!got.contains("plain"));
        assert!(!got.contains("nometa"));
    }

    #[test]
    fn docs_without_metadata_are_noops() {
        let mgr = MetadataIndexManager::new();
        mgr.create_index("cat").unwrap();
        mgr.on_insert(&"d1".to_string(), None);
        assert!(mgr.candidates_eq("cat", &v_str("a")).unwrap().is_empty());
        // delete con None no pánico y no deja estado raro.
        mgr.on_delete(&"d1".to_string(), None);
        mgr.on_update(&"d1".to_string(), None, None);
        assert!(mgr.has_index("cat"));
    }

    #[test]
    fn list_indexes_sorted_and_has_index() {
        let mgr = MetadataIndexManager::new();
        mgr.create_index("b").unwrap();
        mgr.create_index("a").unwrap();
        mgr.create_index("c").unwrap();
        assert_eq!(mgr.list_indexes(), vec!["a", "b", "c"]);
        assert!(mgr.has_index("a"));
        assert!(!mgr.has_index("z"));
        assert!(mgr.drop_index("a").is_ok());
        assert_eq!(mgr.list_indexes(), vec!["b", "c"]);
    }

    #[test]
    fn full_equivalence_sweep_int_only_field() {
        // Barrido de eq + los 4 rangos para un campo Int-only: el índice
        // debe coincidir con el oráculo en todos los casos que devuelve Some.
        let mgr = MetadataIndexManager::new();
        mgr.create_index("n").unwrap();
        let docs: Vec<(VectorId, Option<Metadata>)> = (0..20)
            .map(|i| {
                let id = format!("d{}", i);
                let m = meta(&[("n", MetadataValue::Int(i - 10))]);
                (id, Some(m))
            })
            .collect();
        for (id, m) in &docs {
            mgr.on_insert(id, m.as_ref());
        }
        for thr in [-15i64, -10, -3, 0, 7, 10, 15] {
            let v = MetadataValue::Int(thr);
            assert_eq!(
                mgr.candidates_eq("n", &v).unwrap(),
                oracle_eq(&docs, "n", &v),
                "eq thr={}",
                thr
            );
            for op in [RangeOp::Gt, RangeOp::Gte, RangeOp::Lt, RangeOp::Lte] {
                let got = mgr.candidates_range("n", op, &v).unwrap();
                let want = oracle_range(&docs, "n", op, &v);
                assert_eq!(got, want, "range op={:?} thr={}", op, thr);
            }
        }
    }

    #[test]
    fn full_equivalence_sweep_string_field() {
        let mgr = MetadataIndexManager::new();
        mgr.create_index("s").unwrap();
        let vals = ["a", "b", "c", "d", "e", "f"];
        let docs: Vec<(VectorId, Option<Metadata>)> = vals
            .iter()
            .enumerate()
            .map(|(i, s)| (format!("d{}", i), Some(meta(&[("s", v_str(s))]))))
            .collect();
        for (id, m) in &docs {
            mgr.on_insert(id, m.as_ref());
        }
        for thr in ["a", "c", "f", "z"] {
            let v = v_str(thr);
            assert_eq!(mgr.candidates_eq("s", &v).unwrap(), oracle_eq(&docs, "s", &v));
            for op in [RangeOp::Gt, RangeOp::Gte, RangeOp::Lt, RangeOp::Lte] {
                assert_eq!(
                    mgr.candidates_range("s", op, &v).unwrap(),
                    oracle_range(&docs, "s", op, &v)
                );
            }
        }
    }
}