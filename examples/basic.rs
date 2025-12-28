//! Basic usage example for minimemory

use minimemory::{Config, Distance, IndexType, Metadata, VectorDB};

fn main() -> minimemory::Result<()> {
    // Create a new in-memory vector database
    // Vectors will have 4 dimensions, using cosine similarity
    let config = Config::new(4)
        .with_distance(Distance::Cosine)
        .with_index(IndexType::Flat);

    let db = VectorDB::new(config)?;

    // Insert some vectors with metadata
    let mut meta1 = Metadata::new();
    meta1.insert("title", "Document about cats");
    meta1.insert("category", "animals");

    db.insert("doc-1", &[0.1, 0.2, 0.3, 0.4], Some(meta1))?;

    let mut meta2 = Metadata::new();
    meta2.insert("title", "Document about dogs");
    meta2.insert("category", "animals");

    db.insert("doc-2", &[0.15, 0.25, 0.35, 0.45], Some(meta2))?;

    let mut meta3 = Metadata::new();
    meta3.insert("title", "Document about cars");
    meta3.insert("category", "vehicles");

    db.insert("doc-3", &[0.9, 0.1, 0.05, 0.02], Some(meta3))?;

    // Search for similar vectors
    let query = [0.12, 0.22, 0.32, 0.42];
    let results = db.search(&query, 2)?;

    println!("Search results for query {:?}:", query);
    for result in &results {
        println!("  - ID: {}, Distance: {:.4}", result.id, result.distance);
        if let Some(ref meta) = result.metadata {
            if let Some(title) = meta.get("title") {
                println!("    Title: {:?}", title);
            }
        }
    }

    // Check database stats
    println!("\nDatabase contains {} vectors", db.len());

    // Update a vector
    db.update("doc-1", &[0.2, 0.3, 0.4, 0.5], None)?;

    // Delete a vector
    db.delete("doc-3")?;
    println!("After deletion: {} vectors", db.len());

    Ok(())
}
