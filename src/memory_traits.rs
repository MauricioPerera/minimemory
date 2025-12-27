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

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::types::{Metadata, VectorId};
use crate::VectorDB;
use crate::Config;
use crate::Filter;

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
/// Combina los tres traits en una configuracion cohesiva.
pub trait DomainPreset: Send + Sync + 'static {
    type Domain: DomainClassifier;
    type Concepts: ConceptExtractor;
    type Context: ContextMatcher;

    /// Nombre del preset.
    fn name() -> &'static str;

    /// Descripcion del preset.
    fn description() -> &'static str;

    /// Crea instancias de los componentes.
    fn create() -> (Self::Domain, Self::Concepts, Self::Context) {
        (
            Self::Domain::default(),
            Self::Concepts::default(),
            Self::Context::default(),
        )
    }
}

// ============================================================================
// Transfer Level (Generico)
// ============================================================================

/// Nivel de transferibilidad del conocimiento.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum TransferLevel {
    /// Solo aplica a esta instancia especifica.
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

impl Default for TransferLevel {
    fn default() -> Self {
        Self::Instance
    }
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

/// Resultado de recall con informacion de transferibilidad.
#[derive(Debug, Clone)]
pub struct GenericRecall {
    /// ID del recuerdo.
    pub id: VectorId,
    /// Score de relevancia semantica (0.0 - 1.0).
    pub relevance: f32,
    /// Nivel de transferibilidad.
    pub transfer_level: TransferLevel,
    /// Score combinado (relevancia * transferibilidad).
    pub combined_score: f32,
    /// Conceptos abstractos asociados.
    pub concepts: Vec<String>,
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
    /// Contexto actual.
    current_context: RwLock<Option<InstanceContext>>,
    /// Peso de relevancia vs transferibilidad.
    relevance_weight: f32,
    /// Umbral minimo de transferibilidad.
    transfer_threshold: f32,
}

impl<P: DomainPreset> GenericMemory<P> {
    /// Crea una nueva memoria generica.
    pub fn new(dimensions: usize) -> Result<Self> {
        let config = Config::new(dimensions);
        let db = VectorDB::with_fulltext(config, vec!["content".into(), "description".into()])?;
        let (domain_classifier, concept_extractor, context_matcher) = P::create();

        Ok(Self {
            db,
            domain_classifier,
            concept_extractor,
            context_matcher,
            current_context: RwLock::new(None),
            relevance_weight: 0.6,
            transfer_threshold: 0.3,
        })
    }

    /// Crea con configuracion personalizada.
    pub fn with_config(config: Config) -> Result<Self> {
        let db = VectorDB::with_fulltext(config, vec!["content".into(), "description".into()])?;
        let (domain_classifier, concept_extractor, context_matcher) = P::create();

        Ok(Self {
            db,
            domain_classifier,
            concept_extractor,
            context_matcher,
            current_context: RwLock::new(None),
            relevance_weight: 0.6,
            transfer_threshold: 0.3,
        })
    }

    /// Establece el peso de relevancia (0.0 - 1.0).
    pub fn set_relevance_weight(&mut self, weight: f32) {
        self.relevance_weight = weight.clamp(0.0, 1.0);
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

    /// Aprende nuevo conocimiento.
    pub fn learn(
        &self,
        id: &str,
        embedding: &[f32],
        content: &str,
        description: &str,
        outcome: &str,
    ) -> Result<VectorId> {
        let ctx = self.current_context.read().clone();

        // Extraer conceptos
        let concepts = self.concept_extractor.extract(description, content);

        // Inferir nivel de transferencia
        let transfer_level = self.infer_transfer_level(&concepts, content);

        // Construir metadata
        let mut meta = Metadata::new();
        meta.insert("content", content);
        meta.insert("description", description);
        meta.insert("outcome", outcome);
        meta.insert("transfer_level", transfer_level.as_str());
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
        if ctx.as_ref().map_or(true, |c| c.domain.is_empty()) {
            let domain = self.domain_classifier.classify(content);
            meta.insert("domain", domain.as_str());
        }

        self.db.insert(id, embedding, Some(meta))?;
        Ok(id.to_string())
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

        if instance_patterns
            .iter()
            .any(|p| content_lower.contains(p))
        {
            return TransferLevel::Instance;
        }

        // Por defecto, nivel de contexto
        if universal_count >= 1 {
            TransferLevel::Domain
        } else {
            TransferLevel::Context
        }
    }

    /// Recall con filtrado por transferibilidad.
    pub fn recall(&self, query_embedding: &[f32], k: usize) -> Result<Vec<GenericRecall>> {
        let ctx = self.current_context.read().clone();

        // Buscar en la base de datos
        let results = self.db.search(query_embedding, k * 3)?;

        let mut recalls: Vec<GenericRecall> = results
            .into_iter()
            .filter_map(|r| {
                let meta = r.metadata?;

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

                // Score combinado
                let relevance = 1.0 - r.distance; // Convertir distancia a similitud
                let combined_score = relevance * self.relevance_weight
                    + transfer_score * (1.0 - self.relevance_weight);

                // Extraer conceptos
                let concepts = meta
                    .get("concepts")
                    .and_then(|v| v.as_str())
                    .map(|s| s.split(',').map(String::from).collect())
                    .unwrap_or_default();

                Some(GenericRecall {
                    id: r.id,
                    relevance,
                    transfer_level,
                    combined_score,
                    concepts,
                    metadata: meta,
                })
            })
            .collect();

        // Ordenar por score combinado
        recalls.sort_by(|a, b| b.combined_score.partial_cmp(&a.combined_score).unwrap());
        recalls.truncate(k);

        Ok(recalls)
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
            TransferLevel::Instance => instance_match * 0.6 + context_compat * 0.2 + domain_compat * 0.2,
            TransferLevel::Context => context_compat * 0.5 + domain_compat * 0.3 + 0.2,
            TransferLevel::Domain => domain_compat * 0.6 + 0.4,
            TransferLevel::Universal => 1.0,
        }
    }

    /// Recall solo de conocimiento universal.
    pub fn recall_universal(&self, query_embedding: &[f32], k: usize) -> Result<Vec<GenericRecall>> {
        let results = self.db.search_with_filter(
            query_embedding,
            k,
            Filter::eq("transfer_level", "universal"),
        )?;

        Ok(results
            .into_iter()
            .filter_map(|r| {
                let meta = r.metadata?;
                let concepts = meta
                    .get("concepts")
                    .and_then(|v| v.as_str())
                    .map(|s| s.split(',').map(String::from).collect())
                    .unwrap_or_default();

                Some(GenericRecall {
                    id: r.id,
                    relevance: 1.0 - r.distance,
                    transfer_level: TransferLevel::Universal,
                    combined_score: 1.0 - r.distance,
                    concepts,
                    metadata: meta,
                })
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
                let transfer_level = meta
                    .get("transfer_level")
                    .and_then(|v| v.as_str())
                    .and_then(TransferLevel::from_str)
                    .unwrap_or(TransferLevel::Domain);

                let concepts = meta
                    .get("concepts")
                    .and_then(|v| v.as_str())
                    .map(|s| s.split(',').map(String::from).collect())
                    .unwrap_or_default();

                Some(GenericRecall {
                    id: r.id,
                    relevance: 1.0 - r.distance,
                    transfer_level,
                    combined_score: 1.0 - r.distance,
                    concepts,
                    metadata: meta,
                })
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
                let transfer_level = meta
                    .get("transfer_level")
                    .and_then(|v| v.as_str())
                    .and_then(TransferLevel::from_str)
                    .unwrap_or(TransferLevel::Context);

                let concepts = meta
                    .get("concepts")
                    .and_then(|v| v.as_str())
                    .map(|s| s.split(',').map(String::from).collect())
                    .unwrap_or_default();

                Some(GenericRecall {
                    id: r.id,
                    relevance: 1.0 - r.distance,
                    transfer_level,
                    combined_score: 1.0 - r.distance,
                    concepts,
                    metadata: meta,
                })
            })
            .collect())
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

                let concepts = meta
                    .get("concepts")
                    .and_then(|v| v.as_str())
                    .map(|s| s.split(',').map(String::from).collect())
                    .unwrap_or_default();

                Some(GenericRecall {
                    id: r.id,
                    relevance: r.score,
                    transfer_level,
                    combined_score: r.score,
                    concepts,
                    metadata: meta,
                })
            })
            .collect())
    }

    /// Estadisticas de la memoria.
    pub fn stats(&self) -> MemoryStats {
        MemoryStats {
            total_memories: self.db.len(),
            preset_name: P::name().to_string(),
            has_context: self.current_context.read().is_some(),
        }
    }
}

/// Estadisticas de la memoria.
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub total_memories: usize,
    pub preset_name: String,
    pub has_context: bool,
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
            } else if lower.contains("cli") || lower.contains("terminal") || lower.contains("command") {
                "cli".into()
            } else if lower.contains("pandas") || lower.contains("numpy") || lower.contains("ml") {
                "data_science".into()
            } else if lower.contains("docker") || lower.contains("kubernetes") || lower.contains("ci/cd") {
                "devops".into()
            } else if lower.contains("auth") || lower.contains("security") || lower.contains("encrypt") {
                "security".into()
            } else if lower.contains("sql") || lower.contains("database") || lower.contains("query") {
                "database".into()
            } else if lower.contains("android") || lower.contains("ios") || lower.contains("mobile") {
                "mobile".into()
            } else if lower.contains("kernel") || lower.contains("memory") || lower.contains("syscall") {
                "systems".into()
            } else if lower.contains("game") || lower.contains("render") || lower.contains("sprite") {
                "gamedev".into()
            } else if lower.contains("embedded") || lower.contains("mcu") || lower.contains("firmware") {
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
                ("error handling", &["error", "exception", "try", "catch", "result"][..]),
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
                "rust", "python", "javascript", "typescript", "go", "java",
                "c", "cpp", "csharp", "ruby", "php", "swift", "kotlin",
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

    /// Preset para desarrollo de software.
    pub struct SoftwareDevelopment;

    impl DomainPreset for SoftwareDevelopment {
        type Domain = SoftwareDomainClassifier;
        type Concepts = SoftwareConceptExtractor;
        type Context = ProgrammingLanguageMatcher;

        fn name() -> &'static str {
            "Software Development"
        }

        fn description() -> &'static str {
            "Memory system for software development agents with code-aware transfer"
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
            } else if lower.contains("buy") || lower.contains("price") || lower.contains("offer") || lower.contains("cost") {
                "sales".into()
            } else if lower.contains("joke") || lower.contains("fun") || lower.contains("play") {
                "entertainment".into()
            } else if lower.contains("learn") || lower.contains("explain") || lower.contains("how") {
                "education".into()
            } else if lower.contains("meeting") || lower.contains("report") || lower.contains("deadline") {
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
                ("greeting", &["hello", "hi", "hey", "good morning", "hola"][..]),
                ("farewell", &["bye", "goodbye", "see you", "adios"]),
                ("gratitude", &["thank", "thanks", "gracias", "appreciate"]),
                ("apology", &["sorry", "apologize", "excuse", "disculpa"]),
                ("empathy", &["understand", "feel", "sorry to hear"]),
                ("clarification", &["mean", "clarify", "explain", "what do you"]),
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

    /// Preset para chatbots y agentes conversacionales.
    pub struct Conversational;

    impl DomainPreset for Conversational {
        type Domain = ConversationalDomainClassifier;
        type Concepts = ConversationalConceptExtractor;
        type Context = ConversationalToneMatcher;

        fn name() -> &'static str {
            "Conversational"
        }

        fn description() -> &'static str {
            "Memory system for chatbots and conversational agents"
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
            } else if lower.contains("broken") || lower.contains("not working") || lower.contains("bug") {
                "technical".into()
            } else if lower.contains("return") || lower.contains("refund") || lower.contains("exchange") {
                "returns".into()
            } else if lower.contains("ship") || lower.contains("deliver") || lower.contains("tracking") {
                "shipping".into()
            } else if lower.contains("account") || lower.contains("password") || lower.contains("login") {
                "account".into()
            } else if lower.contains("product") || lower.contains("feature") || lower.contains("spec") {
                "product_info".into()
            } else if lower.contains("complain") || lower.contains("unhappy") || lower.contains("terrible") {
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
                ("escalation needed", &["manager", "supervisor", "escalate"][..]),
                ("resolution", &["solved", "fixed", "resolved", "done"]),
                ("compensation", &["refund", "credit", "discount", "free"]),
                ("verification", &["verify", "confirm", "check identity"]),
                ("policy reference", &["policy", "terms", "conditions"]),
                ("empathy response", &["understand", "sorry", "apologize"]),
                ("follow up needed", &["follow up", "callback", "contact again"]),
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

    /// Preset para servicio al cliente.
    pub struct CustomerService;

    impl DomainPreset for CustomerService {
        type Domain = CustomerServiceDomainClassifier;
        type Concepts = CustomerServiceConceptExtractor;
        type Context = CustomerTierMatcher;

        fn name() -> &'static str {
            "Customer Service"
        }

        fn description() -> &'static str {
            "Memory system for customer service agents"
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use super::presets::*;

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
}
