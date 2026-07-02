use minimemory::{Config, VectorDB};

fn main() {
    let mode = std::env::args().nth(1).unwrap_or_default();
    let dir = std::env::temp_dir().join("mm_wal_crash_demo");
    std::fs::create_dir_all(&dir).unwrap();
    let wal = dir.join("demo.wal");

    match mode.as_str() {
        "write" => {
            let _ = std::fs::remove_file(&wal);
            let mut db = VectorDB::new(Config::new(8)).unwrap();
            db.enable_wal(&wal).unwrap();
            for i in 0..1000 {
                db.insert(format!("doc-{i}"), &vec![i as f32; 8], None)
                    .unwrap();
            }
            println!("[proceso 1] insertados {} docs en RAM", db.len());
            println!("[proceso 1] muriendo AHORA, sin save(), sin destructores (abort)");
            std::process::abort();
        }
        "read" => {
            let db = VectorDB::new_with_wal(Config::new(8), &wal).unwrap();
            println!("[proceso 2] docs recuperados tras el crash: {}", db.len());
            println!("[proceso 2] doc-500 presente: {}", db.contains("doc-500"));
            let hits = db.search(&vec![500.0; 8], 1).unwrap();
            println!("[proceso 2] busqueda del vecino de doc-500: {}", hits[0].id);
        }
        _ => eprintln!("uso: wal_crash_demo write|read"),
    }
}
