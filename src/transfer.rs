//! # Sistema de Transferencia de Conocimiento
//!
//! Permite que el conocimiento aprendido en un proyecto sea aplicable
//! (con adaptaciones) a otros proyectos con diferentes especificaciones.
//!
//! ## Niveles de Transferibilidad
//!
//! - **Universal**: Patrones de diseño, principios SOLID, algoritmos
//! - **Dominio**: Conocimiento específico de un dominio (web, CLI, data)
//! - **Stack**: Conocimiento específico de un lenguaje/framework
//! - **Proyecto**: Conocimiento específico que no se transfiere
//!
//! ## Ejemplo
//!
//! ```rust,ignore
//! use minimemory::transfer::{TransferableMemory, ProjectContext, KnowledgeDomain};
//!
//! let mut memory = TransferableMemory::new()?;
//!
//! // Configurar proyecto actual
//! memory.set_project_context(ProjectContext::new(
//!     "my-rust-api",
//!     "rust",
//!     KnowledgeDomain::WebBackend,
//! ).with_frameworks(vec!["Axum".into()])
//!  .with_patterns(vec!["REST".into()]));
//!
//! // Buscar conocimiento transferible
//! let results = memory.recall_transferable("implementar autenticación", 5)?;
//!
//! for result in results {
//!     if result.applicable {
//!         println!("Aplicable: {}", result.recall.content);
//!         if let Some(adapt) = result.adaptation_needed {
//!             println!("  Adaptación: {}", adapt);
//!         }
//!     }
//! }
//! ```

use parking_lot::RwLock;

use crate::agent_memory::{
    AgentMemory, CodeSnippet, Language, MemoryConfig, MemoryRecall, TaskEpisode, TaskOutcome,
};
use crate::error::Result;
use crate::memory_traits::TransferLevel;
use crate::types::{MetadataValue, VectorId};

// Types are now defined in memory_traits.rs — re-export for backward compatibility.
pub use crate::memory_traits::{KnowledgeDomain, LanguageCompatibility, ProjectContext};

// ============================================================================
// Resultado de Búsqueda con Transferencia
// ============================================================================

/// Resultado de búsqueda con información de transferibilidad.
#[derive(Debug)]
pub struct TransferableRecall {
    /// El recall original de la memoria
    pub recall: MemoryRecall,
    /// Score de transferibilidad (0.0 - 1.0)
    pub transfer_score: f32,
    /// Score combinado (relevancia + transferibilidad)
    pub combined_score: f32,
    /// Si es directamente aplicable al contexto actual
    pub applicable: bool,
    /// Descripción de la adaptación necesaria, si aplica
    pub adaptation_needed: Option<String>,
    /// Nivel de transferencia del conocimiento
    pub transfer_level: TransferLevel,
    /// Conceptos abstractos extraídos
    pub concepts: Vec<String>,
}

// ============================================================================
// Extractor de Conceptos
// ============================================================================

/// Extrae conceptos abstractos de código y tareas.
pub struct ConceptExtractor {
    /// Patrones de diseño conocidos
    design_patterns: Vec<(&'static str, &'static str)>,
    /// Conceptos de dominio
    domain_concepts: Vec<(&'static str, &'static str)>,
    /// Principios de programación
    principles: Vec<(&'static str, &'static str)>,
}

impl Default for ConceptExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl ConceptExtractor {
    pub fn new() -> Self {
        Self {
            design_patterns: vec![
                ("factory", "Factory Pattern"),
                ("singleton", "Singleton Pattern"),
                ("observer", "Observer Pattern"),
                ("strategy", "Strategy Pattern"),
                ("decorator", "Decorator Pattern"),
                ("adapter", "Adapter Pattern"),
                ("facade", "Facade Pattern"),
                ("proxy", "Proxy Pattern"),
                ("builder", "Builder Pattern"),
                ("prototype", "Prototype Pattern"),
                ("middleware", "Middleware Pattern"),
                ("repository", "Repository Pattern"),
                ("unit of work", "Unit of Work Pattern"),
                ("dependency injection", "Dependency Injection"),
                ("event sourcing", "Event Sourcing"),
                ("cqrs", "CQRS Pattern"),
                ("saga", "Saga Pattern"),
                ("circuit breaker", "Circuit Breaker"),
            ],
            domain_concepts: vec![
                ("authentication", "Authentication"),
                ("authorization", "Authorization"),
                ("jwt", "JWT Tokens"),
                ("oauth", "OAuth"),
                ("session", "Session Management"),
                ("rate limit", "Rate Limiting"),
                ("throttl", "Throttling"),
                ("cache", "Caching"),
                ("pagination", "Pagination"),
                ("validation", "Input Validation"),
                ("sanitiz", "Input Sanitization"),
                ("error handling", "Error Handling"),
                ("logging", "Logging"),
                ("monitoring", "Monitoring"),
                ("testing", "Testing"),
                ("mocking", "Mocking"),
                ("serializ", "Serialization"),
                ("deserializ", "Deserialization"),
                ("encryption", "Encryption"),
                ("hashing", "Hashing"),
                ("compression", "Compression"),
                ("streaming", "Data Streaming"),
                ("websocket", "WebSockets"),
                ("graphql", "GraphQL"),
                ("rest", "REST API"),
                ("grpc", "gRPC"),
                ("queue", "Message Queue"),
                ("pub sub", "Pub/Sub"),
                ("batch", "Batch Processing"),
                ("concurrency", "Concurrency"),
                ("parallelism", "Parallelism"),
                ("async", "Async Programming"),
            ],
            principles: vec![
                ("solid", "SOLID Principles"),
                ("dry", "DRY Principle"),
                ("kiss", "KISS Principle"),
                ("yagni", "YAGNI Principle"),
                ("separation of concern", "Separation of Concerns"),
                ("single responsibility", "Single Responsibility"),
                ("open closed", "Open/Closed Principle"),
                ("liskov", "Liskov Substitution"),
                ("interface segregation", "Interface Segregation"),
                ("dependency inversion", "Dependency Inversion"),
                (
                    "composition over inheritance",
                    "Composition over Inheritance",
                ),
                ("fail fast", "Fail Fast"),
                ("defensive programming", "Defensive Programming"),
            ],
        }
    }

    /// Extrae conceptos del texto (tarea + código)
    pub fn extract(&self, task: &str, code: &str) -> Vec<String> {
        let text = format!("{} {}", task, code).to_lowercase();
        let mut concepts = Vec::new();

        // Patrones de diseño
        for (keyword, concept) in &self.design_patterns {
            if text.contains(keyword) {
                concepts.push(concept.to_string());
            }
        }

        // Conceptos de dominio
        for (keyword, concept) in &self.domain_concepts {
            if text.contains(keyword) {
                concepts.push(concept.to_string());
            }
        }

        // Principios
        for (keyword, concept) in &self.principles {
            if text.contains(keyword) {
                concepts.push(concept.to_string());
            }
        }

        // Deduplicar
        concepts.sort();
        concepts.dedup();
        concepts
    }

    /// Determina el nivel de transferencia basado en los conceptos
    pub fn infer_transfer_level(&self, concepts: &[String]) -> TransferLevel {
        // Si contiene principios fundamentales, es universal
        let has_principle = concepts
            .iter()
            .any(|c| self.principles.iter().any(|(_, name)| c == *name));

        if has_principle {
            return TransferLevel::Universal;
        }

        // Si tiene patrones de diseño, es al menos de dominio
        let has_pattern = concepts
            .iter()
            .any(|c| self.design_patterns.iter().any(|(_, name)| c == *name));

        if has_pattern {
            return TransferLevel::Domain;
        }

        // Si tiene conceptos de dominio, es de dominio
        if !concepts.is_empty() {
            return TransferLevel::Domain;
        }

        TransferLevel::Context
    }
}

/// Implement the generic ConceptExtractor trait for interoperability with GenericMemory.
impl crate::memory_traits::ConceptExtractor for ConceptExtractor {
    fn extract(&self, description: &str, content: &str) -> Vec<String> {
        // Delegate to inherent method
        ConceptExtractor::extract(self, description, content)
    }

    fn is_universal(&self, concept: &str) -> bool {
        self.principles.iter().any(|(_, name)| *name == concept)
    }

    fn universal_concepts(&self) -> Vec<&'static str> {
        self.principles.iter().map(|(_, name)| *name).collect()
    }
}

// LanguageCompatibility is now defined in memory_traits.rs and re-exported above.

// ============================================================================
// Memoria Transferible
// ============================================================================

/// Motor de memoria con soporte para transferencia de conocimiento.
pub struct TransferableMemory {
    /// Memoria base
    memory: AgentMemory,
    /// Contexto del proyecto actual
    current_context: RwLock<Option<ProjectContext>>,
    /// Extractor de conceptos
    extractor: ConceptExtractor,
    /// Peso de relevancia semántica vs transferibilidad
    relevance_weight: f32,
    /// Umbral mínimo de transferibilidad
    transfer_threshold: f32,
}

impl TransferableMemory {
    /// Crea una nueva memoria transferible con configuración por defecto.
    pub fn new(config: MemoryConfig) -> Result<Self> {
        Ok(Self {
            memory: AgentMemory::new(config)?,
            current_context: RwLock::new(None),
            extractor: ConceptExtractor::new(),
            relevance_weight: 0.6,
            transfer_threshold: 0.3,
        })
    }

    /// Crea desde una memoria existente.
    pub fn from_memory(memory: AgentMemory) -> Self {
        Self {
            memory,
            current_context: RwLock::new(None),
            extractor: ConceptExtractor::new(),
            relevance_weight: 0.6,
            transfer_threshold: 0.3,
        }
    }

    /// Configura el peso de relevancia (0.0 - 1.0).
    /// El resto es el peso de transferibilidad.
    pub fn with_relevance_weight(mut self, weight: f32) -> Self {
        self.relevance_weight = weight.clamp(0.0, 1.0);
        self
    }

    /// Configura el umbral mínimo de transferibilidad.
    pub fn with_transfer_threshold(mut self, threshold: f32) -> Self {
        self.transfer_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Establece la función de embedding.
    pub fn set_embed_fn<F>(&mut self, f: F)
    where
        F: Fn(&str) -> Vec<f32> + Send + Sync + 'static,
    {
        self.memory.set_embed_fn(f);
    }

    /// Establece el contexto del proyecto actual.
    pub fn set_project_context(&self, context: ProjectContext) {
        self.memory.with_working_context(|ctx| {
            ctx.set_project(&context.name);
        });
        *self.current_context.write() = Some(context);
    }

    /// Obtiene el contexto del proyecto actual.
    pub fn project_context(&self) -> Option<ProjectContext> {
        self.current_context.read().clone()
    }

    /// Limpia el contexto del proyecto.
    pub fn clear_project_context(&self) {
        *self.current_context.write() = None;
    }

    /// Acceso a la memoria base.
    pub fn memory(&self) -> &AgentMemory {
        &self.memory
    }

    // ========================================================================
    // Aprendizaje con Transferencia
    // ========================================================================

    /// Aprende una tarea con metadata de transferibilidad.
    pub fn learn_task_transferable(
        &self,
        task: &str,
        code: &str,
        outcome: TaskOutcome,
        learnings: Vec<&str>,
        transfer_level: Option<TransferLevel>,
        domain: Option<KnowledgeDomain>,
    ) -> Result<VectorId> {
        // Extraer conceptos
        let concepts = self.extractor.extract(task, code);

        // Inferir nivel de transferencia si no se proporciona
        let level =
            transfer_level.unwrap_or_else(|| self.extractor.infer_transfer_level(&concepts));

        // Inferir dominio si no se proporciona
        let domain = domain.unwrap_or_else(|| {
            self.current_context
                .read()
                .as_ref()
                .map(|c| c.domain.clone())
                .unwrap_or(KnowledgeDomain::General)
        });

        // Crear episodio enriquecido
        let mut episode = TaskEpisode {
            task: task.to_string(),
            code: code.to_string(),
            outcome,
            steps: Vec::new(),
            learnings: learnings.iter().map(|s| s.to_string()).collect(),
            errors: Vec::new(),
            language: self
                .current_context
                .read()
                .as_ref()
                .map(|c| Language::from_str(&c.language))
                .unwrap_or(Language::Other("unknown".into())),
            project: self.current_context.read().as_ref().map(|c| c.name.clone()),
            duration_secs: None,
            tags: Vec::new(),
        };

        // Añadir tags de transferencia
        episode.tags.push(format!("transfer:{}", level.as_str()));
        episode.tags.push(format!("domain:{}", domain.as_str()));
        for concept in &concepts {
            episode.tags.push(format!(
                "concept:{}",
                concept.to_lowercase().replace(' ', "_")
            ));
        }

        // Guardar
        self.memory.learn_episode(episode)
    }

    /// Aprende un snippet de código con transferibilidad.
    pub fn learn_code_transferable(
        &self,
        snippet: CodeSnippet,
        transfer_level: Option<TransferLevel>,
        domain: Option<KnowledgeDomain>,
    ) -> Result<VectorId> {
        let concepts = self.extractor.extract(&snippet.description, &snippet.code);
        let level =
            transfer_level.unwrap_or_else(|| self.extractor.infer_transfer_level(&concepts));
        let domain = domain.unwrap_or(KnowledgeDomain::General);

        // Crear snippet enriquecido
        let mut enriched = snippet;
        enriched.tags.push(format!("transfer:{}", level.as_str()));
        enriched.tags.push(format!("domain:{}", domain.as_str()));
        for concept in &concepts {
            enriched.tags.push(format!(
                "concept:{}",
                concept.to_lowercase().replace(' ', "_")
            ));
        }

        self.memory.learn_code(enriched)
    }

    // ========================================================================
    // Búsqueda con Transferencia
    // ========================================================================

    /// Busca conocimiento considerando transferibilidad al contexto actual.
    pub fn recall_transferable(&self, query: &str, k: usize) -> Result<Vec<TransferableRecall>> {
        // Búsqueda amplia
        let all_results = self.memory.recall_similar(query, k * 3)?;

        // Calcular transferibilidad y rankear
        let mut ranked: Vec<TransferableRecall> = all_results
            .into_iter()
            .map(|recall| {
                let concepts = self.extract_concepts_from_recall(&recall);
                let transfer_level = self.infer_level_from_recall(&recall, &concepts);
                let transfer_score = self.calculate_transfer_score(&recall, &concepts);
                let combined_score = recall.relevance_score * self.relevance_weight
                    + transfer_score * (1.0 - self.relevance_weight);

                TransferableRecall {
                    adaptation_needed: self.get_adaptation_needed(&recall),
                    applicable: transfer_score >= self.transfer_threshold,
                    recall,
                    transfer_score,
                    combined_score,
                    transfer_level,
                    concepts,
                }
            })
            .collect();

        // Ordenar por score combinado
        ranked.sort_by(|a, b| {
            b.combined_score
                .partial_cmp(&a.combined_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Limitar resultados
        ranked.truncate(k);

        Ok(ranked)
    }

    /// Busca solo conocimiento universal (siempre aplicable).
    pub fn recall_universal(&self, query: &str, k: usize) -> Result<Vec<TransferableRecall>> {
        let results = self.recall_transferable(query, k * 2)?;
        Ok(results
            .into_iter()
            .filter(|r| r.transfer_level == TransferLevel::Universal)
            .take(k)
            .collect())
    }

    /// Busca conocimiento del mismo dominio.
    pub fn recall_same_domain(&self, query: &str, k: usize) -> Result<Vec<TransferableRecall>> {
        let current_domain = self
            .current_context
            .read()
            .as_ref()
            .map(|c| c.domain.clone());

        let results = self.recall_transferable(query, k * 2)?;

        Ok(results
            .into_iter()
            .filter(|r| {
                if let Some(ref domain) = current_domain {
                    self.is_domain_compatible(&r.recall, domain)
                } else {
                    true
                }
            })
            .take(k)
            .collect())
    }

    /// Busca conocimiento del mismo stack tecnológico.
    pub fn recall_same_stack(&self, query: &str, k: usize) -> Result<Vec<TransferableRecall>> {
        let current_lang = self
            .current_context
            .read()
            .as_ref()
            .map(|c| c.language.clone());

        let results = self.recall_transferable(query, k * 2)?;

        Ok(results
            .into_iter()
            .filter(|r| {
                if let Some(ref lang) = current_lang {
                    self.is_language_compatible(&r.recall, lang)
                } else {
                    true
                }
            })
            .take(k)
            .collect())
    }

    // ========================================================================
    // Métodos Internos
    // ========================================================================

    fn extract_concepts_from_recall(&self, recall: &MemoryRecall) -> Vec<String> {
        let mut concepts = Vec::new();

        // Extraer de tags
        if let Some(ref meta) = recall.metadata {
            if let Some(MetadataValue::String(tags)) = meta.get("tags") {
                for tag in tags.split(',') {
                    if tag.starts_with("concept:") {
                        concepts.push(tag.trim_start_matches("concept:").replace('_', " "));
                    }
                }
            }
        }

        // Si no hay tags, extraer del contenido
        if concepts.is_empty() {
            concepts = self.extractor.extract(&recall.content, "");
        }

        concepts
    }

    fn infer_level_from_recall(&self, recall: &MemoryRecall, concepts: &[String]) -> TransferLevel {
        // Primero buscar en tags
        if let Some(ref meta) = recall.metadata {
            if let Some(MetadataValue::String(tags)) = meta.get("tags") {
                for tag in tags.split(',') {
                    if tag.starts_with("transfer:") {
                        return TransferLevel::from_str(tag.trim_start_matches("transfer:"))
                            .unwrap_or(TransferLevel::Instance);
                    }
                }
            }
        }

        // Inferir de conceptos
        self.extractor.infer_transfer_level(concepts)
    }

    fn calculate_transfer_score(&self, recall: &MemoryRecall, concepts: &[String]) -> f32 {
        let current = self.current_context.read();
        let Some(ref ctx) = *current else {
            return 0.5; // Sin contexto, asumir medio
        };

        let mut score = 0.0;

        // 1. Nivel de transferencia base (30%)
        let level = self.infer_level_from_recall(recall, concepts);
        score += level.transfer_score() * 0.3;

        // 2. Compatibilidad de dominio (25%)
        if self.is_domain_compatible(recall, &ctx.domain) {
            score += 0.25;
        } else if self.is_related_domain(recall, &ctx.domain) {
            score += 0.12;
        }

        // 3. Compatibilidad de lenguaje (30%)
        let lang_compat = self.get_language_compatibility(recall, &ctx.language);
        score += lang_compat * 0.3;

        // 4. Conceptos compartidos (15%)
        let concept_overlap = self.calculate_concept_overlap(concepts, ctx);
        score += concept_overlap * 0.15;

        score.min(1.0)
    }

    fn is_domain_compatible(&self, recall: &MemoryRecall, domain: &KnowledgeDomain) -> bool {
        if let Some(ref meta) = recall.metadata {
            if let Some(MetadataValue::String(tags)) = meta.get("tags") {
                let domain_tag = format!("domain:{}", domain.as_str());
                return tags.contains(&domain_tag);
            }
        }
        false
    }

    fn is_related_domain(&self, recall: &MemoryRecall, domain: &KnowledgeDomain) -> bool {
        let related = domain.related_domains();
        for rel_domain in related {
            if self.is_domain_compatible(recall, &rel_domain) {
                return true;
            }
        }
        false
    }

    fn get_language_compatibility(&self, recall: &MemoryRecall, current_lang: &str) -> f32 {
        if let Some(ref meta) = recall.metadata {
            if let Some(MetadataValue::String(lang)) = meta.get("language") {
                return LanguageCompatibility::compatibility(lang, current_lang);
            }
        }
        0.5 // Desconocido
    }

    fn is_language_compatible(&self, recall: &MemoryRecall, current_lang: &str) -> bool {
        self.get_language_compatibility(recall, current_lang) >= 0.6
    }

    fn calculate_concept_overlap(&self, concepts: &[String], ctx: &ProjectContext) -> f32 {
        if concepts.is_empty() {
            return 0.0;
        }

        let matches = concepts
            .iter()
            .filter(|c| {
                ctx.patterns.iter().any(|p| {
                    p.to_lowercase().contains(&c.to_lowercase())
                        || c.to_lowercase().contains(&p.to_lowercase())
                })
            })
            .count();

        matches as f32 / concepts.len().max(ctx.patterns.len()).max(1) as f32
    }

    fn get_adaptation_needed(&self, recall: &MemoryRecall) -> Option<String> {
        let current = self.current_context.read();
        let Some(ref ctx) = *current else {
            return None;
        };

        if let Some(ref meta) = recall.metadata {
            if let Some(MetadataValue::String(source_lang)) = meta.get("language") {
                let target_lang = ctx.language.as_str();
                return LanguageCompatibility::adaptation_description(source_lang, target_lang);
            }
        }

        None
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transfer_level_ordering() {
        assert!(TransferLevel::Universal > TransferLevel::Domain);
        assert!(TransferLevel::Domain > TransferLevel::Context);
        assert!(TransferLevel::Context > TransferLevel::Instance);
    }

    #[test]
    fn test_transfer_level_score() {
        assert!((TransferLevel::Universal.transfer_score() - 1.0).abs() < 0.01);
        assert!((TransferLevel::Instance.transfer_score() - 0.25).abs() < 0.01);
    }

    #[test]
    fn test_language_compatibility() {
        // Mismo lenguaje
        assert!((LanguageCompatibility::compatibility("rust", "rust") - 1.0).abs() < 0.01);

        // Mismo grupo
        assert!(LanguageCompatibility::compatibility("typescript", "javascript") > 0.5);
        assert!(LanguageCompatibility::compatibility("python", "ruby") > 0.5);

        // Diferentes grupos
        assert!(LanguageCompatibility::compatibility("rust", "python") < 0.5);
    }

    #[test]
    fn test_concept_extraction() {
        let extractor = ConceptExtractor::new();

        let concepts = extractor.extract(
            "Implement JWT authentication with rate limiting",
            "middleware auth jwt token verify",
        );

        assert!(concepts.contains(&"JWT Tokens".to_string()));
        assert!(concepts.contains(&"Authentication".to_string()));
        assert!(concepts.contains(&"Rate Limiting".to_string()));
        assert!(concepts.contains(&"Middleware Pattern".to_string()));
    }

    #[test]
    fn test_infer_transfer_level() {
        let extractor = ConceptExtractor::new();

        // Con principios = Universal
        let concepts = vec!["SOLID Principles".to_string()];
        assert_eq!(
            extractor.infer_transfer_level(&concepts),
            TransferLevel::Universal
        );

        // Con patrones = Domain
        let concepts = vec!["Factory Pattern".to_string()];
        assert_eq!(
            extractor.infer_transfer_level(&concepts),
            TransferLevel::Domain
        );

        // Con conceptos de dominio = Domain
        let concepts = vec!["Authentication".to_string()];
        assert_eq!(
            extractor.infer_transfer_level(&concepts),
            TransferLevel::Domain
        );

        // Sin conceptos = Stack
        let concepts: Vec<String> = vec![];
        assert_eq!(
            extractor.infer_transfer_level(&concepts),
            TransferLevel::Context
        );
    }

    #[test]
    fn test_domain_related() {
        let web = KnowledgeDomain::WebBackend;
        let related = web.related_domains();

        assert!(related.contains(&KnowledgeDomain::Database));
        assert!(related.contains(&KnowledgeDomain::Security));
        assert!(!related.contains(&KnowledgeDomain::GameDev));
    }

    #[test]
    fn test_project_context_builder() {
        let ctx = ProjectContext::new("my-api", "rust", KnowledgeDomain::WebBackend)
            .with_frameworks(vec!["Axum".into(), "SQLx".into()])
            .with_patterns(vec!["REST".into(), "Clean Architecture".into()]);

        assert_eq!(ctx.name, "my-api");
        assert_eq!(ctx.frameworks.len(), 2);
        assert_eq!(ctx.patterns.len(), 2);
    }

    #[test]
    fn test_transferable_memory_creation() {
        let config = MemoryConfig::small();
        let memory = TransferableMemory::new(config);
        assert!(memory.is_ok());
    }

    #[test]
    fn test_set_project_context() {
        let config = MemoryConfig::small();
        let memory = TransferableMemory::new(config).unwrap();

        assert!(memory.project_context().is_none());

        memory.set_project_context(ProjectContext::new(
            "test-project",
            "rust",
            KnowledgeDomain::CLI,
        ));

        let ctx = memory.project_context();
        assert!(ctx.is_some());
        assert_eq!(ctx.unwrap().name, "test-project");
    }
}
