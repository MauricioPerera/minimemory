//! Storage en disco usando archivos y serialización binaria.
//!
//! Proporciona persistencia para la base de datos vectorial.

use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

use crate::error::Result;
use crate::types::{Metadata, StoredVector};

use super::format::{FileHeader, VectorEntry, HEADER_SIZE};

/// Guarda vectores a un archivo .mmdb
pub fn save_vectors<P: AsRef<Path>>(
    path: P,
    header: &mut FileHeader,
    vectors: impl Iterator<Item = StoredVector>,
) -> Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);

    // Reservar espacio para el header (lo escribiremos al final con offsets correctos)
    let placeholder = vec![0u8; HEADER_SIZE];
    writer.write_all(&placeholder)?;

    // Escribir documentos
    let mut count = 0u64;
    for stored in vectors {
        let entry = VectorEntry {
            id: stored.id,
            vector: stored.vector,
            metadata: stored.metadata,
        };

        let encoded = bincode::serialize(&entry)?;
        let len = encoded.len() as u32;

        writer.write_all(&len.to_le_bytes())?;
        writer.write_all(&encoded)?;

        count += 1;
    }

    // Obtener posición actual (inicio del índice)
    let index_offset = writer.stream_position().unwrap_or(0);
    header.index_offset = index_offset;
    header.num_vectors = count;

    // TODO: Escribir índice serializado aquí

    // Escribir footer (checksum simple)
    let checksum: u32 = 0; // TODO: Calcular CRC32
    writer.write_all(&checksum.to_le_bytes())?;
    writer.write_all(b"END!")?;

    // Volver al inicio y escribir header real
    writer.flush()?;
    drop(writer);

    // Reescribir header con offsets correctos
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .open(path.as_ref())?;

    header.write_to(&mut file)?;

    Ok(())
}

/// Carga vectores desde un archivo .mmdb
pub fn load_vectors<P: AsRef<Path>>(
    path: P,
) -> Result<(FileHeader, Vec<StoredVector>)> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    // Leer header
    let header = FileHeader::read_from(&mut reader)?;

    // Leer vectores
    let mut vectors = Vec::with_capacity(header.num_vectors as usize);
    let mut buf4 = [0u8; 4];

    for _ in 0..header.num_vectors {
        // Leer longitud
        if reader.read_exact(&mut buf4).is_err() {
            break;
        }
        let len = u32::from_le_bytes(buf4) as usize;

        // Leer datos
        let mut data = vec![0u8; len];
        reader.read_exact(&mut data)?;

        // Deserializar
        let entry: VectorEntry = bincode::deserialize(&data)?;

        vectors.push(StoredVector {
            id: entry.id,
            vector: entry.vector,
            metadata: entry.metadata,
        });
    }

    Ok((header, vectors))
}

/// Extensión del trait Read para obtener posición del stream
trait StreamPosition {
    fn stream_position(&mut self) -> std::io::Result<u64>;
}

impl<W: Write + std::io::Seek> StreamPosition for BufWriter<W> {
    fn stream_position(&mut self) -> std::io::Result<u64> {
        self.seek(std::io::SeekFrom::Current(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::distance::Distance;
    use crate::index::IndexType;
    use std::fs;
    use std::path::PathBuf;

    fn temp_path() -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("minimemory_test_{}.mmdb", std::process::id()));
        path
    }

    #[test]
    fn test_save_and_load() {
        let path = temp_path();

        // Crear datos de prueba
        let vectors = vec![
            StoredVector {
                id: "a".to_string(),
                vector: Some(vec![1.0, 2.0, 3.0]),
                metadata: None,
            },
            StoredVector {
                id: "b".to_string(),
                vector: Some(vec![4.0, 5.0, 6.0]),
                metadata: Some(Metadata::new()),
            },
        ];

        let mut header = FileHeader::new(3, 2, Distance::Cosine, &IndexType::Flat);

        // Guardar
        save_vectors(&path, &mut header, vectors.clone().into_iter()).unwrap();

        // Cargar
        let (loaded_header, loaded_vectors) = load_vectors(&path).unwrap();

        assert_eq!(loaded_header.dimensions, 3);
        assert_eq!(loaded_header.num_vectors, 2);
        assert_eq!(loaded_vectors.len(), 2);
        assert_eq!(loaded_vectors[0].id, "a");
        assert_eq!(loaded_vectors[0].vector, Some(vec![1.0, 2.0, 3.0]));

        // Limpiar
        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_save_with_metadata() {
        let path = temp_path();

        let mut meta = Metadata::new();
        meta.insert("title", "Test");
        meta.insert("score", 42i64);

        let vectors = vec![StoredVector {
            id: "x".to_string(),
            vector: Some(vec![1.0]),
            metadata: Some(meta),
        }];

        let mut header = FileHeader::new(1, 1, Distance::Euclidean, &IndexType::Flat);
        save_vectors(&path, &mut header, vectors.into_iter()).unwrap();

        let (_, loaded) = load_vectors(&path).unwrap();

        assert!(loaded[0].metadata.is_some());
        let meta = loaded[0].metadata.as_ref().unwrap();
        assert!(meta.get("title").is_some());

        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_save_metadata_only_document() {
        let path = temp_path();

        let mut meta = Metadata::new();
        meta.insert("title", "Blog Post");

        let vectors = vec![
            StoredVector {
                id: "doc-1".to_string(),
                vector: None, // No vector, metadata only
                metadata: Some(meta),
            },
            StoredVector {
                id: "vec-1".to_string(),
                vector: Some(vec![1.0, 2.0]),
                metadata: None,
            },
        ];

        let mut header = FileHeader::new(2, 2, Distance::Cosine, &IndexType::Flat);
        save_vectors(&path, &mut header, vectors.into_iter()).unwrap();

        let (_, loaded) = load_vectors(&path).unwrap();

        assert_eq!(loaded.len(), 2);
        assert!(loaded[0].vector.is_none()); // metadata only
        assert!(loaded[0].metadata.is_some());
        assert!(loaded[1].vector.is_some()); // has vector

        fs::remove_file(&path).ok();
    }
}
