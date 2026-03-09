//! # Memoria Agéntica para Desarrollo de Código
//!
//! Sistema de memoria diseñado para agentes de IA que desarrollan código.
//! Proporciona memoria semántica, episódica y de trabajo.
//!
//! ## Tipos de Memoria
//!
//! - **Semántica**: Conocimiento general (APIs, patrones, documentación)
//! - **Episódica**: Experiencias pasadas (tareas completadas, errores, soluciones)
//! - **Working**: Contexto actual (proyecto, archivos abiertos, goals)
//!
//! ## Ejemplo Básico
//!
//! ```rust,ignore
//! use minimemory::agent_memory::{AgentMemory, MemoryConfig, TaskOutcome};
//!
//! // Crear memoria del agente
//! let mut memory = AgentMemory::new(MemoryConfig::default()).unwrap();
//!
//! // Aprender de una tarea completada
//! memory.learn_task(
//!     "Implementar autenticación JWT",
//!     "fn verify_token(token: &str) -> Result<Claims>...",
//!     TaskOutcome::Success,
//!     vec!["Usar jsonwebtoken crate", "Validar expiration"]
//! ).unwrap();
//!
//! // Recordar experiencias similares
//! let experiences = memory.recall_similar("autenticación de usuarios", 5).unwrap();
//! ```

use std::collections::HashMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::memory_traits::presets::SoftwareDevelopment;
use crate::memory_traits::GenericMemory;
use crate::partial_index::PartialIndexConfig;
use crate::query::Filter;
use crate::replication::ChangeLog;
use crate::search::HybridSearchParams;
use crate::types::{Metadata, SearchResult, VectorId};
use crate::Config;
use crate::VectorDB;

// ============================================================================
// Tipos de Memoria
// ============================================================================

/// Resultado de una tarea
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskOutcome {
    /// Tarea completada exitosamente
    Success,
    /// Tarea falló
    Failure,
    /// Tarea parcialmente completada
    Partial,
    /// Tarea cancelada
    Cancelled,
}

impl TaskOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskOutcome::Success => "success",
            TaskOutcome::Failure => "failure",
            TaskOutcome::Partial => "partial",
            TaskOutcome::Cancelled => "cancelled",
        }
    }
}

/// Tipo de entrada de memoria
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryType {
    /// Experiencia de tarea (episódica)
    Episode,
    /// Snippet de código aprendido
    CodeSnippet,
    /// Conocimiento de API/librería
    ApiKnowledge,
    /// Patrón de código
    Pattern,
    /// Error y su solución
    ErrorSolution,
    /// Documentación
    Documentation,
    /// Contexto de proyecto
    ProjectContext,
}

impl MemoryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryType::Episode => "episode",
            MemoryType::CodeSnippet => "code_snippet",
            MemoryType::ApiKnowledge => "api_knowledge",
            MemoryType::Pattern => "pattern",
            MemoryType::ErrorSolution => "error_solution",
            MemoryType::Documentation => "documentation",
            MemoryType::ProjectContext => "project_context",
        }
    }
}

/// Lenguaje de programación
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Java,
    CSharp,
    Cpp,
    Other(String),
}

impl Language {
    pub fn as_str(&self) -> &str {
        match self {
            Language::Rust => "rust",
            Language::Python => "python",
            Language::JavaScript => "javascript",
            Language::TypeScript => "typescript",
            Language::Go => "go",
            Language::Java => "java",
            Language::CSharp => "csharp",
            Language::Cpp => "cpp",
            Language::Other(s) => s,
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "rust" | "rs" => Language::Rust,
            "python" | "py" => Language::Python,
            "javascript" | "js" => Language::JavaScript,
            "typescript" | "ts" => Language::TypeScript,
            "go" | "golang" => Language::Go,
            "java" => Language::Java,
            "csharp" | "c#" | "cs" => Language::CSharp,
            "cpp" | "c++" => Language::Cpp,
            other => Language::Other(other.to_string()),
        }
    }
}

// ============================================================================
// Estructuras de Entrada
// ============================================================================

/// Episodio de tarea completada
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEpisode {
    /// Descripción de la tarea
    pub task: String,
    /// Código escrito/modificado
    pub code: String,
    /// Resultado de la tarea
    pub outcome: TaskOutcome,
    /// Pasos tomados
    pub steps: Vec<String>,
    /// Aprendizajes extraídos
    pub learnings: Vec<String>,
    /// Errores encontrados
    pub errors: Vec<String>,
    /// Lenguaje principal
    pub language: Language,
    /// Proyecto asociado
    pub project: Option<String>,
    /// Duración en segundos
    pub duration_secs: Option<u64>,
    /// Tags adicionales
    pub tags: Vec<String>,
}

/// Snippet de código aprendido
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSnippet {
    /// Código fuente
    pub code: String,
    /// Descripción/propósito
    pub description: String,
    /// Lenguaje
    pub language: Language,
    /// Dependencias requeridas
    pub dependencies: Vec<String>,
    /// Caso de uso
    pub use_case: String,
    /// Calidad estimada (0-1)
    pub quality_score: f32,
    /// Tags
    pub tags: Vec<String>,
}

/// Conocimiento de API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKnowledge {
    /// Nombre de la librería/API
    pub library: String,
    /// Función/método
    pub function: String,
    /// Descripción
    pub description: String,
    /// Ejemplo de uso
    pub example: String,
    /// Parámetros
    pub parameters: Vec<String>,
    /// Versión
    pub version: Option<String>,
}

/// Error y su solución
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorSolution {
    /// Mensaje de error
    pub error_message: String,
    /// Tipo de error
    pub error_type: String,
    /// Causa raíz
    pub root_cause: String,
    /// Solución aplicada
    pub solution: String,
    /// Código corregido
    pub fixed_code: Option<String>,
    /// Lenguaje
    pub language: Language,
}

// ============================================================================
// Working Memory (Contexto Actual)
// ============================================================================

/// Contexto de trabajo actual del agente
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkingContext {
    /// Proyecto actual
    pub current_project: Option<String>,
    /// Archivos abiertos/modificados
    pub open_files: Vec<String>,
    /// Tarea actual
    pub current_task: Option<String>,
    /// Goals activos
    pub active_goals: Vec<String>,
    /// Variables de contexto
    pub variables: HashMap<String, String>,
    /// Historial de conversación reciente
    pub conversation_history: Vec<ConversationTurn>,
    /// Errores recientes
    pub recent_errors: Vec<String>,
}

/// Turno de conversación
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    /// Rol (user/assistant)
    pub role: String,
    /// Contenido
    pub content: String,
    /// Timestamp
    pub timestamp: u64,
}

impl WorkingContext {
    pub fn new() -> Self {
        Self::default()
    }

    /// Establece el proyecto actual
    pub fn set_project(&mut self, project: impl Into<String>) {
        self.current_project = Some(project.into());
    }

    /// Establece la tarea actual
    pub fn set_task(&mut self, task: impl Into<String>) {
        self.current_task = Some(task.into());
    }

    /// Añade un archivo abierto
    pub fn add_open_file(&mut self, file: impl Into<String>) {
        let file = file.into();
        if !self.open_files.contains(&file) {
            self.open_files.push(file);
        }
    }

    /// Añade un goal
    pub fn add_goal(&mut self, goal: impl Into<String>) {
        self.active_goals.push(goal.into());
    }

    /// Completa un goal
    pub fn complete_goal(&mut self, goal: &str) {
        self.active_goals.retain(|g| g != goal);
    }

    /// Añade turno de conversación
    pub fn add_conversation(&mut self, role: &str, content: &str) {
        self.conversation_history.push(ConversationTurn {
            role: role.to_string(),
            content: content.to_string(),
            timestamp: current_timestamp(),
        });
        // Mantener solo los últimos 20 turnos
        if self.conversation_history.len() > 20 {
            self.conversation_history.remove(0);
        }
    }

    /// Registra un error reciente
    pub fn add_error(&mut self, error: impl Into<String>) {
        self.recent_errors.push(error.into());
        if self.recent_errors.len() > 10 {
            self.recent_errors.remove(0);
        }
    }

    /// Limpia el contexto
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Serializa el contexto a texto para embedding
    pub fn to_context_string(&self) -> String {
        let mut parts = Vec::new();

        if let Some(ref project) = self.current_project {
            parts.push(format!("Project: {}", project));
        }
        if let Some(ref task) = self.current_task {
            parts.push(format!("Task: {}", task));
        }
        if !self.active_goals.is_empty() {
            parts.push(format!("Goals: {}", self.active_goals.join(", ")));
        }
        if !self.open_files.is_empty() {
            parts.push(format!("Files: {}", self.open_files.join(", ")));
        }

        parts.join("\n")
    }
}

// ============================================================================
// Configuración
// ============================================================================

/// Configuración de memoria del agente
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    /// Dimensiones del embedding
    pub embedding_dimensions: usize,
    /// Campos para BM25
    pub indexed_fields: Vec<String>,
    /// Usar HNSW para índice principal
    pub use_hnsw: bool,
    /// Parámetros HNSW
    pub hnsw_m: usize,
    pub hnsw_ef: usize,
    /// Máximo de episodios a mantener
    pub max_episodes: usize,
    /// Habilitar change log
    pub enable_changelog: bool,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            embedding_dimensions: 1536, // OpenAI ada-002
            indexed_fields: vec![
                "task".into(),
                "code".into(),
                "description".into(),
                "learnings".into(),
                "error_message".into(),
                "solution".into(),
            ],
            use_hnsw: true,
            hnsw_m: 16,
            hnsw_ef: 200,
            max_episodes: 10000,
            enable_changelog: true,
        }
    }
}

impl MemoryConfig {
    pub fn new(embedding_dimensions: usize) -> Self {
        Self {
            embedding_dimensions,
            ..Default::default()
        }
    }

    /// Configuración para modelos pequeños (384 dims)
    pub fn small() -> Self {
        Self {
            embedding_dimensions: 384,
            ..Default::default()
        }
    }

    /// Configuración para OpenAI (1536 dims)
    pub fn openai() -> Self {
        Self::default()
    }

    /// Configuración para modelos grandes (4096 dims)
    pub fn large() -> Self {
        Self {
            embedding_dimensions: 4096,
            ..Default::default()
        }
    }

    /// Configuración para all-MiniLM-L6-v2 (384 dims, ligero).
    ///
    /// Requiere feature `embeddings`.
    #[cfg(feature = "embeddings")]
    pub fn minilm() -> Self {
        Self {
            embedding_dimensions: 384,
            ..Default::default()
        }
    }

    /// Configuración para BGE-small-en-v1.5 (384 dims, alta calidad inglés).
    ///
    /// Requiere feature `embeddings`.
    #[cfg(feature = "embeddings")]
    pub fn bge_small() -> Self {
        Self {
            embedding_dimensions: 384,
            ..Default::default()
        }
    }

    /// Configuración para EmbeddingGemma (multilingüe, Matryoshka).
    ///
    /// # Argumentos
    ///
    /// * `dimensions` - Dimensiones de salida: 768 (full), 512, 256, o 128
    ///
    /// Requiere feature `embeddings`.
    #[cfg(feature = "embeddings")]
    pub fn gemma(dimensions: usize) -> Self {
        Self {
            embedding_dimensions: dimensions,
            ..Default::default()
        }
    }
}

// ============================================================================
// Resultado de Recall
// ============================================================================

/// Resultado de búsqueda en memoria
#[derive(Debug, Clone)]
pub struct MemoryRecall {
    /// ID de la entrada
    pub id: VectorId,
    /// Tipo de memoria
    pub memory_type: MemoryType,
    /// Score de relevancia (menor = más relevante)
    pub relevance_score: f32,
    /// Contenido principal
    pub content: String,
    /// Metadata adicional
    pub metadata: Option<Metadata>,
}

/// Estadísticas de la memoria
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    /// Total de entradas
    pub total_entries: usize,
    /// Episodios
    pub episodes: usize,
    /// Snippets de código
    pub code_snippets: usize,
    /// Conocimiento de APIs
    pub api_knowledge: usize,
    /// Soluciones de errores
    pub error_solutions: usize,
    /// Patrones
    pub patterns: usize,
    /// Proyectos indexados
    pub projects: Vec<String>,
}

// ============================================================================
// Memoria Principal del Agente
// ============================================================================

/// Sistema de memoria para agentes de desarrollo de código.
///
/// Internamente usa `GenericMemory<SoftwareDevelopment>` para heredar
/// prioridad automática, decay temporal, usage stats y transfer scoring.
/// Mantiene su propia API (learn_task, learn_code, etc.) como facade.
pub struct AgentMemory {
    /// Sistema de memoria genérico (provee priority, decay, usage stats)
    inner: GenericMemory<SoftwareDevelopment>,
    /// Contexto de trabajo actual
    working: RwLock<WorkingContext>,
    /// Change log para replicación
    changelog: Option<ChangeLog>,
    /// Función de embedding (opcional, para uso externo)
    embed_fn: Option<Box<dyn Fn(&str) -> Vec<f32> + Send + Sync>>,
}

impl AgentMemory {
    /// Crea una nueva memoria de agente
    pub fn new(config: MemoryConfig) -> Result<Self> {
        let db_config = if config.use_hnsw {
            Config::new(config.embedding_dimensions).with_index(crate::IndexType::HNSW {
                m: config.hnsw_m,
                ef_construction: config.hnsw_ef,
            })
        } else {
            Config::new(config.embedding_dimensions)
        };

        let db = VectorDB::with_fulltext(db_config, config.indexed_fields.clone())?;
        let inner = GenericMemory::<SoftwareDevelopment>::with_db(db);

        let changelog = if config.enable_changelog {
            Some(ChangeLog::with_instance_id("agent-memory"))
        } else {
            None
        };

        Ok(Self {
            inner,
            working: RwLock::new(WorkingContext::new()),
            changelog,
            embed_fn: None,
        })
    }

    /// Carga memoria desde archivo
    pub fn load<P: AsRef<Path>>(path: P, config: MemoryConfig) -> Result<Self> {
        let db = VectorDB::open_with_fulltext(path, config.indexed_fields.clone())?;
        let inner = GenericMemory::<SoftwareDevelopment>::with_db(db);

        let changelog = if config.enable_changelog {
            Some(ChangeLog::with_instance_id("agent-memory"))
        } else {
            None
        };

        // Restore WorkingContext from special document
        let working = if let Some((_, Some(meta))) = inner.db().get("__working_context__")? {
            if let Some(crate::MetadataValue::String(json)) = meta.get("__data__") {
                serde_json::from_str(json).unwrap_or_default()
            } else {
                WorkingContext::new()
            }
        } else {
            WorkingContext::new()
        };

        Ok(Self {
            inner,
            working: RwLock::new(working),
            changelog,
            embed_fn: None,
        })
    }

    /// Guarda memoria a archivo
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        // Persist WorkingContext as a special metadata-only document
        let ctx = self.working.read().clone();
        if let Ok(json) = serde_json::to_string(&ctx) {
            let mut meta = Metadata::new();
            meta.insert("__data__", json.as_str());
            meta.insert("type", "__internal__");

            // Remove old context doc if exists, then insert new one
            let _ = self.db().delete("__working_context__");
            let _ = self.db().insert_document("__working_context__", None, Some(meta));
        }

        self.db().save(path)
    }

    /// Crea AgentMemory con embeddings locales usando un modelo de HuggingFace.
    ///
    /// Descarga el modelo automáticamente (cacheado en `~/.cache/huggingface/`).
    /// No requiere API key ni conexión después de la primera descarga.
    ///
    /// # Ejemplo
    ///
    /// ```rust,ignore
    /// use minimemory::agent_memory::AgentMemory;
    /// use minimemory::embeddings::EmbeddingModel;
    ///
    /// // Modelo ligero para inglés
    /// let memory = AgentMemory::with_local_embeddings(EmbeddingModel::MiniLM)?;
    ///
    /// // Modelo multilingüe con dimensiones reducidas
    /// let memory = AgentMemory::with_local_embeddings(
    ///     EmbeddingModel::Gemma { dimensions: 256 }
    /// )?;
    /// ```
    #[cfg(feature = "embeddings")]
    pub fn with_local_embeddings(
        model: crate::embeddings::EmbeddingModel,
    ) -> Result<Self> {
        let config = MemoryConfig::new(model.dimensions());
        Self::with_local_embeddings_config(model, config)
    }

    /// Crea AgentMemory con embeddings locales y configuración personalizada.
    #[cfg(feature = "embeddings")]
    pub fn with_local_embeddings_config(
        model: crate::embeddings::EmbeddingModel,
        mut config: MemoryConfig,
    ) -> Result<Self> {
        config.embedding_dimensions = model.dimensions();
        let mut memory = Self::new(config)?;

        let embedder = crate::embeddings::Embedder::new(model)?;
        memory.embed_fn = Some(Box::new(embedder.into_embed_fn()));

        Ok(memory)
    }

    /// Establece la función de embedding
    pub fn set_embed_fn<F>(&mut self, f: F)
    where
        F: Fn(&str) -> Vec<f32> + Send + Sync + 'static,
    {
        self.embed_fn = Some(Box::new(f));
    }

    /// Genera embedding usando la función externa configurada.
    ///
    /// Retorna error si no se ha configurado `embed_fn` via `set_embed_fn()`.
    fn embed(&self, text: &str) -> Result<Vec<f32>> {
        if let Some(ref f) = self.embed_fn {
            Ok(f(text))
        } else {
            Err(crate::error::Error::InvalidConfig(
                "No embedding function set. Call set_embed_fn() first".into(),
            ))
        }
    }

    // ========================================================================
    // Working Memory
    // ========================================================================

    /// Obtiene referencia al contexto de trabajo
    pub fn working_context(&self) -> impl std::ops::Deref<Target = WorkingContext> + '_ {
        self.working.read()
    }

    /// Modifica el contexto de trabajo
    pub fn with_working_context<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut WorkingContext) -> R,
    {
        f(&mut self.working.write())
    }

    /// Establece el proyecto actual y crea índice parcial
    pub fn focus_project(&self, project: &str) -> Result<()> {
        self.working.write().set_project(project);

        // Crear índice parcial para el proyecto
        let index_name = format!("project_{}", project.replace(['/', '\\', ' '], "_"));
        if !self.db().has_partial_index(&index_name) {
            self.db().create_partial_index(
                &index_name,
                PartialIndexConfig::new(Filter::eq("project", project)),
            )?;
        }

        Ok(())
    }

    // ========================================================================
    // Learning (Almacenar Memorias)
    // ========================================================================

    /// Aprende de una tarea completada
    pub fn learn_task(
        &self,
        task: &str,
        code: &str,
        outcome: TaskOutcome,
        learnings: Vec<&str>,
    ) -> Result<VectorId> {
        self.learn_episode(TaskEpisode {
            task: task.to_string(),
            code: code.to_string(),
            outcome,
            steps: Vec::new(),
            learnings: learnings.into_iter().map(String::from).collect(),
            errors: Vec::new(),
            language: Language::Other("unknown".into()),
            project: self.working.read().current_project.clone(),
            duration_secs: None,
            tags: Vec::new(),
        })
    }

    /// Aprende un episodio completo
    pub fn learn_episode(&self, episode: TaskEpisode) -> Result<VectorId> {
        let id = format!("episode-{}", generate_id());

        let mut meta = Metadata::new();
        meta.insert("type", MemoryType::Episode.as_str());
        meta.insert("task", episode.task.as_str());
        meta.insert("code", episode.code.as_str());
        meta.insert("outcome", episode.outcome.as_str());
        meta.insert("language", episode.language.as_str());
        meta.insert("learnings", episode.learnings.join("\n"));
        meta.insert("description", episode.task.as_str());

        if let Some(ref project) = episode.project {
            meta.insert("project", project.as_str());
        }
        if !episode.tags.is_empty() {
            meta.insert("tags", episode.tags.join(","));
        }
        if !episode.errors.is_empty() {
            meta.insert("errors", episode.errors.join("\n"));
        }

        // Crear texto para embedding
        let embed_text = format!(
            "{}\n{}\n{}",
            episode.task,
            episode.code,
            episode.learnings.join("\n")
        );
        let embedding = self.embed(&embed_text)?;

        // Delegate to GenericMemory for priority, decay, usage stats, transfer level
        self.inner.learn_raw(&id, &embedding, meta, &embed_text)?;

        if let Some(ref log) = self.changelog {
            log.track_insert(&id, &embedding, None);
        }

        Ok(id)
    }

    /// Aprende un snippet de código
    pub fn learn_code(&self, snippet: CodeSnippet) -> Result<VectorId> {
        let id = format!("code-{}", generate_id());

        let mut meta = Metadata::new();
        meta.insert("type", MemoryType::CodeSnippet.as_str());
        meta.insert("code", snippet.code.as_str());
        meta.insert("description", snippet.description.as_str());
        meta.insert("language", snippet.language.as_str());
        meta.insert("use_case", snippet.use_case.as_str());
        meta.insert("quality", snippet.quality_score as f64);

        if !snippet.dependencies.is_empty() {
            meta.insert("dependencies", snippet.dependencies.join(","));
        }
        if !snippet.tags.is_empty() {
            meta.insert("tags", snippet.tags.join(","));
        }

        if let Some(ref project) = self.working.read().current_project {
            meta.insert("project", project.as_str());
        }

        let embed_text = format!(
            "{}\n{}\n{}",
            snippet.description, snippet.code, snippet.use_case
        );
        let embedding = self.embed(&embed_text)?;

        self.inner.learn_raw(&id, &embedding, meta, &embed_text)?;

        if let Some(ref log) = self.changelog {
            log.track_insert(&id, &embedding, None);
        }

        Ok(id)
    }

    /// Aprende conocimiento de API
    pub fn learn_api(&self, api: ApiKnowledge) -> Result<VectorId> {
        let id = format!("api-{}", generate_id());

        let mut meta = Metadata::new();
        meta.insert("type", MemoryType::ApiKnowledge.as_str());
        meta.insert("library", api.library.as_str());
        meta.insert("function", api.function.as_str());
        meta.insert("description", api.description.as_str());
        meta.insert("code", api.example.as_str());

        if let Some(ref version) = api.version {
            meta.insert("version", version.as_str());
        }

        let embed_text = format!(
            "{} {} {}\n{}",
            api.library, api.function, api.description, api.example
        );
        let embedding = self.embed(&embed_text)?;

        self.inner.learn_raw(&id, &embedding, meta, &embed_text)?;

        if let Some(ref log) = self.changelog {
            log.track_insert(&id, &embedding, None);
        }

        Ok(id)
    }

    /// Aprende solución a un error
    pub fn learn_error_solution(&self, error: ErrorSolution) -> Result<VectorId> {
        let id = format!("error-{}", generate_id());

        let mut meta = Metadata::new();
        meta.insert("type", MemoryType::ErrorSolution.as_str());
        meta.insert("error_message", error.error_message.as_str());
        meta.insert("error_type", error.error_type.as_str());
        meta.insert("description", error.error_message.as_str());
        meta.insert("solution", error.solution.as_str());
        meta.insert("language", error.language.as_str());

        if let Some(ref code) = error.fixed_code {
            meta.insert("code", code.as_str());
        }

        if let Some(ref project) = self.working.read().current_project {
            meta.insert("project", project.as_str());
        }

        let embed_text = format!(
            "{}\n{}\n{}",
            error.error_message, error.root_cause, error.solution
        );
        let embedding = self.embed(&embed_text)?;

        self.inner.learn_raw(&id, &embedding, meta, &embed_text)?;

        if let Some(ref log) = self.changelog {
            log.track_insert(&id, &embedding, None);
        }

        Ok(id)
    }

    // ========================================================================
    // Recall (Recuperar Memorias)
    // ========================================================================

    /// Busca memorias similares por texto
    pub fn recall_similar(&self, query: &str, k: usize) -> Result<Vec<MemoryRecall>> {
        let embedding = self.embed(query)?;

        // Búsqueda híbrida: vector + keywords
        let params = HybridSearchParams::hybrid(embedding, query, k);
        let results = self.db().hybrid_search(params)?;

        Ok(results.into_iter().map(|r| self.to_recall(r)).collect())
    }

    /// Busca memorias similares con embedding externo
    pub fn recall_by_embedding(&self, embedding: &[f32], k: usize) -> Result<Vec<MemoryRecall>> {
        let results = self.db().search(embedding, k)?;
        Ok(results
            .into_iter()
            .map(|r| self.to_recall_from_search(r))
            .collect())
    }

    /// Busca experiencias similares (solo episodios)
    pub fn recall_experiences(&self, query: &str, k: usize) -> Result<Vec<MemoryRecall>> {
        let embedding = self.embed(query)?;

        let results = self.db().search_with_filter(
            &embedding,
            k,
            Filter::eq("type", MemoryType::Episode.as_str()),
        )?;

        Ok(results
            .into_iter()
            .map(|r| self.to_recall_from_search(r))
            .collect())
    }

    /// Busca código similar
    pub fn recall_code(&self, query: &str, k: usize) -> Result<Vec<MemoryRecall>> {
        let embedding = self.embed(query)?;

        let results = self.db().search_with_filter(
            &embedding,
            k,
            Filter::eq("type", MemoryType::CodeSnippet.as_str()),
        )?;

        Ok(results
            .into_iter()
            .map(|r| self.to_recall_from_search(r))
            .collect())
    }

    /// Busca soluciones a errores
    pub fn recall_error_solutions(
        &self,
        error_message: &str,
        k: usize,
    ) -> Result<Vec<MemoryRecall>> {
        let embedding = self.embed(error_message)?;

        let results = self.db().search_with_filter(
            &embedding,
            k,
            Filter::eq("type", MemoryType::ErrorSolution.as_str()),
        )?;

        Ok(results
            .into_iter()
            .map(|r| self.to_recall_from_search(r))
            .collect())
    }

    /// Busca por keywords exactos
    pub fn recall_by_keywords(&self, keywords: &str, k: usize) -> Result<Vec<MemoryRecall>> {
        let results = self.db().keyword_search(keywords, k)?;
        Ok(results.into_iter().map(|r| self.to_recall(r)).collect())
    }

    /// Busca en el proyecto actual (usa índice parcial)
    pub fn recall_in_project(&self, query: &str, k: usize) -> Result<Vec<MemoryRecall>> {
        let project = self.working.read().current_project.clone();

        if let Some(project) = project {
            let index_name = format!("project_{}", project.replace(['/', '\\', ' '], "_"));

            if self.db().has_partial_index(&index_name) {
                let embedding = self.embed(query)?;
                let results = self.db().search_partial(&index_name, &embedding, k)?;
                return Ok(results
                    .into_iter()
                    .map(|r| self.to_recall_from_search(r))
                    .collect());
            }
        }

        // Fallback a búsqueda general con filtro
        if let Some(ref project) = self.working.read().current_project {
            let embedding = self.embed(query)?;
            let results = self.db().search_with_filter(
                &embedding,
                k,
                Filter::eq("project", project.as_str()),
            )?;
            return Ok(results
                .into_iter()
                .map(|r| self.to_recall_from_search(r))
                .collect());
        }

        self.recall_similar(query, k)
    }

    /// Busca experiencias exitosas similares
    pub fn recall_successful(&self, query: &str, k: usize) -> Result<Vec<MemoryRecall>> {
        let embedding = self.embed(query)?;

        let results = self.db().search_with_filter(
            &embedding,
            k,
            Filter::all(vec![
                Filter::eq("type", MemoryType::Episode.as_str()),
                Filter::eq("outcome", "success"),
            ]),
        )?;

        Ok(results
            .into_iter()
            .map(|r| self.to_recall_from_search(r))
            .collect())
    }

    /// Busca experiencias fallidas para evitar errores
    pub fn recall_failures(&self, query: &str, k: usize) -> Result<Vec<MemoryRecall>> {
        let embedding = self.embed(query)?;

        let results = self.db().search_with_filter(
            &embedding,
            k,
            Filter::all(vec![
                Filter::eq("type", MemoryType::Episode.as_str()),
                Filter::eq("outcome", "failure"),
            ]),
        )?;

        Ok(results
            .into_iter()
            .map(|r| self.to_recall_from_search(r))
            .collect())
    }

    // ========================================================================
    // Utilidades
    // ========================================================================

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
                    _ => MemoryType::Episode,
                },
                _ => MemoryType::Episode,
            })
            .unwrap_or(MemoryType::Episode);

        let content = result
            .metadata
            .as_ref()
            .and_then(|m| m.get("task").or(m.get("description")).or(m.get("code")))
            .map(|v| match v {
                crate::MetadataValue::String(s) => s.clone(),
                _ => String::new(),
            })
            .unwrap_or_default();

        MemoryRecall {
            id: result.id,
            memory_type,
            relevance_score: result.score,
            content,
            metadata: result.metadata,
        }
    }

    fn to_recall_from_search(&self, result: SearchResult) -> MemoryRecall {
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
                    _ => MemoryType::Episode,
                },
                _ => MemoryType::Episode,
            })
            .unwrap_or(MemoryType::Episode);

        let content = result
            .metadata
            .as_ref()
            .and_then(|m| m.get("task").or(m.get("description")).or(m.get("code")))
            .map(|v| match v {
                crate::MetadataValue::String(s) => s.clone(),
                _ => String::new(),
            })
            .unwrap_or_default();

        MemoryRecall {
            id: result.id,
            memory_type,
            relevance_score: result.distance,
            content,
            metadata: result.metadata,
        }
    }

    /// Obtiene estadísticas de la memoria
    pub fn stats(&self) -> Result<MemoryStats> {
        // Single pass: count types by iterating all IDs and reading metadata
        let mut episodes = 0usize;
        let mut code_snippets = 0usize;
        let mut api_knowledge = 0usize;
        let mut error_solutions = 0usize;
        let mut patterns = 0usize;
        let mut internal = 0usize;

        for id in self.db().list_ids()? {
            if let Some((_, Some(meta))) = self.db().get(&id)? {
                if let Some(crate::MetadataValue::String(t)) = meta.get("type") {
                    match t.as_str() {
                        "episode" => episodes += 1,
                        "code_snippet" => code_snippets += 1,
                        "api_knowledge" => api_knowledge += 1,
                        "error_solution" => error_solutions += 1,
                        "pattern" => patterns += 1,
                        "__internal__" => internal += 1,
                        _ => {}
                    }
                }
            }
        }

        let total = self.db().len() - internal;

        Ok(MemoryStats {
            total_entries: total,
            episodes,
            code_snippets,
            api_knowledge,
            error_solutions,
            patterns,
            projects: self
                .db()
                .list_partial_indexes()
                .iter()
                .filter_map(|idx| idx.name.strip_prefix("project_").map(String::from))
                .collect(),
        })
    }

    /// Elimina una entrada de memoria
    pub fn forget(&self, id: &str) -> Result<bool> {
        let deleted = self.db().delete(id)?;
        if deleted {
            if let Some(ref log) = self.changelog {
                log.track_delete(id);
            }
        }
        Ok(deleted)
    }

    /// Limpia memorias antiguas
    pub fn cleanup_old(&self, max_age_days: u32) -> Result<usize> {
        let cutoff = current_timestamp().saturating_sub(max_age_days as u64 * 24 * 60 * 60);
        let mut deleted = 0;

        let all_ids = self.db().list_ids()?;
        for id in all_ids {
            if let Some((_, Some(meta))) = self.db().get(&id)? {
                if let Some(crate::MetadataValue::Int(ts)) = meta.get("timestamp") {
                    if (*ts as u64) < cutoff && self.db().delete(&id)? {
                        deleted += 1;
                    }
                }
            }
        }

        Ok(deleted)
    }

    /// Acceso a la base de datos subyacente
    pub fn db(&self) -> &VectorDB {
        self.inner.db()
    }

    /// Acceso al GenericMemory subyacente (para features avanzadas de prioridad/transfer)
    pub fn generic_memory(&self) -> &GenericMemory<SoftwareDevelopment> {
        &self.inner
    }

    /// Acceso al changelog
    pub fn changelog(&self) -> Option<&ChangeLog> {
        self.changelog.as_ref()
    }
}

// ============================================================================
// Helpers
// ============================================================================

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn generate_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let count = COUNTER.fetch_add(1, Ordering::SeqCst);
    let time = current_timestamp();
    format!("{:x}{:04x}", time, count & 0xFFFF)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_working_context() {
        let mut ctx = WorkingContext::new();
        ctx.set_project("my-project");
        ctx.set_task("implement feature");
        ctx.add_goal("Write tests");
        ctx.add_open_file("src/main.rs");

        assert_eq!(ctx.current_project, Some("my-project".to_string()));
        assert_eq!(ctx.current_task, Some("implement feature".to_string()));
        assert_eq!(ctx.active_goals.len(), 1);
    }

    #[test]
    fn test_agent_memory_creation() {
        let config = MemoryConfig::small();
        let memory = AgentMemory::new(config).unwrap();

        let stats = memory.stats().unwrap();
        assert_eq!(stats.total_entries, 0);
    }

    fn dummy_embed(dims: usize) -> impl Fn(&str) -> Vec<f32> + Send + Sync {
        move |text: &str| {
            // Simple deterministic hash-based embedding for tests
            let mut vec = vec![0.0f32; dims];
            for (i, byte) in text.bytes().enumerate() {
                vec[i % dims] += byte as f32 / 255.0;
            }
            // Normalize
            let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > 0.0 {
                vec.iter_mut().for_each(|x| *x /= norm);
            }
            vec
        }
    }

    #[test]
    fn test_learn_task_requires_embed_fn() {
        let config = MemoryConfig::small();
        let memory = AgentMemory::new(config).unwrap();

        // Without embed_fn, learn_task should fail
        let result = memory.learn_task(
            "Implement login",
            "fn login() { ... }",
            TaskOutcome::Success,
            vec!["Use bcrypt for passwords"],
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_learn_task() {
        let config = MemoryConfig::small();
        let mut memory = AgentMemory::new(config).unwrap();
        memory.set_embed_fn(dummy_embed(384));

        let id = memory
            .learn_task(
                "Implement login",
                "fn login() { ... }",
                TaskOutcome::Success,
                vec!["Use bcrypt for passwords"],
            )
            .unwrap();

        assert!(id.starts_with("episode-"));
        assert_eq!(memory.db().len(), 1);
    }

    #[test]
    fn test_learn_code_snippet() {
        let config = MemoryConfig::small();
        let mut memory = AgentMemory::new(config).unwrap();
        memory.set_embed_fn(dummy_embed(384));

        let id = memory
            .learn_code(CodeSnippet {
                code: "fn hello() { println!(\"Hello\"); }".to_string(),
                description: "Simple hello function".to_string(),
                language: Language::Rust,
                dependencies: vec![],
                use_case: "Greeting".to_string(),
                quality_score: 0.9,
                tags: vec!["example".to_string()],
            })
            .unwrap();

        assert!(id.starts_with("code-"));
    }

    #[test]
    fn test_learn_error_solution() {
        let config = MemoryConfig::small();
        let mut memory = AgentMemory::new(config).unwrap();
        memory.set_embed_fn(dummy_embed(384));

        let id = memory
            .learn_error_solution(ErrorSolution {
                error_message: "cannot borrow as mutable".to_string(),
                error_type: "E0596".to_string(),
                root_cause: "Missing mut keyword".to_string(),
                solution: "Add mut to variable declaration".to_string(),
                fixed_code: Some("let mut x = 5;".to_string()),
                language: Language::Rust,
            })
            .unwrap();

        assert!(id.starts_with("error-"));
    }

    #[test]
    fn test_focus_project() {
        let config = MemoryConfig::small();
        let memory = AgentMemory::new(config).unwrap();

        memory.focus_project("test-project").unwrap();

        assert_eq!(
            memory.working_context().current_project,
            Some("test-project".to_string())
        );
        assert!(memory.db().has_partial_index("project_test-project"));
    }
}
