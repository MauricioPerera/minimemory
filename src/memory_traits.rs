//! # Memory Traits - Domain-Agnostic Memory System
//!
//! Este modulo define los traits core que permiten crear sistemas de memoria
//! para cualquier dominio: desarrollo de software, chatbots, ventas, soporte, etc.
//!
//! ## Arquitectura
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    NIVEL 4: UNIVERSAL                       │
//! │  Conocimiento siempre aplicable en cualquier contexto       │
//! ├─────────────────────────────────────────────────────────────┤
//! │                    NIVEL 3: DOMINIO                         │
//! │  Conocimiento especifico de un area (web, soporte, etc.)    │
//! ├─────────────────────────────────────────────────────────────┤
//! │                    NIVEL 2: CONTEXTO                        │
//! │  Conocimiento de un contexto especifico (lenguaje, tono)    │
//! ├─────────────────────────────────────────────────────────────┤
//! │                    NIVEL 1: INSTANCIA                       │
//! │  Conocimiento de una instancia particular (proyecto, user)  │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Ejemplo: Agente de Software
//!
//! ```rust,ignore
//! use minimemory::memory_traits::*;
//! use minimemory::memory_traits::presets::SoftwareDevelopment;
//!
//! let memory = GenericMemory::<SoftwareDevelopment>::new(384)?;
//! memory.set_instance("my-project", "rust", "WebBackend");
//! memory.learn("auth", embedding, "JWT implementation", "success")?;
//! ```
//!
//! ## Ejemplo: Chatbot de Telegram
//!
//! ```rust,ignore
//! use minimemory::memory_traits::*;
//! use minimemory::memory_traits::presets::Conversational;
//!
//! let memory = GenericMemory::<Conversational>::new(384)?;
//! memory.set_instance("@user123", "casual", "Support");
//! memory.learn("greeting", embedding, "User prefers informal tone", "positive")?;
//! ```

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::Result;
use crate::types::{Metadata, VectorId};
use crate::Config;
use crate::Filter;
use crate::VectorDB;

// ============================================================================
// Core Traits
// ============================================================================

/// Clasificador de dominios.
///
/// Determina a que dominio pertenece un contenido dado.
pub trait DomainClassifier: Send + Sync + Default {
    /// Nombre del tipo de dominio (para metadata).
    fn domain_type_name(&self) -> &'static str;

    /// Lista de dominios disponibles.
    fn available_domains(&self) -> Vec<&'static str>;

    /// Clasifica contenido en un dominio.
    fn classify(&self, content: &str) -> String;

    /// Determina si dos dominios estan relacionados.
    fn related(&self, domain1: &str, domain2: &str) -> bool;

    /// Score de relacion entre dominios (0.0 - 1.0).
    fn relatedness_score(&self, domain1: &str, domain2: &str) -> f32 {
        if domain1 == domain2 {
            1.0
        } else if self.related(domain1, domain2) {
            0.7
        } else {
            0.3
        }
    }
}

/// Extractor de conceptos abstractos.
///
/// Identifica patrones y principios transferibles del contenido.
pub trait ConceptExtractor: Send + Sync + Default {
    /// Extrae conceptos abstractos del contenido.
    fn extract(&self, description: &str, content: &str) -> Vec<String>;

    /// Determina si un concepto es universal (aplica a cualquier contexto).
    fn is_universal(&self, concept: &str) -> bool;

    /// Lista de conceptos universales predefinidos.
    fn universal_concepts(&self) -> Vec<&'static str>;
}

/// Evaluador de compatibilidad de contexto.
///
/// Determina que tan compatible es el conocimiento entre contextos.
pub trait ContextMatcher: Send + Sync + Default {
    /// Nombre del tipo de contexto (para metadata).
    fn context_type_name(&self) -> &'static str;

    /// Lista de contextos disponibles.
    fn available_contexts(&self) -> Vec<&'static str>;

    /// Score de compatibilidad entre contextos (0.0 - 1.0).
    fn compatibility(&self, context1: &str, context2: &str) -> f32;

    /// Agrupa contextos en familias relacionadas.
    fn context_family(&self, context: &str) -> Option<&'static str>;
}

/// Configuracion de un preset de dominio.
///
/// Combina los cuatro traits en una configuracion cohesiva.
pub trait DomainPreset: Send + Sync + 'static {
    type Domain: DomainClassifier;
    type Concepts: ConceptExtractor;
    type Context: ContextMatcher;
    type Priority: PriorityCalculator;

    /// Nombre del preset.
    fn name() -> &'static str;

    /// Descripcion del preset.
    fn description() -> &'static str;

    /// Configuracion de decay por defecto para este dominio.
    fn default_decay() -> DecayConfig {
        DecayConfig::default()
    }

    /// Pesos de prioridad por defecto para este dominio.
    fn default_weights() -> PriorityWeights {
        PriorityWeights::default()
    }

    /// Crea instancias de los componentes.
    fn create() -> (Self::Domain, Self::Concepts, Self::Context, Self::Priority) {
        (
            Self::Domain::default(),
            Self::Concepts::default(),
            Self::Context::default(),
            Self::Priority::default(),
        )
    }
}

// ============================================================================
// Transfer Level (Generico)
// ============================================================================

/// Nivel de transferibilidad del conocimiento.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
#[derive(Default)]
pub enum TransferLevel {
    /// Solo aplica a esta instancia especifica.
    #[default]
    Instance = 1,
    /// Aplica al mismo contexto (lenguaje, tono, etc.).
    Context = 2,
    /// Aplica al mismo dominio (web, soporte, etc.).
    Domain = 3,
    /// Conocimiento universal, siempre aplicable.
    Universal = 4,
}

impl TransferLevel {
    /// Convierte a string para metadata.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Instance => "instance",
            Self::Context => "context",
            Self::Domain => "domain",
            Self::Universal => "universal",
        }
    }

    /// Crea desde string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "instance" | "project" | "specific" => Some(Self::Instance),
            "context" | "stack" | "language" => Some(Self::Context),
            "domain" | "area" => Some(Self::Domain),
            "universal" | "global" | "always" => Some(Self::Universal),
            _ => None,
        }
    }

    /// Score de transferibilidad (0.25 - 1.0).
    pub fn transfer_score(&self) -> f32 {
        match self {
            Self::Instance => 0.25,
            Self::Context => 0.5,
            Self::Domain => 0.75,
            Self::Universal => 1.0,
        }
    }

    /// Determina si este nivel es transferible al nivel objetivo.
    pub fn transfers_to(&self, target: TransferLevel) -> bool {
        *self >= target
    }
}

// ============================================================================
// Priority System (Hibrido)
// ============================================================================

/// Nivel de prioridad base (manual).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
#[derive(Default)]
pub enum Priority {
    /// Prioridad minima, puede ser olvidado.
    Low = 1,
    /// Prioridad normal, comportamiento por defecto.
    #[default]
    Normal = 2,
    /// Prioridad alta, preferido en recall.
    High = 3,
    /// Prioridad critica, siempre incluido.
    Critical = 4,
}

impl Priority {
    /// Convierte a string para metadata.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Normal => "normal",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }

    /// Crea desde string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "low" | "minor" | "trivial" => Some(Self::Low),
            "normal" | "medium" | "default" => Some(Self::Normal),
            "high" | "important" | "major" => Some(Self::High),
            "critical" | "urgent" | "essential" | "security" => Some(Self::Critical),
            _ => None,
        }
    }

    /// Score base de prioridad (0.25 - 1.0).
    pub fn base_score(&self) -> f32 {
        match self {
            Self::Low => 0.25,
            Self::Normal => 0.5,
            Self::High => 0.75,
            Self::Critical => 1.0,
        }
    }
}

/// Estadisticas de uso de una memoria.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UsageStats {
    /// Numero de veces que ha sido accedida.
    pub access_count: u32,
    /// Timestamp del ultimo acceso.
    pub last_accessed: i64,
    /// Timestamp de creacion.
    pub created_at: i64,
    /// Veces que fue util (feedback positivo).
    pub useful_count: u32,
}

impl UsageStats {
    /// Crea nuevas estadisticas con timestamp actual.
    pub fn new() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        Self {
            access_count: 0,
            last_accessed: now,
            created_at: now,
            useful_count: 0,
        }
    }

    /// Registra un acceso.
    pub fn record_access(&mut self) {
        self.access_count += 1;
        self.last_accessed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
    }

    /// Registra que fue util.
    pub fn record_useful(&mut self) {
        self.useful_count += 1;
    }

    /// Calcula score por frecuencia de uso (0.0 - 1.0).
    /// Usa logaritmo para evitar que memorias muy usadas dominen.
    pub fn frequency_score(&self) -> f32 {
        if self.access_count == 0 {
            0.0
        } else {
            // log2(access + 1) / 10, capped at 1.0
            ((self.access_count as f32 + 1.0).log2() / 10.0).min(1.0)
        }
    }

    /// Calcula score de utilidad (0.0 - 1.0).
    pub fn usefulness_score(&self) -> f32 {
        if self.access_count == 0 {
            0.5 // Neutral si nunca fue accedida
        } else {
            self.useful_count as f32 / self.access_count as f32
        }
    }

    /// Edad en segundos.
    pub fn age_seconds(&self) -> i64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        now - self.created_at
    }

    /// Segundos desde ultimo acceso.
    pub fn staleness_seconds(&self) -> i64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        now - self.last_accessed
    }
}

/// Configuracion de decay temporal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecayConfig {
    /// Activar decay temporal.
    pub enabled: bool,
    /// Vida media en segundos (tiempo para perder 50% de prioridad).
    pub half_life_seconds: i64,
    /// Piso minimo de decay (nunca baja de este valor).
    pub min_decay: f32,
    /// Excepciones: niveles que no decaen.
    pub immune_priorities: Vec<Priority>,
}

impl Default for DecayConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            half_life_seconds: 30 * 24 * 60 * 60, // 30 dias
            min_decay: 0.1,
            immune_priorities: vec![Priority::Critical],
        }
    }
}

impl DecayConfig {
    /// Sin decay (todo persiste igual).
    pub fn no_decay() -> Self {
        Self {
            enabled: false,
            ..Default::default()
        }
    }

    /// Decay rapido (1 semana).
    pub fn fast() -> Self {
        Self {
            enabled: true,
            half_life_seconds: 7 * 24 * 60 * 60,
            min_decay: 0.2,
            immune_priorities: vec![Priority::Critical, Priority::High],
        }
    }

    /// Decay lento (90 dias).
    pub fn slow() -> Self {
        Self {
            enabled: true,
            half_life_seconds: 90 * 24 * 60 * 60,
            min_decay: 0.05,
            immune_priorities: vec![Priority::Critical],
        }
    }

    /// Calcula factor de decay basado en edad.
    pub fn calculate_decay(&self, age_seconds: i64, priority: Priority) -> f32 {
        if !self.enabled || self.immune_priorities.contains(&priority) {
            return 1.0;
        }

        // Exponential decay: 0.5^(age / half_life)
        let decay = 0.5_f32.powf(age_seconds as f32 / self.half_life_seconds as f32);
        decay.max(self.min_decay)
    }
}

/// Calculador de prioridad automatica basado en contenido.
pub trait PriorityCalculator: Send + Sync + Default {
    /// Calcula prioridad automatica basada en contenido.
    fn calculate(&self, description: &str, content: &str, outcome: &str) -> Priority;

    /// Keywords que indican prioridad critica.
    fn critical_keywords(&self) -> Vec<&'static str>;

    /// Keywords que indican prioridad alta.
    fn high_keywords(&self) -> Vec<&'static str>;

    /// Keywords que indican prioridad baja.
    fn low_keywords(&self) -> Vec<&'static str>;
}

/// Pesos para combinar factores de prioridad.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorityWeights {
    /// Peso de prioridad manual/automatica.
    pub base_priority: f32,
    /// Peso de frecuencia de uso.
    pub frequency: f32,
    /// Peso de utilidad (feedback).
    pub usefulness: f32,
    /// Peso de recencia (inverso de staleness).
    pub recency: f32,
}

impl Default for PriorityWeights {
    fn default() -> Self {
        Self {
            base_priority: 0.4,
            frequency: 0.2,
            usefulness: 0.25,
            recency: 0.15,
        }
    }
}

impl PriorityWeights {
    /// Prioriza la prioridad manual.
    pub fn manual_focused() -> Self {
        Self {
            base_priority: 0.6,
            frequency: 0.15,
            usefulness: 0.15,
            recency: 0.1,
        }
    }

    /// Prioriza el uso frecuente.
    pub fn usage_focused() -> Self {
        Self {
            base_priority: 0.2,
            frequency: 0.4,
            usefulness: 0.25,
            recency: 0.15,
        }
    }

    /// Prioriza la recencia.
    pub fn recency_focused() -> Self {
        Self {
            base_priority: 0.25,
            frequency: 0.15,
            usefulness: 0.2,
            recency: 0.4,
        }
    }

    /// Calcula score combinado de prioridad.
    pub fn calculate_score(&self, base: f32, frequency: f32, usefulness: f32, recency: f32) -> f32 {
        (self.base_priority * base
            + self.frequency * frequency
            + self.usefulness * usefulness
            + self.recency * recency)
            .clamp(0.0, 1.0)
    }
}

/// Score de recencia basado en staleness.
pub fn recency_score(staleness_seconds: i64) -> f32 {
    // Score exponencial: 1.0 si es reciente, decae con el tiempo
    // 1 hora -> 0.95, 1 dia -> 0.75, 1 semana -> 0.5, 1 mes -> 0.25
    let hours = staleness_seconds as f32 / 3600.0;
    (-hours / 168.0).exp() // 168 horas = 1 semana como punto medio
}

// ============================================================================
// Instance Context (Generico)
// ============================================================================

/// Contexto de la instancia actual.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstanceContext {
    /// Identificador de la instancia (proyecto, usuario, sesion).
    pub instance_id: String,
    /// Contexto actual (lenguaje, tono, tipo).
    pub context: String,
    /// Dominio actual.
    pub domain: String,
    /// Metadata adicional especifica del dominio.
    pub extra: HashMap<String, String>,
}

impl InstanceContext {
    /// Crea un nuevo contexto de instancia.
    pub fn new(instance_id: impl Into<String>) -> Self {
        Self {
            instance_id: instance_id.into(),
            ..Default::default()
        }
    }

    /// Establece el contexto.
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = context.into();
        self
    }

    /// Establece el dominio.
    pub fn with_domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = domain.into();
        self
    }

    /// Agrega metadata extra.
    pub fn with_extra(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.extra.insert(key.into(), value.into());
        self
    }
}

// ============================================================================
// Memory Recall (Generico)
// ============================================================================

/// Resultado de recall con informacion de transferibilidad y prioridad.
#[derive(Debug, Clone)]
pub struct GenericRecall {
    /// ID del recuerdo.
    pub id: VectorId,
    /// Score de relevancia semantica (0.0 - 1.0).
    pub relevance: f32,
    /// Nivel de transferibilidad.
    pub transfer_level: TransferLevel,
    /// Prioridad base del recuerdo.
    pub priority: Priority,
    /// Score de prioridad hibrido (incluye uso, recencia, decay).
    pub priority_score: f32,
    /// Score combinado final.
    pub combined_score: f32,
    /// Conceptos abstractos asociados.
    pub concepts: Vec<String>,
    /// Estadisticas de uso.
    pub usage: UsageStats,
    /// Metadata del recuerdo.
    pub metadata: Metadata,
}

// ============================================================================
// Generic Memory System
// ============================================================================

/// Sistema de memoria generico basado en traits.
pub struct GenericMemory<P: DomainPreset> {
    /// Base de datos vectorial.
    db: VectorDB,
    /// Clasificador de dominios.
    domain_classifier: P::Domain,
    /// Extractor de conceptos.
    concept_extractor: P::Concepts,
    /// Evaluador de contexto.
    context_matcher: P::Context,
    /// Calculador de prioridad.
    priority_calculator: P::Priority,
    /// Contexto actual.
    current_context: RwLock<Option<InstanceContext>>,
    /// Estadisticas de uso por ID.
    usage_stats: RwLock<HashMap<String, UsageStats>>,
    /// Configuracion de decay.
    decay_config: DecayConfig,
    /// Pesos de prioridad.
    priority_weights: PriorityWeights,
    /// Peso de relevancia vs transferibilidad vs prioridad.
    relevance_weight: f32,
    /// Peso de transferibilidad.
    transfer_weight: f32,
    /// Peso de prioridad.
    priority_weight: f32,
    /// Umbral minimo de transferibilidad.
    transfer_threshold: f32,
}

impl<P: DomainPreset> GenericMemory<P> {
    /// Crea una nueva memoria generica.
    pub fn new(dimensions: usize) -> Result<Self> {
        let config = Config::new(dimensions);
        let db = VectorDB::with_fulltext(config, vec!["content".into(), "description".into()])?;
        let (domain_classifier, concept_extractor, context_matcher, priority_calculator) =
            P::create();

        Ok(Self {
            db,
            domain_classifier,
            concept_extractor,
            context_matcher,
            priority_calculator,
            current_context: RwLock::new(None),
            usage_stats: RwLock::new(HashMap::new()),
            decay_config: P::default_decay(),
            priority_weights: P::default_weights(),
            relevance_weight: 0.4,
            transfer_weight: 0.3,
            priority_weight: 0.3,
            transfer_threshold: 0.3,
        })
    }

    /// Crea con configuracion personalizada.
    pub fn with_config(config: Config) -> Result<Self> {
        let db = VectorDB::with_fulltext(config, vec!["content".into(), "description".into()])?;
        let (domain_classifier, concept_extractor, context_matcher, priority_calculator) =
            P::create();

        Ok(Self {
            db,
            domain_classifier,
            concept_extractor,
            context_matcher,
            priority_calculator,
            current_context: RwLock::new(None),
            usage_stats: RwLock::new(HashMap::new()),
            decay_config: P::default_decay(),
            priority_weights: P::default_weights(),
            relevance_weight: 0.4,
            transfer_weight: 0.3,
            priority_weight: 0.3,
            transfer_threshold: 0.3,
        })
    }

    /// Establece la configuracion de decay.
    pub fn set_decay_config(&mut self, config: DecayConfig) {
        self.decay_config = config;
    }

    /// Establece los pesos de prioridad.
    pub fn set_priority_weights(&mut self, weights: PriorityWeights) {
        self.priority_weights = weights;
    }

    /// Establece los pesos del score final.
    /// Los tres pesos deben sumar 1.0.
    pub fn set_score_weights(&mut self, relevance: f32, transfer: f32, priority: f32) {
        let total = relevance + transfer + priority;
        self.relevance_weight = relevance / total;
        self.transfer_weight = transfer / total;
        self.priority_weight = priority / total;
    }

    /// Establece el umbral de transferibilidad.
    pub fn set_transfer_threshold(&mut self, threshold: f32) {
        self.transfer_threshold = threshold.clamp(0.0, 1.0);
    }

    /// Establece el contexto actual de la instancia.
    pub fn set_context(&self, context: InstanceContext) {
        *self.current_context.write() = Some(context);
    }

    /// Atajo para establecer contexto con parametros comunes.
    pub fn set_instance(
        &self,
        instance_id: impl Into<String>,
        context: impl Into<String>,
        domain: impl Into<String>,
    ) {
        self.set_context(
            InstanceContext::new(instance_id)
                .with_context(context)
                .with_domain(domain),
        );
    }

    /// Obtiene el contexto actual.
    pub fn current_context(&self) -> Option<InstanceContext> {
        self.current_context.read().clone()
    }

    /// Aprende nuevo conocimiento con prioridad automatica.
    pub fn learn(
        &self,
        id: &str,
        embedding: &[f32],
        content: &str,
        description: &str,
        outcome: &str,
    ) -> Result<VectorId> {
        // Calcular prioridad automatica
        let priority = self
            .priority_calculator
            .calculate(description, content, outcome);
        self.learn_with_priority(id, embedding, content, description, outcome, priority)
    }

    /// Aprende nuevo conocimiento con prioridad manual.
    pub fn learn_with_priority(
        &self,
        id: &str,
        embedding: &[f32],
        content: &str,
        description: &str,
        outcome: &str,
        priority: Priority,
    ) -> Result<VectorId> {
        let ctx = self.current_context.read().clone();

        // Extraer conceptos
        let concepts = self.concept_extractor.extract(description, content);

        // Inferir nivel de transferencia
        let transfer_level = self.infer_transfer_level(&concepts, content);

        // Crear estadisticas de uso
        let usage = UsageStats::new();
        self.usage_stats.write().insert(id.to_string(), usage);

        // Construir metadata
        let mut meta = Metadata::new();
        meta.insert("content", content);
        meta.insert("description", description);
        meta.insert("outcome", outcome);
        meta.insert("transfer_level", transfer_level.as_str());
        meta.insert("priority", priority.as_str());
        meta.insert("concepts", concepts.join(","));
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        meta.insert("timestamp", timestamp);

        if let Some(ref ctx) = ctx {
            meta.insert("instance_id", ctx.instance_id.as_str());
            meta.insert("context", ctx.context.as_str());
            meta.insert("domain", ctx.domain.as_str());

            for (k, v) in &ctx.extra {
                meta.insert(k.as_str(), v.as_str());
            }
        }

        // Clasificar dominio automaticamente si no esta establecido
        if ctx.as_ref().is_none_or(|c| c.domain.is_empty()) {
            let domain = self.domain_classifier.classify(content);
            meta.insert("domain", domain.as_str());
        }

        self.db.insert(id, embedding, Some(meta))?;
        Ok(id.to_string())
    }

    /// Registra feedback positivo (la memoria fue util).
    pub fn mark_useful(&self, id: &str) {
        if let Some(stats) = self.usage_stats.write().get_mut(id) {
            stats.record_useful();
        }
    }

    /// Actualiza la prioridad de una memoria existente.
    pub fn update_priority(&self, id: &str, priority: Priority) -> Result<()> {
        // Get current vector and update metadata
        if let Some((vec_opt, meta_opt)) = self.db.get(id)? {
            if let (Some(vec), Some(mut meta)) = (vec_opt, meta_opt) {
                meta.insert("priority", priority.as_str());
                // Re-insert with updated metadata
                self.db.insert(id, &vec, Some(meta))?;
            }
        }
        Ok(())
    }

    /// Inferir nivel de transferencia basado en conceptos y contenido.
    fn infer_transfer_level(&self, concepts: &[String], content: &str) -> TransferLevel {
        // Si tiene conceptos universales, es universal
        let universal_count = concepts
            .iter()
            .filter(|c| self.concept_extractor.is_universal(c))
            .count();

        if universal_count >= 2 {
            return TransferLevel::Universal;
        }

        // Analizar contenido para determinar especificidad
        let content_lower = content.to_lowercase();

        // Patrones que indican conocimiento especifico de instancia
        let instance_patterns = [
            "this project",
            "este proyecto",
            "specific to",
            "only here",
            "custom",
            "our",
            "nuestra",
        ];

        if instance_patterns.iter().any(|p| content_lower.contains(p)) {
            return TransferLevel::Instance;
        }

        // Por defecto, nivel de contexto
        if universal_count >= 1 {
            TransferLevel::Domain
        } else {
            TransferLevel::Context
        }
    }

    /// Recall con filtrado por transferibilidad y prioridad hibrida.
    pub fn recall(&self, query_embedding: &[f32], k: usize) -> Result<Vec<GenericRecall>> {
        let ctx = self.current_context.read().clone();

        // Buscar en la base de datos
        let results = self.db.search(query_embedding, k * 3)?;

        let mut recalls: Vec<GenericRecall> = results
            .into_iter()
            .filter_map(|r| {
                let meta = r.metadata?;
                let id = r.id.clone();

                // Obtener nivel de transferencia
                let transfer_level = meta
                    .get("transfer_level")
                    .and_then(|v| v.as_str())
                    .and_then(TransferLevel::from_str)
                    .unwrap_or(TransferLevel::Instance);

                // Calcular score de transferibilidad
                let transfer_score = self.calculate_transfer_score(&ctx, &meta, transfer_level);

                if transfer_score < self.transfer_threshold {
                    return None;
                }

                // Obtener prioridad base
                let priority = meta
                    .get("priority")
                    .and_then(|v| v.as_str())
                    .and_then(Priority::from_str)
                    .unwrap_or(Priority::Normal);

                // Obtener o crear estadisticas de uso
                let usage = self
                    .usage_stats
                    .read()
                    .get(&id)
                    .cloned()
                    .unwrap_or_default();

                // Calcular score de prioridad hibrido
                let priority_score = self.calculate_priority_score(&usage, priority);

                // Score combinado final: relevancia + transferibilidad + prioridad
                let relevance = 1.0 - r.distance; // Convertir distancia a similitud
                let combined_score = relevance * self.relevance_weight
                    + transfer_score * self.transfer_weight
                    + priority_score * self.priority_weight;

                // Extraer conceptos
                let concepts = meta
                    .get("concepts")
                    .and_then(|v| v.as_str())
                    .map(|s: &str| s.split(',').map(String::from).collect())
                    .unwrap_or_default();

                Some(GenericRecall {
                    id,
                    relevance,
                    transfer_level,
                    priority,
                    priority_score,
                    combined_score,
                    concepts,
                    usage,
                    metadata: meta,
                })
            })
            .collect();

        // Registrar acceso para todas las memorias retornadas
        {
            let mut stats = self.usage_stats.write();
            for recall in &recalls {
                if let Some(s) = stats.get_mut(&recall.id) {
                    s.record_access();
                }
            }
        }

        // Ordenar por score combinado
        recalls.sort_by(|a, b| b.combined_score.partial_cmp(&a.combined_score).unwrap());
        recalls.truncate(k);

        Ok(recalls)
    }

    /// Calcula el score de prioridad hibrido.
    fn calculate_priority_score(&self, usage: &UsageStats, priority: Priority) -> f32 {
        // Score base de prioridad
        let base = priority.base_score();

        // Score de frecuencia
        let frequency = usage.frequency_score();

        // Score de utilidad
        let usefulness = usage.usefulness_score();

        // Score de recencia
        let recency = recency_score(usage.staleness_seconds());

        // Combinar con pesos
        let raw_score = self
            .priority_weights
            .calculate_score(base, frequency, usefulness, recency);

        // Aplicar decay temporal
        let age = usage.age_seconds();
        let decay = self.decay_config.calculate_decay(age, priority);

        raw_score * decay
    }

    /// Calcula el score de transferibilidad para un recuerdo.
    fn calculate_transfer_score(
        &self,
        current_ctx: &Option<InstanceContext>,
        meta: &Metadata,
        level: TransferLevel,
    ) -> f32 {
        // Universal siempre tiene score maximo
        if level == TransferLevel::Universal {
            return 1.0;
        }

        let Some(ctx) = current_ctx else {
            return level.transfer_score();
        };

        // Obtener valores del recuerdo
        let stored_instance = meta
            .get("instance_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let stored_context = meta.get("context").and_then(|v| v.as_str()).unwrap_or("");
        let stored_domain = meta.get("domain").and_then(|v| v.as_str()).unwrap_or("");

        // Calcular compatibilidad
        let instance_match = if ctx.instance_id == stored_instance {
            1.0
        } else {
            0.0
        };
        let context_compat = self
            .context_matcher
            .compatibility(&ctx.context, stored_context);
        let domain_compat = self
            .domain_classifier
            .relatedness_score(&ctx.domain, stored_domain);

        // Ponderar segun nivel
        match level {
            TransferLevel::Instance => {
                instance_match * 0.6 + context_compat * 0.2 + domain_compat * 0.2
            }
            TransferLevel::Context => context_compat * 0.5 + domain_compat * 0.3 + 0.2,
            TransferLevel::Domain => domain_compat * 0.6 + 0.4,
            TransferLevel::Universal => 1.0,
        }
    }

    /// Helper para crear GenericRecall con todos los campos.
    fn make_recall(&self, id: String, distance: f32, meta: Metadata) -> GenericRecall {
        let transfer_level = meta
            .get("transfer_level")
            .and_then(|v| v.as_str())
            .and_then(TransferLevel::from_str)
            .unwrap_or(TransferLevel::Instance);

        let priority = meta
            .get("priority")
            .and_then(|v| v.as_str())
            .and_then(Priority::from_str)
            .unwrap_or(Priority::Normal);

        let usage = self
            .usage_stats
            .read()
            .get(&id)
            .cloned()
            .unwrap_or_default();
        let priority_score = self.calculate_priority_score(&usage, priority);
        let relevance = 1.0 - distance;

        let concepts = meta
            .get("concepts")
            .and_then(|v| v.as_str())
            .map(|s: &str| s.split(',').map(String::from).collect())
            .unwrap_or_default();

        GenericRecall {
            id,
            relevance,
            transfer_level,
            priority,
            priority_score,
            combined_score: relevance, // Simple score for filtered queries
            concepts,
            usage,
            metadata: meta,
        }
    }

    /// Recall solo de conocimiento universal.
    pub fn recall_universal(
        &self,
        query_embedding: &[f32],
        k: usize,
    ) -> Result<Vec<GenericRecall>> {
        let results = self.db.search_with_filter(
            query_embedding,
            k,
            Filter::eq("transfer_level", "universal"),
        )?;

        Ok(results
            .into_iter()
            .filter_map(|r| {
                let meta = r.metadata?;
                Some(self.make_recall(r.id, r.distance, meta))
            })
            .collect())
    }

    /// Recall en el mismo dominio.
    pub fn recall_same_domain(
        &self,
        query_embedding: &[f32],
        k: usize,
    ) -> Result<Vec<GenericRecall>> {
        let ctx = self.current_context.read().clone();
        let domain = ctx.map(|c| c.domain).unwrap_or_default();

        if domain.is_empty() {
            return self.recall(query_embedding, k);
        }

        let results = self.db.search_with_filter(
            query_embedding,
            k,
            Filter::eq("domain", domain.as_str()),
        )?;

        Ok(results
            .into_iter()
            .filter_map(|r| {
                let meta = r.metadata?;
                Some(self.make_recall(r.id, r.distance, meta))
            })
            .collect())
    }

    /// Recall en el mismo contexto.
    pub fn recall_same_context(
        &self,
        query_embedding: &[f32],
        k: usize,
    ) -> Result<Vec<GenericRecall>> {
        let ctx = self.current_context.read().clone();
        let context = ctx.map(|c| c.context).unwrap_or_default();

        if context.is_empty() {
            return self.recall(query_embedding, k);
        }

        let results = self.db.search_with_filter(
            query_embedding,
            k,
            Filter::eq("context", context.as_str()),
        )?;

        Ok(results
            .into_iter()
            .filter_map(|r| {
                let meta = r.metadata?;
                Some(self.make_recall(r.id, r.distance, meta))
            })
            .collect())
    }

    /// Recall solo de prioridad critica.
    pub fn recall_critical(&self, query_embedding: &[f32], k: usize) -> Result<Vec<GenericRecall>> {
        let results =
            self.db
                .search_with_filter(query_embedding, k, Filter::eq("priority", "critical"))?;

        Ok(results
            .into_iter()
            .filter_map(|r| {
                let meta = r.metadata?;
                Some(self.make_recall(r.id, r.distance, meta))
            })
            .collect())
    }

    /// Recall de prioridad alta o critica.
    pub fn recall_high_priority(
        &self,
        query_embedding: &[f32],
        k: usize,
    ) -> Result<Vec<GenericRecall>> {
        let results = self.db.search_with_filter(
            query_embedding,
            k * 2,
            Filter::or(vec![
                Filter::eq("priority", "critical"),
                Filter::eq("priority", "high"),
            ]),
        )?;

        let mut recalls: Vec<GenericRecall> = results
            .into_iter()
            .filter_map(|r| {
                let meta = r.metadata?;
                Some(self.make_recall(r.id, r.distance, meta))
            })
            .collect();

        // Sort by priority score
        recalls.sort_by(|a, b| b.priority_score.partial_cmp(&a.priority_score).unwrap());
        recalls.truncate(k);
        Ok(recalls)
    }

    /// Busqueda por keywords.
    pub fn recall_by_keywords(&self, keywords: &str, k: usize) -> Result<Vec<GenericRecall>> {
        let results = self.db.keyword_search(keywords, k)?;

        Ok(results
            .into_iter()
            .filter_map(|r| {
                let meta = r.metadata?;
                let transfer_level = meta
                    .get("transfer_level")
                    .and_then(|v| v.as_str())
                    .and_then(TransferLevel::from_str)
                    .unwrap_or(TransferLevel::Instance);
                let priority = meta
                    .get("priority")
                    .and_then(|v| v.as_str())
                    .and_then(Priority::from_str)
                    .unwrap_or(Priority::Normal);
                let id = r.id.clone();
                let usage = self
                    .usage_stats
                    .read()
                    .get(&id)
                    .cloned()
                    .unwrap_or_default();
                let priority_score = self.calculate_priority_score(&usage, priority);

                let concepts = meta
                    .get("concepts")
                    .and_then(|v| v.as_str())
                    .map(|s: &str| s.split(',').map(String::from).collect())
                    .unwrap_or_default();

                Some(GenericRecall {
                    id,
                    relevance: r.score,
                    transfer_level,
                    priority,
                    priority_score,
                    combined_score: r.score,
                    concepts,
                    usage,
                    metadata: meta,
                })
            })
            .collect())
    }

    /// Estadisticas de la memoria.
    pub fn stats(&self) -> MemoryStats {
        let usage_stats = self.usage_stats.read();
        let total_accesses: u32 = usage_stats.values().map(|u| u.access_count).sum();
        let avg_usefulness = if usage_stats.is_empty() {
            0.0
        } else {
            usage_stats
                .values()
                .map(|u| u.usefulness_score())
                .sum::<f32>()
                / usage_stats.len() as f32
        };

        MemoryStats {
            total_memories: self.db.len(),
            preset_name: P::name().to_string(),
            has_context: self.current_context.read().is_some(),
            total_accesses,
            avg_usefulness,
        }
    }
}

/// Estadisticas de la memoria.
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub total_memories: usize,
    pub preset_name: String,
    pub has_context: bool,
    pub total_accesses: u32,
    pub avg_usefulness: f32,
}

// ============================================================================
// Presets Module
// ============================================================================

pub mod presets {
    //! Presets predefinidos para diferentes dominios.

    use super::*;

    // ------------------------------------------------------------------------
    // Software Development Preset
    // ------------------------------------------------------------------------

    /// Clasificador de dominios para desarrollo de software.
    #[derive(Debug, Default)]
    pub struct SoftwareDomainClassifier;

    impl DomainClassifier for SoftwareDomainClassifier {
        fn domain_type_name(&self) -> &'static str {
            "software_domain"
        }

        fn available_domains(&self) -> Vec<&'static str> {
            vec![
                "web_backend",
                "web_frontend",
                "cli",
                "data_science",
                "systems",
                "mobile",
                "devops",
                "security",
                "database",
                "gamedev",
                "embedded",
                "general",
            ]
        }

        fn classify(&self, content: &str) -> String {
            let lower = content.to_lowercase();

            if lower.contains("api") || lower.contains("endpoint") || lower.contains("rest") {
                "web_backend".into()
            } else if lower.contains("react") || lower.contains("vue") || lower.contains("css") {
                "web_frontend".into()
            } else if lower.contains("cli")
                || lower.contains("terminal")
                || lower.contains("command")
            {
                "cli".into()
            } else if lower.contains("pandas") || lower.contains("numpy") || lower.contains("ml") {
                "data_science".into()
            } else if lower.contains("docker")
                || lower.contains("kubernetes")
                || lower.contains("ci/cd")
            {
                "devops".into()
            } else if lower.contains("auth")
                || lower.contains("security")
                || lower.contains("encrypt")
            {
                "security".into()
            } else if lower.contains("sql") || lower.contains("database") || lower.contains("query")
            {
                "database".into()
            } else if lower.contains("android") || lower.contains("ios") || lower.contains("mobile")
            {
                "mobile".into()
            } else if lower.contains("kernel")
                || lower.contains("memory")
                || lower.contains("syscall")
            {
                "systems".into()
            } else if lower.contains("game") || lower.contains("render") || lower.contains("sprite")
            {
                "gamedev".into()
            } else if lower.contains("embedded")
                || lower.contains("mcu")
                || lower.contains("firmware")
            {
                "embedded".into()
            } else {
                "general".into()
            }
        }

        fn related(&self, domain1: &str, domain2: &str) -> bool {
            let web = ["web_backend", "web_frontend"];
            let low_level = ["systems", "embedded"];
            let data = ["data_science", "database"];

            (web.contains(&domain1) && web.contains(&domain2))
                || (low_level.contains(&domain1) && low_level.contains(&domain2))
                || (data.contains(&domain1) && data.contains(&domain2))
        }
    }

    /// Extractor de conceptos para desarrollo de software.
    #[derive(Debug, Default)]
    pub struct SoftwareConceptExtractor;

    impl ConceptExtractor for SoftwareConceptExtractor {
        fn extract(&self, description: &str, content: &str) -> Vec<String> {
            let text = format!("{} {}", description, content).to_lowercase();
            let mut concepts = Vec::new();

            let patterns = [
                (
                    "error handling",
                    &["error", "exception", "try", "catch", "result"][..],
                ),
                ("validation", &["valid", "check", "verify", "sanitize"]),
                ("caching", &["cache", "memoize", "ttl"]),
                ("async", &["async", "await", "future", "promise"]),
                ("testing", &["test", "mock", "assert", "spec"]),
                ("logging", &["log", "trace", "debug", "info"]),
                ("authentication", &["auth", "login", "jwt", "token"]),
                ("pagination", &["page", "limit", "offset", "cursor"]),
                ("rate limiting", &["rate", "throttle", "limit"]),
                ("middleware", &["middleware", "interceptor", "filter"]),
            ];

            for (concept, keywords) in patterns {
                if keywords.iter().any(|k| text.contains(k)) {
                    concepts.push(concept.to_string());
                }
            }

            concepts
        }

        fn is_universal(&self, concept: &str) -> bool {
            self.universal_concepts().contains(&concept)
        }

        fn universal_concepts(&self) -> Vec<&'static str> {
            vec![
                "error handling",
                "validation",
                "caching",
                "logging",
                "testing",
            ]
        }
    }

    /// Evaluador de contexto para lenguajes de programacion.
    #[derive(Debug, Default)]
    pub struct ProgrammingLanguageMatcher;

    impl ContextMatcher for ProgrammingLanguageMatcher {
        fn context_type_name(&self) -> &'static str {
            "programming_language"
        }

        fn available_contexts(&self) -> Vec<&'static str> {
            vec![
                "rust",
                "python",
                "javascript",
                "typescript",
                "go",
                "java",
                "c",
                "cpp",
                "csharp",
                "ruby",
                "php",
                "swift",
                "kotlin",
            ]
        }

        fn compatibility(&self, ctx1: &str, ctx2: &str) -> f32 {
            if ctx1 == ctx2 {
                return 1.0;
            }

            let family1 = self.context_family(ctx1);
            let family2 = self.context_family(ctx2);

            if family1.is_some() && family1 == family2 {
                0.8
            } else {
                0.3
            }
        }

        fn context_family(&self, context: &str) -> Option<&'static str> {
            match context.to_lowercase().as_str() {
                "javascript" | "typescript" => Some("js_family"),
                "c" | "cpp" | "rust" => Some("systems"),
                "java" | "kotlin" | "scala" => Some("jvm"),
                "python" | "ruby" => Some("dynamic"),
                "swift" | "objective-c" => Some("apple"),
                _ => None,
            }
        }
    }

    /// Calculador de prioridad para desarrollo de software.
    #[derive(Debug, Default)]
    pub struct SoftwarePriorityCalculator;

    impl PriorityCalculator for SoftwarePriorityCalculator {
        fn calculate(&self, description: &str, content: &str, outcome: &str) -> Priority {
            let text = format!("{} {} {}", description, content, outcome).to_lowercase();

            // Critical: security, production issues, data loss
            if self.critical_keywords().iter().any(|k| text.contains(k)) {
                return Priority::Critical;
            }

            // High: bugs, errors, performance
            if self.high_keywords().iter().any(|k| text.contains(k)) {
                return Priority::High;
            }

            // Low: style, comments, documentation
            if self.low_keywords().iter().any(|k| text.contains(k)) {
                return Priority::Low;
            }

            Priority::Normal
        }

        fn critical_keywords(&self) -> Vec<&'static str> {
            vec![
                "security",
                "vulnerability",
                "cve",
                "injection",
                "xss",
                "csrf",
                "production",
                "outage",
                "data loss",
                "corruption",
                "breach",
                "critical",
                "urgent",
                "emergency",
                "hotfix",
            ]
        }

        fn high_keywords(&self) -> Vec<&'static str> {
            vec![
                "bug",
                "error",
                "exception",
                "crash",
                "failure",
                "broken",
                "performance",
                "slow",
                "memory leak",
                "timeout",
                "important",
                "priority",
                "blocking",
            ]
        }

        fn low_keywords(&self) -> Vec<&'static str> {
            vec![
                "style",
                "formatting",
                "comment",
                "typo",
                "rename",
                "refactor",
                "cleanup",
                "todo",
                "nice to have",
            ]
        }
    }

    /// Preset para desarrollo de software.
    pub struct SoftwareDevelopment;

    impl DomainPreset for SoftwareDevelopment {
        type Domain = SoftwareDomainClassifier;
        type Concepts = SoftwareConceptExtractor;
        type Context = ProgrammingLanguageMatcher;
        type Priority = SoftwarePriorityCalculator;

        fn name() -> &'static str {
            "Software Development"
        }

        fn description() -> &'static str {
            "Memory system for software development agents with code-aware transfer"
        }

        fn default_decay() -> DecayConfig {
            // Code knowledge decays slowly (90 days)
            DecayConfig::slow()
        }

        fn default_weights() -> PriorityWeights {
            // Prioritize usefulness for code
            PriorityWeights {
                base_priority: 0.3,
                frequency: 0.25,
                usefulness: 0.35,
                recency: 0.1,
            }
        }
    }

    // ------------------------------------------------------------------------
    // Conversational Preset (Chatbots)
    // ------------------------------------------------------------------------

    /// Clasificador de dominios conversacionales.
    #[derive(Debug, Default)]
    pub struct ConversationalDomainClassifier;

    impl DomainClassifier for ConversationalDomainClassifier {
        fn domain_type_name(&self) -> &'static str {
            "conversation_domain"
        }

        fn available_domains(&self) -> Vec<&'static str> {
            vec![
                "support",
                "sales",
                "entertainment",
                "education",
                "casual",
                "professional",
            ]
        }

        fn classify(&self, content: &str) -> String {
            let lower = content.to_lowercase();

            if lower.contains("help") || lower.contains("problem") || lower.contains("issue") {
                "support".into()
            } else if lower.contains("buy")
                || lower.contains("price")
                || lower.contains("offer")
                || lower.contains("cost")
            {
                "sales".into()
            } else if lower.contains("joke") || lower.contains("fun") || lower.contains("play") {
                "entertainment".into()
            } else if lower.contains("learn") || lower.contains("explain") || lower.contains("how")
            {
                "education".into()
            } else if lower.contains("meeting")
                || lower.contains("report")
                || lower.contains("deadline")
            {
                "professional".into()
            } else {
                "casual".into()
            }
        }

        fn related(&self, domain1: &str, domain2: &str) -> bool {
            let formal = ["support", "sales", "professional"];
            let informal = ["entertainment", "casual"];
            let learning = ["education", "support"];

            (formal.contains(&domain1) && formal.contains(&domain2))
                || (informal.contains(&domain1) && informal.contains(&domain2))
                || (learning.contains(&domain1) && learning.contains(&domain2))
        }
    }

    /// Extractor de conceptos conversacionales.
    #[derive(Debug, Default)]
    pub struct ConversationalConceptExtractor;

    impl ConceptExtractor for ConversationalConceptExtractor {
        fn extract(&self, description: &str, content: &str) -> Vec<String> {
            let text = format!("{} {}", description, content).to_lowercase();
            let mut concepts = Vec::new();

            let patterns = [
                (
                    "greeting",
                    &["hello", "hi", "hey", "good morning", "hola"][..],
                ),
                ("farewell", &["bye", "goodbye", "see you", "adios"]),
                ("gratitude", &["thank", "thanks", "gracias", "appreciate"]),
                ("apology", &["sorry", "apologize", "excuse", "disculpa"]),
                ("empathy", &["understand", "feel", "sorry to hear"]),
                (
                    "clarification",
                    &["mean", "clarify", "explain", "what do you"],
                ),
                ("confirmation", &["yes", "correct", "right", "exactly"]),
                ("negation", &["no", "not", "don't", "can't"]),
                ("urgency", &["urgent", "asap", "immediately", "now"]),
                ("frustration", &["angry", "upset", "frustrated", "annoyed"]),
            ];

            for (concept, keywords) in patterns {
                if keywords.iter().any(|k| text.contains(k)) {
                    concepts.push(concept.to_string());
                }
            }

            concepts
        }

        fn is_universal(&self, concept: &str) -> bool {
            self.universal_concepts().contains(&concept)
        }

        fn universal_concepts(&self) -> Vec<&'static str> {
            vec![
                "greeting",
                "farewell",
                "gratitude",
                "empathy",
                "clarification",
            ]
        }
    }

    /// Evaluador de tono conversacional.
    #[derive(Debug, Default)]
    pub struct ConversationalToneMatcher;

    impl ContextMatcher for ConversationalToneMatcher {
        fn context_type_name(&self) -> &'static str {
            "conversational_tone"
        }

        fn available_contexts(&self) -> Vec<&'static str> {
            vec![
                "formal",
                "informal",
                "friendly",
                "professional",
                "technical",
                "casual",
                "empathetic",
            ]
        }

        fn compatibility(&self, ctx1: &str, ctx2: &str) -> f32 {
            if ctx1 == ctx2 {
                return 1.0;
            }

            let family1 = self.context_family(ctx1);
            let family2 = self.context_family(ctx2);

            if family1.is_some() && family1 == family2 {
                0.8
            } else {
                0.4
            }
        }

        fn context_family(&self, context: &str) -> Option<&'static str> {
            match context.to_lowercase().as_str() {
                "formal" | "professional" | "technical" => Some("formal"),
                "informal" | "friendly" | "casual" => Some("informal"),
                "empathetic" => Some("supportive"),
                _ => None,
            }
        }
    }

    /// Calculador de prioridad conversacional.
    #[derive(Debug, Default)]
    pub struct ConversationalPriorityCalculator;

    impl PriorityCalculator for ConversationalPriorityCalculator {
        fn calculate(&self, description: &str, content: &str, outcome: &str) -> Priority {
            let text = format!("{} {} {}", description, content, outcome).to_lowercase();

            // Critical: user preferences, safety, explicit requests
            if self.critical_keywords().iter().any(|k| text.contains(k)) {
                return Priority::Critical;
            }

            // High: emotional states, important preferences
            if self.high_keywords().iter().any(|k| text.contains(k)) {
                return Priority::High;
            }

            // Low: casual interactions, generic responses
            if self.low_keywords().iter().any(|k| text.contains(k)) {
                return Priority::Low;
            }

            Priority::Normal
        }

        fn critical_keywords(&self) -> Vec<&'static str> {
            vec![
                "never",
                "always",
                "hate",
                "love",
                "allergy",
                "allergic",
                "important",
                "remember",
                "don't forget",
                "must",
                "preference",
                "please don't",
                "stop",
            ]
        }

        fn high_keywords(&self) -> Vec<&'static str> {
            vec![
                "frustrated",
                "angry",
                "upset",
                "disappointed",
                "happy",
                "excited",
                "grateful",
                "thank",
                "favorite",
                "prefer",
                "like",
                "dislike",
            ]
        }

        fn low_keywords(&self) -> Vec<&'static str> {
            vec![
                "ok", "fine", "sure", "maybe", "whatever", "casual", "just", "random",
            ]
        }
    }

    /// Preset para chatbots y agentes conversacionales.
    pub struct Conversational;

    impl DomainPreset for Conversational {
        type Domain = ConversationalDomainClassifier;
        type Concepts = ConversationalConceptExtractor;
        type Context = ConversationalToneMatcher;
        type Priority = ConversationalPriorityCalculator;

        fn name() -> &'static str {
            "Conversational"
        }

        fn description() -> &'static str {
            "Memory system for chatbots and conversational agents"
        }

        fn default_decay() -> DecayConfig {
            // Conversations decay faster (1 week)
            DecayConfig::fast()
        }

        fn default_weights() -> PriorityWeights {
            // Prioritize recency for conversations
            PriorityWeights::recency_focused()
        }
    }

    // ------------------------------------------------------------------------
    // Customer Service Preset
    // ------------------------------------------------------------------------

    /// Clasificador de dominios de servicio al cliente.
    #[derive(Debug, Default)]
    pub struct CustomerServiceDomainClassifier;

    impl DomainClassifier for CustomerServiceDomainClassifier {
        fn domain_type_name(&self) -> &'static str {
            "service_domain"
        }

        fn available_domains(&self) -> Vec<&'static str> {
            vec![
                "billing",
                "technical",
                "returns",
                "shipping",
                "account",
                "product_info",
                "complaints",
                "general",
            ]
        }

        fn classify(&self, content: &str) -> String {
            let lower = content.to_lowercase();

            if lower.contains("bill") || lower.contains("charge") || lower.contains("payment") {
                "billing".into()
            } else if lower.contains("broken")
                || lower.contains("not working")
                || lower.contains("bug")
            {
                "technical".into()
            } else if lower.contains("return")
                || lower.contains("refund")
                || lower.contains("exchange")
            {
                "returns".into()
            } else if lower.contains("ship")
                || lower.contains("deliver")
                || lower.contains("tracking")
            {
                "shipping".into()
            } else if lower.contains("account")
                || lower.contains("password")
                || lower.contains("login")
            {
                "account".into()
            } else if lower.contains("product")
                || lower.contains("feature")
                || lower.contains("spec")
            {
                "product_info".into()
            } else if lower.contains("complain")
                || lower.contains("unhappy")
                || lower.contains("terrible")
            {
                "complaints".into()
            } else {
                "general".into()
            }
        }

        fn related(&self, domain1: &str, domain2: &str) -> bool {
            let money = ["billing", "returns"];
            let logistics = ["shipping", "returns"];
            let tech = ["technical", "account"];

            (money.contains(&domain1) && money.contains(&domain2))
                || (logistics.contains(&domain1) && logistics.contains(&domain2))
                || (tech.contains(&domain1) && tech.contains(&domain2))
        }
    }

    /// Extractor de conceptos de servicio al cliente.
    #[derive(Debug, Default)]
    pub struct CustomerServiceConceptExtractor;

    impl ConceptExtractor for CustomerServiceConceptExtractor {
        fn extract(&self, description: &str, content: &str) -> Vec<String> {
            let text = format!("{} {}", description, content).to_lowercase();
            let mut concepts = Vec::new();

            let patterns = [
                (
                    "escalation needed",
                    &["manager", "supervisor", "escalate"][..],
                ),
                ("resolution", &["solved", "fixed", "resolved", "done"]),
                ("compensation", &["refund", "credit", "discount", "free"]),
                ("verification", &["verify", "confirm", "check identity"]),
                ("policy reference", &["policy", "terms", "conditions"]),
                ("empathy response", &["understand", "sorry", "apologize"]),
                (
                    "follow up needed",
                    &["follow up", "callback", "contact again"],
                ),
                ("urgent", &["urgent", "emergency", "asap"]),
            ];

            for (concept, keywords) in patterns {
                if keywords.iter().any(|k| text.contains(k)) {
                    concepts.push(concept.to_string());
                }
            }

            concepts
        }

        fn is_universal(&self, concept: &str) -> bool {
            self.universal_concepts().contains(&concept)
        }

        fn universal_concepts(&self) -> Vec<&'static str> {
            vec![
                "empathy response",
                "verification",
                "resolution",
                "follow up needed",
            ]
        }
    }

    /// Evaluador de tipo de cliente.
    #[derive(Debug, Default)]
    pub struct CustomerTierMatcher;

    impl ContextMatcher for CustomerTierMatcher {
        fn context_type_name(&self) -> &'static str {
            "customer_tier"
        }

        fn available_contexts(&self) -> Vec<&'static str> {
            vec!["vip", "premium", "standard", "new", "at_risk", "churned"]
        }

        fn compatibility(&self, ctx1: &str, ctx2: &str) -> f32 {
            if ctx1 == ctx2 {
                return 1.0;
            }

            let family1 = self.context_family(ctx1);
            let family2 = self.context_family(ctx2);

            if family1.is_some() && family1 == family2 {
                0.7
            } else {
                0.5 // Customer service patterns are often broadly applicable
            }
        }

        fn context_family(&self, context: &str) -> Option<&'static str> {
            match context.to_lowercase().as_str() {
                "vip" | "premium" => Some("high_value"),
                "standard" | "new" => Some("regular"),
                "at_risk" | "churned" => Some("retention"),
                _ => None,
            }
        }
    }

    /// Calculador de prioridad para servicio al cliente.
    #[derive(Debug, Default)]
    pub struct CustomerServicePriorityCalculator;

    impl PriorityCalculator for CustomerServicePriorityCalculator {
        fn calculate(&self, description: &str, content: &str, outcome: &str) -> Priority {
            let text = format!("{} {} {}", description, content, outcome).to_lowercase();

            // Critical: VIP, legal, escalations
            if self.critical_keywords().iter().any(|k| text.contains(k)) {
                return Priority::Critical;
            }

            // High: complaints, refunds, unhappy
            if self.high_keywords().iter().any(|k| text.contains(k)) {
                return Priority::High;
            }

            // Low: general inquiries, routine
            if self.low_keywords().iter().any(|k| text.contains(k)) {
                return Priority::Low;
            }

            Priority::Normal
        }

        fn critical_keywords(&self) -> Vec<&'static str> {
            vec![
                "vip",
                "enterprise",
                "legal",
                "lawyer",
                "sue",
                "escalate",
                "manager",
                "supervisor",
                "ceo",
                "fraud",
                "breach",
                "unauthorized",
            ]
        }

        fn high_keywords(&self) -> Vec<&'static str> {
            vec![
                "complaint",
                "unhappy",
                "refund",
                "cancel",
                "broken",
                "defective",
                "wrong",
                "missing",
                "urgent",
                "immediately",
                "asap",
            ]
        }

        fn low_keywords(&self) -> Vec<&'static str> {
            vec![
                "question",
                "inquiry",
                "information",
                "how to",
                "general",
                "routine",
                "standard",
            ]
        }
    }

    /// Preset para servicio al cliente.
    pub struct CustomerService;

    impl DomainPreset for CustomerService {
        type Domain = CustomerServiceDomainClassifier;
        type Concepts = CustomerServiceConceptExtractor;
        type Context = CustomerTierMatcher;
        type Priority = CustomerServicePriorityCalculator;

        fn name() -> &'static str {
            "Customer Service"
        }

        fn description() -> &'static str {
            "Memory system for customer service agents"
        }

        fn default_decay() -> DecayConfig {
            // Customer interactions decay moderately (30 days)
            DecayConfig::default()
        }

        fn default_weights() -> PriorityWeights {
            // Prioritize manual priority for customer service
            PriorityWeights::manual_focused()
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::presets::*;
    use super::*;

    #[test]
    fn test_transfer_level_ordering() {
        assert!(TransferLevel::Universal > TransferLevel::Domain);
        assert!(TransferLevel::Domain > TransferLevel::Context);
        assert!(TransferLevel::Context > TransferLevel::Instance);
    }

    #[test]
    fn test_transfer_level_scores() {
        assert_eq!(TransferLevel::Universal.transfer_score(), 1.0);
        assert_eq!(TransferLevel::Domain.transfer_score(), 0.75);
        assert_eq!(TransferLevel::Context.transfer_score(), 0.5);
        assert_eq!(TransferLevel::Instance.transfer_score(), 0.25);
    }

    #[test]
    fn test_instance_context_builder() {
        let ctx = InstanceContext::new("my-project")
            .with_context("rust")
            .with_domain("web_backend")
            .with_extra("framework", "actix");

        assert_eq!(ctx.instance_id, "my-project");
        assert_eq!(ctx.context, "rust");
        assert_eq!(ctx.domain, "web_backend");
        assert_eq!(ctx.extra.get("framework"), Some(&"actix".to_string()));
    }

    #[test]
    fn test_software_domain_classifier() {
        let classifier = SoftwareDomainClassifier;

        assert_eq!(classifier.classify("REST API endpoint"), "web_backend");
        assert_eq!(classifier.classify("React component"), "web_frontend");
        assert_eq!(classifier.classify("CLI tool"), "cli");
        assert_eq!(classifier.classify("pandas dataframe"), "data_science");
    }

    #[test]
    fn test_software_concept_extractor() {
        let extractor = SoftwareConceptExtractor;

        let concepts = extractor.extract(
            "JWT authentication with rate limiting",
            "middleware auth jwt token",
        );

        assert!(concepts.contains(&"authentication".to_string()));
        assert!(concepts.contains(&"rate limiting".to_string()));
        assert!(concepts.contains(&"middleware".to_string()));
    }

    #[test]
    fn test_programming_language_matcher() {
        let matcher = ProgrammingLanguageMatcher;

        assert_eq!(matcher.compatibility("rust", "rust"), 1.0);
        assert_eq!(matcher.compatibility("javascript", "typescript"), 0.8);
        assert!(matcher.compatibility("rust", "python") < 0.5);
    }

    #[test]
    fn test_conversational_domain_classifier() {
        let classifier = ConversationalDomainClassifier;

        assert_eq!(classifier.classify("I need help with my order"), "support");
        assert_eq!(classifier.classify("How much does it cost?"), "sales");
        assert_eq!(classifier.classify("Tell me a joke"), "entertainment");
    }

    #[test]
    fn test_conversational_concept_extractor() {
        let extractor = ConversationalConceptExtractor;

        let concepts = extractor.extract("User greeting", "Hello, how are you?");
        assert!(concepts.contains(&"greeting".to_string()));

        let concepts = extractor.extract("User frustrated", "I'm very upset about this");
        assert!(concepts.contains(&"frustration".to_string()));
    }

    #[test]
    fn test_customer_service_domain_classifier() {
        let classifier = CustomerServiceDomainClassifier;

        assert_eq!(classifier.classify("Wrong charge on my bill"), "billing");
        assert_eq!(classifier.classify("Product not working"), "technical");
        assert_eq!(classifier.classify("I want to return this"), "returns");
    }

    #[test]
    fn test_generic_memory_creation() {
        let memory = GenericMemory::<SoftwareDevelopment>::new(4).unwrap();
        assert_eq!(memory.stats().preset_name, "Software Development");
        assert!(!memory.stats().has_context);
    }

    #[test]
    fn test_generic_memory_set_context() {
        let memory = GenericMemory::<Conversational>::new(4).unwrap();

        memory.set_instance("@user123", "casual", "support");

        let ctx = memory.current_context().unwrap();
        assert_eq!(ctx.instance_id, "@user123");
        assert_eq!(ctx.context, "casual");
        assert_eq!(ctx.domain, "support");
    }

    #[test]
    fn test_domain_relatedness() {
        let classifier = SoftwareDomainClassifier;

        assert!(classifier.related("web_backend", "web_frontend"));
        assert!(classifier.related("systems", "embedded"));
        assert!(!classifier.related("web_backend", "gamedev"));
    }

    #[test]
    fn test_preset_names() {
        assert_eq!(SoftwareDevelopment::name(), "Software Development");
        assert_eq!(Conversational::name(), "Conversational");
        assert_eq!(CustomerService::name(), "Customer Service");
    }

    // ========================================================================
    // Priority System Tests
    // ========================================================================

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::Critical > Priority::High);
        assert!(Priority::High > Priority::Normal);
        assert!(Priority::Normal > Priority::Low);
    }

    #[test]
    fn test_priority_scores() {
        assert_eq!(Priority::Critical.base_score(), 1.0);
        assert_eq!(Priority::High.base_score(), 0.75);
        assert_eq!(Priority::Normal.base_score(), 0.5);
        assert_eq!(Priority::Low.base_score(), 0.25);
    }

    #[test]
    fn test_priority_from_str() {
        assert_eq!(Priority::from_str("critical"), Some(Priority::Critical));
        assert_eq!(Priority::from_str("urgent"), Some(Priority::Critical));
        assert_eq!(Priority::from_str("high"), Some(Priority::High));
        assert_eq!(Priority::from_str("normal"), Some(Priority::Normal));
        assert_eq!(Priority::from_str("low"), Some(Priority::Low));
        assert_eq!(Priority::from_str("unknown"), None);
    }

    #[test]
    fn test_usage_stats_frequency_score() {
        let mut stats = UsageStats::new();
        assert_eq!(stats.frequency_score(), 0.0);

        stats.access_count = 1;
        assert!(stats.frequency_score() > 0.0);

        stats.access_count = 100;
        assert!(stats.frequency_score() > 0.5);
        assert!(stats.frequency_score() <= 1.0);
    }

    #[test]
    fn test_usage_stats_usefulness() {
        let mut stats = UsageStats::new();
        assert_eq!(stats.usefulness_score(), 0.5); // Neutral when no access

        stats.access_count = 10;
        stats.useful_count = 8;
        assert_eq!(stats.usefulness_score(), 0.8);
    }

    #[test]
    fn test_decay_config_calculation() {
        let config = DecayConfig::default();

        // Critical priority should not decay
        assert_eq!(config.calculate_decay(1000000, Priority::Critical), 1.0);

        // Normal priority should decay
        let decay = config.calculate_decay(30 * 24 * 60 * 60, Priority::Normal);
        assert!(decay < 1.0);
        assert!(decay >= 0.4); // After half-life, should be around 0.5
    }

    #[test]
    fn test_decay_config_no_decay() {
        let config = DecayConfig::no_decay();
        assert_eq!(config.calculate_decay(1000000, Priority::Low), 1.0);
    }

    #[test]
    fn test_priority_weights() {
        let weights = PriorityWeights::default();
        let score = weights.calculate_score(0.5, 0.3, 0.8, 0.6);
        assert!(score > 0.0 && score <= 1.0);
    }

    #[test]
    fn test_recency_score() {
        // Very recent should be high
        assert!(recency_score(0) > 0.99);

        // 1 week old should be around 0.37 (e^-1)
        let week_old = recency_score(7 * 24 * 60 * 60);
        assert!(week_old > 0.3 && week_old < 0.5);

        // Very old should be low
        assert!(recency_score(365 * 24 * 60 * 60) < 0.1);
    }

    #[test]
    fn test_software_priority_calculator() {
        let calc = SoftwarePriorityCalculator;

        // Security issue = Critical
        assert_eq!(
            calc.calculate("XSS vulnerability fix", "sanitize input", "fixed"),
            Priority::Critical
        );

        // Bug = High
        assert_eq!(
            calc.calculate("Bug fix", "crash on startup", "resolved"),
            Priority::High
        );

        // Style = Low
        assert_eq!(
            calc.calculate("Formatting", "apply prettier", "done"),
            Priority::Low
        );

        // Normal code = Normal
        assert_eq!(
            calc.calculate("Add feature", "new button", "completed"),
            Priority::Normal
        );
    }

    #[test]
    fn test_conversational_priority_calculator() {
        let calc = ConversationalPriorityCalculator;

        // User preference = Critical
        assert_eq!(
            calc.calculate("User preference", "I never want spam", "noted"),
            Priority::Critical
        );

        // Emotional state = High
        assert_eq!(
            calc.calculate("User upset", "I'm frustrated", "apologized"),
            Priority::High
        );

        // Casual = Low
        assert_eq!(
            calc.calculate("Casual chat", "just ok", "acknowledged"),
            Priority::Low
        );
    }

    #[test]
    fn test_customer_service_priority_calculator() {
        let calc = CustomerServicePriorityCalculator;

        // VIP = Critical
        assert_eq!(
            calc.calculate("VIP customer", "enterprise account", "handled"),
            Priority::Critical
        );

        // Complaint = High
        assert_eq!(
            calc.calculate("Customer complaint", "refund request", "processed"),
            Priority::High
        );

        // Inquiry = Low
        assert_eq!(
            calc.calculate("General question", "product information", "answered"),
            Priority::Low
        );
    }

    #[test]
    fn test_priority_weights_presets() {
        let manual = PriorityWeights::manual_focused();
        assert!(manual.base_priority > manual.frequency);

        let usage = PriorityWeights::usage_focused();
        assert!(usage.frequency > usage.base_priority);

        let recency = PriorityWeights::recency_focused();
        assert!(recency.recency > recency.base_priority);
    }

    #[test]
    fn test_memory_stats_includes_usage() {
        let memory = GenericMemory::<SoftwareDevelopment>::new(4).unwrap();
        let stats = memory.stats();
        assert_eq!(stats.total_accesses, 0);
        assert_eq!(stats.avg_usefulness, 0.0);
    }
}
