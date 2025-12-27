//! Ejemplo de uso de minimemory con Ollama para embeddings reales.
//!
//! Este ejemplo demuestra:
//! - Conexión con Ollama para generar embeddings
//! - Uso de GenericMemory con el preset SoftwareDevelopment
//! - Sistema de prioridades híbrido
//! - Recall con diferentes niveles de transferencia
//!
//! Requisitos:
//! - Ollama corriendo en localhost:11434
//! - Modelo de embedding instalado (ej: embeddinggemma, nomic-embed-text)
//!
//! Ejecutar con:
//! ```bash
//! cargo run --example ollama_memory
//! ```

use std::io::{self, Write};
use minimemory::memory_traits::{GenericMemory, Priority, InstanceContext};
use minimemory::memory_traits::presets::SoftwareDevelopment;

/// Cliente simple para Ollama embeddings.
struct OllamaClient {
    base_url: String,
    model: String,
}

impl OllamaClient {
    fn new(model: &str) -> Self {
        Self {
            base_url: "http://localhost:11434".to_string(),
            model: model.to_string(),
        }
    }

    /// Genera embedding para un texto usando Ollama.
    fn embed(&self, text: &str) -> Result<Vec<f32>, String> {
        let client = std::process::Command::new("curl")
            .args([
                "-s",
                "-X", "POST",
                &format!("{}/api/embed", self.base_url),
                "-d", &format!(r#"{{"model": "{}", "input": "{}"}}"#,
                    self.model,
                    text.replace('"', "\\\"").replace('\n', " ")
                ),
            ])
            .output()
            .map_err(|e| format!("Failed to call curl: {}", e))?;

        let response = String::from_utf8_lossy(&client.stdout);

        // Parse JSON response manually (avoiding extra dependencies)
        if let Some(start) = response.find("\"embeddings\":[[") {
            let start = start + 15;
            if let Some(end) = response[start..].find("]]") {
                let nums_str = &response[start..start + end];
                let nums: Result<Vec<f32>, _> = nums_str
                    .split(',')
                    .map(|s| s.trim().parse::<f32>())
                    .collect();
                return nums.map_err(|e| format!("Parse error: {}", e));
            }
        }

        Err(format!("Failed to parse response: {}", response))
    }

    /// Obtiene la dimensión de los embeddings.
    fn dimensions(&self) -> Result<usize, String> {
        let test = self.embed("test")?;
        Ok(test.len())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== minimemory + Ollama Demo ===\n");

    // Inicializar cliente Ollama
    print!("Conectando con Ollama... ");
    io::stdout().flush()?;

    let ollama = OllamaClient::new("embeddinggemma");
    let dims = ollama.dimensions()?;
    println!("OK! (modelo: embeddinggemma, dims: {})\n", dims);

    // Crear memoria con preset de desarrollo de software
    let memory = GenericMemory::<SoftwareDevelopment>::new(dims)?;

    // Establecer contexto del proyecto actual
    memory.set_context(
        InstanceContext::new("demo-project")
            .with_context("rust")
            .with_domain("backend")
    );

    println!("Memoria inicializada con preset: SoftwareDevelopment\n");

    // =========================================================================
    // Fase 1: Aprender experiencias de desarrollo
    // =========================================================================
    println!("--- Fase 1: Aprendiendo experiencias ---\n");

    let experiences = vec![
        // Experiencias críticas (seguridad)
        ("security-fix-1", "Fixed SQL injection vulnerability in user login",
         "Sanitized user input using parameterized queries", "success", Priority::Critical),

        // Experiencias de alta prioridad (bugs)
        ("bug-fix-1", "Fixed null pointer exception in payment processing",
         "Added null check before accessing payment object", "success", Priority::High),
        ("bug-fix-2", "Fixed race condition in concurrent user sessions",
         "Implemented mutex lock for session access", "success", Priority::High),

        // Experiencias normales
        ("feature-1", "Implemented user authentication with JWT tokens",
         "Created auth middleware and token validation", "success", Priority::Normal),
        ("feature-2", "Added pagination to API endpoints",
         "Implemented cursor-based pagination for large datasets", "success", Priority::Normal),
        ("refactor-1", "Refactored database connection pooling",
         "Migrated from single connection to connection pool", "success", Priority::Normal),

        // Experiencias de baja prioridad
        ("style-1", "Fixed code formatting in utils module",
         "Applied rustfmt to all files", "success", Priority::Low),
        ("docs-1", "Updated API documentation",
         "Added examples to README", "success", Priority::Low),
    ];

    for (id, description, content, outcome, priority) in &experiences {
        print!("  Aprendiendo: {}... ", id);
        io::stdout().flush()?;

        let embedding = ollama.embed(&format!("{} {}", description, content))?;
        memory.learn_with_priority(id, &embedding, content, description, outcome, *priority)?;

        println!("OK (priority: {:?})", priority);
    }

    println!("\nTotal experiencias: {}\n", memory.stats().total_memories);

    // =========================================================================
    // Fase 2: Recall con diferentes consultas
    // =========================================================================
    println!("--- Fase 2: Probando recall ---\n");

    // Consulta sobre seguridad
    println!("Query: 'security vulnerability fix'");
    let query = ollama.embed("security vulnerability fix")?;
    let results = memory.recall(&query, 3)?;

    println!("Top 3 resultados:");
    for (i, r) in results.iter().enumerate() {
        println!("  {}. {} (relevance: {:.3}, priority: {:?}, transfer: {:?})",
            i + 1, r.id, r.relevance, r.priority, r.transfer_level);
    }
    println!();

    // Consulta solo prioridad crítica
    println!("Query: recall_critical (solo prioridad Critical)");
    let critical = memory.recall_critical(&query, 5)?;
    println!("Resultados críticos: {}", critical.len());
    for r in &critical {
        println!("  - {} (priority: {:?})", r.id, r.priority);
    }
    println!();

    // Consulta alta prioridad
    println!("Query: recall_high_priority (High + Critical)");
    let high = memory.recall_high_priority(&query, 5)?;
    println!("Resultados alta prioridad: {}", high.len());
    for r in &high {
        println!("  - {} (priority: {:?}, score: {:.3})", r.id, r.priority, r.priority_score);
    }
    println!();

    // =========================================================================
    // Fase 3: Feedback y evolución de prioridades
    // =========================================================================
    println!("--- Fase 3: Feedback positivo ---\n");

    // Simular que algunas memorias fueron útiles
    println!("Marcando experiencias como útiles...");
    memory.mark_useful("security-fix-1");
    memory.mark_useful("security-fix-1"); // Más útil
    memory.mark_useful("bug-fix-1");
    memory.mark_useful("feature-1");

    // Re-query para ver cómo cambian los scores
    println!("\nRe-query después de feedback:");
    let query2 = ollama.embed("authentication security")?;
    let results2 = memory.recall(&query2, 5)?;

    for r in &results2 {
        println!("  {} - relevance: {:.3}, priority_score: {:.3}, combined: {:.3}",
            r.id, r.relevance, r.priority_score, r.combined_score);
    }
    println!();

    // =========================================================================
    // Fase 4: Búsqueda por keywords
    // =========================================================================
    println!("--- Fase 4: Búsqueda por keywords ---\n");

    println!("Query keywords: 'authentication JWT token'");
    let keyword_results = memory.recall_by_keywords("authentication JWT token", 3)?;

    for r in &keyword_results {
        println!("  - {} (score: {:.3})", r.id, r.relevance);
    }
    println!();

    // =========================================================================
    // Estadísticas finales
    // =========================================================================
    println!("--- Estadísticas finales ---\n");
    let stats = memory.stats();
    println!("Total memorias: {}", stats.total_memories);
    println!("Preset: {}", stats.preset_name);
    println!("Total accesos: {}", stats.total_accesses);
    println!("Utilidad promedio: {:.2}%", stats.avg_usefulness * 100.0);

    println!("\n=== Demo completada! ===");
    Ok(())
}
