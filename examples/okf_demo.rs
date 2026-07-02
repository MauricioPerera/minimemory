//! Demo de ingesta y búsqueda de un bundle OKF v0.1 (sin embeddings, solo BM25).
//!
//! Corre con: `cargo run --example okf_demo`

use std::fs;
use std::path::PathBuf;

use minimemory::chunking::ChunkConfig;
use minimemory::okf::{OkfConfig, OkfIndex};

fn write(dir: &PathBuf, rel: &str, content: &str) {
    let p = dir.join(rel);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&p, content).unwrap();
}

fn main() {
    let mut bundle = std::env::temp_dir();
    bundle.push(format!(
        "okf_demo_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&bundle).unwrap();

    // index.md en la raíz: DEBE saltarse (no es un concepto).
    write(
        &bundle,
        "index.md",
        "---\ntype: index\n---\n# Índice del bundle — no debe ingerirse.\n",
    );

    // 3 conceptos con types distintos.
    write(
        &bundle,
        "concepts/rust.md",
        "---\n
type: language
title: Rust
description: Lenguaje de sistemas sin garbage collector
tags: [systems, memory-safety, rust]
timestamp: 2026-06-01T00:00:00Z
---
# Rust
Rust es un lenguaje de sistemas que garantiza seguridad de memoria sin garbage
collector. Combina performance con prevención de data races en tiempo de
compilación.
",
    );

    write(
        &bundle,
        "concepts/sqlite.md",
        "---\ntype: database
title: SQLite
description: Base de datos embebida sin servidor
tags:
  - embedded
  - sql
  - serverless
---
# SQLite
SQLite es una base de datos embebida, sin servidor, que guarda todo en un único
archivo. Ideal para apps locales y dispositivos edge.
",
    );

    write(
        &bundle,
        "tables/users.md",
        "---\ntype: table
title: Users
resource: https://example.com/users
tags: [users, auth]
---
# Tabla users

| id | name    | email             |
|----|---------|-------------------|
| 1  | Ada     | ada@example.com   |
| 2  | Grace   | grace@example.com |

La tabla users almacena las cuentas de la aplicación.
",
    );

    let index = OkfIndex::new(OkfConfig::new(ChunkConfig::default())).unwrap();
    let stats = index.ingest_bundle(&bundle).unwrap();
    println!("== Ingesta OKF ==");
    println!("estadísticas: {stats}");
    println!("conceptos ingeridos: {:?}", index.concepts());

    println!("\n== Búsqueda keyword (BM25) con filtro type=database ==");
    for hit in index
        .search("base de datos embebida", 5, Some("database"))
        .unwrap()
    {
        println!(
            "  [{}] {} (score {:.4}) — {}",
            hit.concept_id,
            hit.title.as_deref().unwrap_or("?"),
            hit.score,
            hit.snippet.replace('\n', " ")
        );
    }

    println!("\n== Búsqueda keyword sin filtro (todos los types) ==");
    for hit in index.search("rust memory safety", 5, None).unwrap() {
        println!(
            "  [{}] {} (score {:.4}) — {}",
            hit.concept_id,
            hit.title.as_deref().unwrap_or("?"),
            hit.score,
            hit.snippet.replace('\n', " ")
        );
    }

    // Limpieza.
    let _ = fs::remove_dir_all(&bundle);
}