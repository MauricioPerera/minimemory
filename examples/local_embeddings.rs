//! Ejemplo: Embeddings locales con minimemory
//!
//! Genera embeddings directamente en Rust sin necesidad de APIs externas.
//!
//! ## Ejecución
//!
//! ```bash
//! # Modelo ligero (MiniLM, 22.7M params, 384 dims)
//! cargo run --example local_embeddings --features embeddings
//!
//! # Nota: La primera ejecución descarga el modelo (~91 MB para MiniLM)
//! ```

#[cfg(feature = "embeddings")]
fn main() -> minimemory::Result<()> {
    use minimemory::agent_memory::{AgentMemory, TaskOutcome};
    use minimemory::embeddings::{Embedder, EmbeddingModel};

    println!("=== minimemory: Embeddings Locales ===\n");

    // ---------------------------------------------------------------
    // 1. Uso directo del Embedder
    // ---------------------------------------------------------------
    println!("1. Uso directo del Embedder\n");

    let embedder = Embedder::new(EmbeddingModel::MiniLM)?;
    println!(
        "   Modelo: {:?} ({} dims)",
        "MiniLM",
        embedder.dimensions()
    );

    let text = "Rust is a systems programming language focused on safety and performance.";
    let embedding = embedder.embed(text)?;
    println!("   Texto: \"{}\"", &text[..60]);
    println!("   Embedding: [{:.4}, {:.4}, {:.4}, ...]", embedding[0], embedding[1], embedding[2]);
    println!("   Longitud: {}\n", embedding.len());

    // Batch embeddings
    let texts = vec![
        "Machine learning with Rust",
        "Aprendizaje automático con Rust",
        "Web development in JavaScript",
    ];
    let embeddings = embedder.embed_batch(&texts)?;
    println!("   Batch de {} textos procesados\n", embeddings.len());

    // ---------------------------------------------------------------
    // 2. Integración con VectorDB
    // ---------------------------------------------------------------
    println!("2. Integración con VectorDB\n");

    use minimemory::{Config, Distance, VectorDB};

    let db = VectorDB::new(
        Config::new(384).with_distance(Distance::Cosine),
    )?;

    // Insertar documentos con embeddings generados localmente
    let docs = vec![
        ("rust-1", "Memory safety without garbage collection"),
        ("rust-2", "Zero-cost abstractions in systems programming"),
        ("python-1", "Machine learning with scikit-learn"),
        ("js-1", "Building web applications with React"),
    ];

    for (id, text) in &docs {
        let emb = embedder.embed(text)?;
        db.insert(*id, &emb, None)?;
    }

    // Buscar documentos similares
    let query = "safe systems programming";
    let query_emb = embedder.embed(query)?;
    let results = db.search(&query_emb, 2)?;

    println!("   Query: \"{}\"", query);
    for r in &results {
        println!("   → {} (distance: {:.4})", r.id, r.distance);
    }
    println!();

    // ---------------------------------------------------------------
    // 3. Integración con AgentMemory
    // ---------------------------------------------------------------
    println!("3. Integración con AgentMemory\n");

    let memory = AgentMemory::with_local_embeddings(EmbeddingModel::MiniLM)?;

    memory.learn_task(
        "Implement JWT authentication",
        "fn verify_token(token: &str) -> Result<Claims> { ... }",
        TaskOutcome::Success,
        vec!["Use jsonwebtoken crate", "Always validate expiration"],
    )?;

    memory.learn_task(
        "Add CORS middleware",
        "fn cors_layer() -> CorsLayer { ... }",
        TaskOutcome::Success,
        vec!["Use tower-http", "Allow specific origins in production"],
    )?;

    let recalls = memory.recall_similar("authentication and security", 2)?;
    println!("   Query: \"authentication and security\"");
    for r in &recalls {
        println!("   → {} (score: {:.4}): {}", r.id, r.relevance_score, r.content);
    }

    let stats = memory.stats()?;
    println!("\n   Estadísticas: {} entradas totales", stats.total_entries);

    println!("\n=== Completado ===");
    Ok(())
}

#[cfg(not(feature = "embeddings"))]
fn main() {
    eprintln!("Este ejemplo requiere el feature 'embeddings':");
    eprintln!("  cargo run --example local_embeddings --features embeddings");
}
