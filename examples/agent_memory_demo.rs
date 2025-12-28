//! Demo del sistema de memoria agéntica
//!
//! Ejecutar con: cargo run --example agent_memory_demo

use minimemory::agent_memory::{
    AgentMemory, CodeSnippet, ErrorSolution, Language, MemoryConfig, TaskOutcome,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Demo de Memoria Agéntica ===\n");

    // 1. Crear memoria con dimensiones pequeñas para demo
    let mut memory = AgentMemory::new(MemoryConfig::small())?;

    // Simular función de embedding (en producción usarías OpenAI, etc.)
    memory.set_embed_fn(|text| {
        // Embedding simple basado en hash del texto
        // En producción: llamar a OpenAI, Sentence Transformers, etc.
        simple_hash_embedding(text, 384)
    });

    println!("✓ Memoria creada con 384 dimensiones\n");

    // 2. Establecer contexto de trabajo
    memory.with_working_context(|ctx| {
        ctx.set_project("mi-api-rust");
        ctx.set_task("Implementar autenticación JWT");
        ctx.add_goal("Crear middleware de auth");
        ctx.add_goal("Escribir tests");
        ctx.add_open_file("src/auth/mod.rs");
        ctx.add_open_file("src/middleware.rs");
    });

    println!("Contexto de trabajo:");
    {
        let ctx = memory.working_context();
        println!("  Proyecto: {:?}", ctx.current_project);
        println!("  Tarea: {:?}", ctx.current_task);
        println!("  Goals: {:?}", ctx.active_goals);
        println!("  Archivos: {:?}\n", ctx.open_files);
    }

    // 3. Aprender de tareas completadas
    println!("--- Aprendiendo experiencias ---\n");

    let id1 = memory.learn_task(
        "Implementar JWT authentication middleware",
        r#"
pub async fn auth_middleware(
    req: Request,
    next: Next,
) -> Result<Response, AuthError> {
    let token = extract_token(&req)?;
    let claims = verify_jwt(token)?;
    req.extensions_mut().insert(claims);
    Ok(next.run(req).await)
}
        "#,
        TaskOutcome::Success,
        vec![
            "Usar jsonwebtoken crate",
            "Extraer token del header Authorization",
            "Validar expiration claim",
        ],
    )?;
    println!("✓ Aprendida tarea JWT: {}", id1);

    let id2 = memory.learn_task(
        "Conectar a PostgreSQL con SQLx",
        r#"
let pool = PgPoolOptions::new()
    .max_connections(5)
    .connect(&database_url)
    .await?;
        "#,
        TaskOutcome::Success,
        vec![
            "Usar sqlx con feature postgres",
            "Pool de conexiones recomendado: 5",
        ],
    )?;
    println!("✓ Aprendida tarea PostgreSQL: {}", id2);

    let id3 = memory.learn_task(
        "Implementar rate limiting",
        "// Código incompleto...",
        TaskOutcome::Failure,
        vec!["Falló por no manejar correctamente el estado compartido"],
    )?;
    println!("✓ Aprendida tarea fallida: {}\n", id3);

    // 4. Aprender snippets de código
    println!("--- Aprendiendo código ---\n");

    let code_id = memory.learn_code(CodeSnippet {
        code: r#"
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
}

pub fn create_jwt(user_id: &str, secret: &[u8]) -> Result<String, Error> {
    let claims = Claims {
        sub: user_id.to_owned(),
        exp: (Utc::now() + Duration::hours(24)).timestamp() as usize,
        iat: Utc::now().timestamp() as usize,
    };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret))
}
        "#
        .to_string(),
        description: "Crear y firmar JWT tokens".to_string(),
        language: Language::Rust,
        dependencies: vec!["jsonwebtoken".into(), "chrono".into(), "serde".into()],
        use_case: "Autenticación de usuarios con tokens JWT".to_string(),
        quality_score: 0.95,
        tags: vec!["auth".into(), "jwt".into(), "security".into()],
    })?;
    println!("✓ Aprendido snippet JWT: {}\n", code_id);

    // 5. Aprender soluciones a errores
    println!("--- Aprendiendo soluciones a errores ---\n");

    let error_id = memory.learn_error_solution(ErrorSolution {
        error_message: "error[E0596]: cannot borrow `*self` as mutable".to_string(),
        error_type: "E0596".to_string(),
        root_cause: "Intentando mutar self en un método que recibe &self".to_string(),
        solution: "Cambiar &self a &mut self en la firma del método, o usar interior mutability con RefCell/Mutex".to_string(),
        fixed_code: Some("fn update(&mut self, value: i32) { self.data = value; }".to_string()),
        language: Language::Rust,
    })?;
    println!("✓ Aprendida solución error: {}\n", error_id);

    // 6. Recordar experiencias similares
    println!("--- Buscando en memoria ---\n");

    println!("Buscando: 'autenticación de usuarios'");
    let results = memory.recall_similar("autenticación de usuarios", 3)?;
    for (i, r) in results.iter().enumerate() {
        println!(
            "  {}. [{}] {} (score: {:.4})",
            i + 1,
            format!("{:?}", r.memory_type),
            truncate(&r.content, 50),
            r.relevance_score
        );
    }
    println!();

    println!("Buscando código: 'JWT token creation'");
    let code_results = memory.recall_code("JWT token creation", 2)?;
    for r in &code_results {
        println!(
            "  - {} (score: {:.4})",
            truncate(&r.content, 60),
            r.relevance_score
        );
    }
    println!();

    println!("Buscando solo experiencias exitosas:");
    let successes = memory.recall_successful("database connection", 2)?;
    for r in &successes {
        println!("  ✓ {}", truncate(&r.content, 60));
    }
    println!();

    println!("Buscando experiencias fallidas (para evitar errores):");
    let failures = memory.recall_failures("state management", 2)?;
    for r in &failures {
        println!("  ✗ {}", truncate(&r.content, 60));
    }
    println!();

    // 7. Buscar soluciones a errores
    println!("Buscando solución para: 'cannot borrow as mutable'");
    let solutions = memory.recall_error_solutions("cannot borrow as mutable", 1)?;
    for r in &solutions {
        if let Some(ref meta) = r.metadata {
            if let Some(minimemory::MetadataValue::String(sol)) = meta.get("solution") {
                println!("  Solución: {}", sol);
            }
        }
    }
    println!();

    // 8. Estadísticas
    let stats = memory.stats()?;
    println!("--- Estadísticas de Memoria ---");
    println!("  Total entradas: {}", stats.total_entries);
    println!("  Episodios: {}", stats.episodes);
    println!("  Snippets: {}", stats.code_snippets);
    println!("  Soluciones: {}", stats.error_solutions);
    println!();

    // 9. Persistencia
    let path = "demo_agent_memory.mmdb";
    memory.save(path)?;
    println!("✓ Memoria guardada en: {}", path);

    // Cargar de nuevo
    let loaded = AgentMemory::load(path, MemoryConfig::small())?;
    println!("✓ Memoria cargada: {} entradas", loaded.db().len());

    // Limpiar archivo de demo
    std::fs::remove_file(path)?;
    println!("✓ Archivo de demo eliminado\n");

    println!("=== Demo completada ===");
    Ok(())
}

/// Embedding simple basado en hash (solo para demo)
/// En producción usar OpenAI, Sentence Transformers, etc.
fn simple_hash_embedding(text: &str, dims: usize) -> Vec<f32> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut embedding = vec![0.0f32; dims];
    let words: Vec<&str> = text.split_whitespace().collect();

    for (i, word) in words.iter().enumerate() {
        let mut hasher = DefaultHasher::new();
        word.to_lowercase().hash(&mut hasher);
        let hash = hasher.finish();

        // Distribuir el hash en el embedding
        for j in 0..8 {
            let idx = ((hash >> (j * 8)) as usize + i) % dims;
            let val = ((hash >> (j * 4)) & 0xFF) as f32 / 255.0 - 0.5;
            embedding[idx] += val;
        }
    }

    // Normalizar
    let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in &mut embedding {
            *x /= norm;
        }
    }

    embedding
}

fn truncate(s: &str, max: usize) -> String {
    let s = s.replace('\n', " ").replace("  ", " ");
    if s.len() > max {
        format!("{}...", &s[..max])
    } else {
        s
    }
}
