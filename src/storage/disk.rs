//! Storage en disco usando archivos y serialización binaria.
//!
//! Proporciona persistencia para la base de datos vectorial.
//! Soporta escrituras atómicas (write to .tmp + rename) y CRC32 checksums.
//!
//! ## Optimizaciones de I/O:
//! - BufWriter/BufReader de 256KB (vs 8KB default) para reducir syscalls
//! - Reutilización de buffer en carga de vectores (1 alloc vs N allocs)
//! - Header padding en stack (evita alloc heap para 22 bytes)

use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Seek, Write};
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::types::StoredVector;

use super::format::{FileHeader, VectorEntry, HEADER_SIZE};

/// Temporary file suffix for atomic writes
const TMP_SUFFIX: &str = ".tmp";

/// Returns the temporary path for atomic writes
fn tmp_path(path: &Path) -> PathBuf {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(TMP_SUFFIX);
    PathBuf::from(tmp)
}

/// Optional index data to persist alongside vectors
pub struct IndexBlocks<'a> {
    /// Serialized HNSW index data
    pub hnsw: Option<&'a [u8]>,
    /// Serialized BM25 index data
    pub bm25: Option<&'a [u8]>,
}

impl<'a> IndexBlocks<'a> {
    /// No index data
    pub fn none() -> Self {
        Self {
            hnsw: None,
            bm25: None,
        }
    }
}

/// Loaded index data from a .mmdb file
#[derive(Default, Debug)]
pub struct LoadedIndexBlocks {
    /// Serialized HNSW index data
    pub hnsw: Option<Vec<u8>>,
    /// Serialized BM25 index data
    pub bm25: Option<Vec<u8>>,
}

/// Guarda vectores a un archivo .mmdb
///
/// Uses atomic writes: data is written to a temporary file first,
/// then renamed to the target path. This prevents corruption from
/// crashes during write.
pub fn save_vectors<P: AsRef<Path>>(
    path: P,
    header: &mut FileHeader,
    vectors: impl Iterator<Item = StoredVector>,
    index_blocks: &IndexBlocks<'_>,
) -> Result<()> {
    let target = path.as_ref();
    let temp = tmp_path(target);

    // Write to temporary file first
    let result = write_vectors_to_file(&temp, header, vectors, index_blocks);

    if let Err(e) = result {
        // Clean up temp file on error
        let _ = fs::remove_file(&temp);
        return Err(e);
    }

    // Atomic rename: temp -> target
    fs::rename(&temp, target).map_err(|e| {
        // Clean up temp file if rename fails
        let _ = fs::remove_file(&temp);
        Error::Io(e)
    })?;

    Ok(())
}

/// Internal: writes all data to a file, including CRC32 checksum
fn write_vectors_to_file(
    path: &Path,
    header: &mut FileHeader,
    vectors: impl Iterator<Item = StoredVector>,
    index_blocks: &IndexBlocks<'_>,
) -> Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::with_capacity(256 * 1024, file);

    // Reservar espacio para el header (lo escribiremos al final con offsets correctos)
    let placeholder = [0u8; HEADER_SIZE];
    writer.write_all(&placeholder)?;

    // CRC32 hasher for all data after header
    let mut hasher = crc32fast::Hasher::new();

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
        let len_bytes = len.to_le_bytes();

        writer.write_all(&len_bytes)?;
        writer.write_all(&encoded)?;

        hasher.update(&len_bytes);
        hasher.update(&encoded);

        count += 1;
    }

    // Obtener posición actual (inicio del índice)
    let index_offset = writer.stream_position()?;
    header.index_offset = index_offset;
    header.num_vectors = count;

    // Write serialized index data (tagged block format)
    // Each block: tag (4 bytes) + length (4 bytes) + data
    for (tag, data) in [
        (b"HNSW", index_blocks.hnsw),
        (b"BM25", index_blocks.bm25),
    ] {
        if let Some(data) = data {
            writer.write_all(tag)?;
            let len = data.len() as u32;
            let len_bytes = len.to_le_bytes();
            writer.write_all(&len_bytes)?;
            writer.write_all(data)?;

            hasher.update(tag);
            hasher.update(&len_bytes);
            hasher.update(data);
        }
    }
    // End marker (4 zero bytes)
    let end_marker = 0u32.to_le_bytes();
    writer.write_all(&end_marker)?;
    hasher.update(&end_marker);

    // Write footer with CRC32 checksum
    let checksum = hasher.finalize();
    writer.write_all(&checksum.to_le_bytes())?;
    writer.write_all(b"END!")?;

    // Flush and rewrite header with correct offsets
    writer.flush()?;
    drop(writer);

    // Reescribir header con offsets correctos
    let mut file = std::fs::OpenOptions::new().write(true).open(path)?;
    header.write_to(&mut file)?;

    Ok(())
}

/// Carga vectores desde un archivo .mmdb
///
/// Returns (header, vectors, loaded_index_blocks).
/// Index data is present in version 2+ files.
/// Verifies CRC32 checksum if present (v2+ files).
pub fn load_vectors<P: AsRef<Path>>(
    path: P,
) -> Result<(FileHeader, Vec<StoredVector>, LoadedIndexBlocks)> {
    let file = File::open(path)?;
    let file_len = file.metadata()?.len();
    let mut reader = BufReader::with_capacity(256 * 1024, file);

    // Leer header
    let header = FileHeader::read_from(&mut reader)?;

    // CRC32 hasher for verification
    let mut hasher = crc32fast::Hasher::new();

    // Leer vectores
    let mut vectors = Vec::with_capacity(header.num_vectors as usize);
    let mut buf4 = [0u8; 4];
    // Reuse a single buffer across iterations to avoid per-vector heap allocations
    let mut data: Vec<u8> = Vec::with_capacity(4096);

    /// Maximum allowed size for a single serialized vector entry (16MB)
    const MAX_ENTRY_SIZE: usize = 16 * 1024 * 1024;

    for _ in 0..header.num_vectors {
        // Leer longitud
        reader.read_exact(&mut buf4)?;
        let len = u32::from_le_bytes(buf4) as usize;

        if len > MAX_ENTRY_SIZE {
            return Err(Error::InvalidConfig(format!(
                "Vector entry size {} exceeds maximum {} — file may be corrupted",
                len, MAX_ENTRY_SIZE
            )));
        }

        hasher.update(&buf4);

        // Leer datos (reuse buffer, only grows if needed)
        data.resize(len, 0);
        reader.read_exact(&mut data[..len])?;

        hasher.update(&data[..len]);

        // Deserializar
        let entry: VectorEntry = bincode::deserialize(&data)?;

        vectors.push(StoredVector {
            id: entry.id,
            vector: entry.vector,
            metadata: entry.metadata,
        });
    }

    // Read index data from tagged blocks (v2+)
    let mut index_blocks = LoadedIndexBlocks::default();
    if header.index_offset > 0 {
        loop {
            // Read 4-byte tag (EOF here is expected — end of file)
            match reader.read_exact(&mut buf4) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(Error::Io(e)),
            }
            hasher.update(&buf4);

            // End marker (4 zero bytes)
            if buf4 == [0, 0, 0, 0] {
                break;
            }
            // Read block length
            let mut len_buf = [0u8; 4];
            reader.read_exact(&mut len_buf)?;
            hasher.update(&len_buf);

            let block_len = u32::from_le_bytes(len_buf) as usize;
            if block_len > MAX_ENTRY_SIZE {
                return Err(Error::InvalidConfig(format!(
                    "Index block size {} exceeds maximum {} — file may be corrupted",
                    block_len, MAX_ENTRY_SIZE
                )));
            }
            // Read block data
            let mut block_data = vec![0u8; block_len];
            reader.read_exact(&mut block_data)?;
            hasher.update(&block_data);

            match &buf4 {
                b"HNSW" => index_blocks.hnsw = Some(block_data),
                b"BM25" => index_blocks.bm25 = Some(block_data),
                _ => {} // Skip unknown blocks
            }
        }
    }

    // Verify CRC32 checksum (footer: 4 bytes checksum + 4 bytes "END!")
    // Only verify if we have enough bytes remaining for the footer
    let current_pos = reader.stream_position().map_err(Error::Io)?;
    if current_pos + 8 <= file_len {
        let mut checksum_buf = [0u8; 4];
        let mut end_marker = [0u8; 4];

        if reader.read_exact(&mut checksum_buf).is_ok()
            && reader.read_exact(&mut end_marker).is_ok()
            && &end_marker == b"END!"
        {
            let stored_checksum = u32::from_le_bytes(checksum_buf);
            // Only verify if checksum is non-zero (v1 files wrote 0)
            if stored_checksum != 0 {
                let computed = hasher.finalize();
                if computed != stored_checksum {
                    return Err(Error::InvalidConfig(format!(
                        "CRC32 checksum mismatch: expected {:08x}, got {:08x}. File may be corrupted.",
                        stored_checksum, computed
                    )));
                }
            }
        }
    }

    Ok((header, vectors, index_blocks))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::distance::Distance;
    use crate::index::IndexType;
    use crate::types::Metadata;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_path() -> PathBuf {
        let unique_id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let mut path = std::env::temp_dir();
        path.push(format!(
            "minimemory_test_{}_{}.mmdb",
            std::process::id(),
            unique_id
        ));
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
        save_vectors(&path, &mut header, vectors.clone().into_iter(), &IndexBlocks::none()).unwrap();

        // Cargar
        let (loaded_header, loaded_vectors, _) = load_vectors(&path).unwrap();

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
        save_vectors(&path, &mut header, vectors.into_iter(), &IndexBlocks::none()).unwrap();

        let (_, loaded, _) = load_vectors(&path).unwrap();

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
        save_vectors(&path, &mut header, vectors.into_iter(), &IndexBlocks::none()).unwrap();

        let (_, loaded, _) = load_vectors(&path).unwrap();

        assert_eq!(loaded.len(), 2);
        assert!(loaded[0].vector.is_none()); // metadata only
        assert!(loaded[0].metadata.is_some());
        assert!(loaded[1].vector.is_some()); // has vector

        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_crc32_detects_corruption() {
        let path = temp_path();

        let vectors = vec![StoredVector {
            id: "a".to_string(),
            vector: Some(vec![1.0, 2.0, 3.0]),
            metadata: None,
        }];

        let mut header = FileHeader::new(3, 1, Distance::Cosine, &IndexType::Flat);
        save_vectors(&path, &mut header, vectors.into_iter(), &IndexBlocks::none()).unwrap();

        // Corrupt a float value in the vector data section.
        // File layout after header (64 bytes):
        //   [64..68] entry length prefix (u32 le)
        //   [68..77] bincode string "a" (8-byte len + 1 char)
        //   [77]     Option tag (1 = Some)
        //   [78..86] vec length (u64 le = 3)
        //   [86..90] f32 1.0
        //   [90..94] f32 2.0  <-- corrupt here
        //   [94..98] f32 3.0
        //   [98]     Option tag (0 = None for metadata)
        // Corrupting a float byte still allows bincode to deserialize,
        // so CRC32 is the one that catches it.
        {
            use std::io::{Seek, SeekFrom, Write};
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .open(&path)
                .unwrap();
            file.seek(SeekFrom::Start(90)).unwrap();
            file.write_all(&[0xFF]).unwrap();
        }

        // Load should detect checksum mismatch
        let result = load_vectors(&path);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("CRC32 checksum mismatch"),
            "Expected CRC32 error, got: {}",
            err_msg
        );

        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_atomic_write_no_temp_file_remains() {
        let path = temp_path();
        let temp = tmp_path(&path);

        let vectors = vec![StoredVector {
            id: "a".to_string(),
            vector: Some(vec![1.0]),
            metadata: None,
        }];

        let mut header = FileHeader::new(1, 1, Distance::Cosine, &IndexType::Flat);
        save_vectors(&path, &mut header, vectors.into_iter(), &IndexBlocks::none()).unwrap();

        // Target file should exist, temp file should not
        assert!(path.exists(), "Target file should exist");
        assert!(!temp.exists(), "Temp file should not remain after save");

        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_atomic_write_preserves_original_on_save() {
        let path = temp_path();

        // First save
        let vectors1 = vec![StoredVector {
            id: "original".to_string(),
            vector: Some(vec![1.0]),
            metadata: None,
        }];
        let mut header = FileHeader::new(1, 1, Distance::Cosine, &IndexType::Flat);
        save_vectors(&path, &mut header, vectors1.into_iter(), &IndexBlocks::none()).unwrap();

        // Second save (overwrites atomically)
        let vectors2 = vec![StoredVector {
            id: "updated".to_string(),
            vector: Some(vec![2.0]),
            metadata: None,
        }];
        let mut header2 = FileHeader::new(1, 1, Distance::Cosine, &IndexType::Flat);
        save_vectors(&path, &mut header2, vectors2.into_iter(), &IndexBlocks::none()).unwrap();

        // Should load the updated version
        let (_, loaded, _) = load_vectors(&path).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, "updated");

        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_save_with_index_data_and_crc32() {
        let path = temp_path();

        let vectors = vec![StoredVector {
            id: "v1".to_string(),
            vector: Some(vec![1.0, 2.0]),
            metadata: None,
        }];

        let hnsw_bytes = b"fake-hnsw-index-data-for-testing";
        let bm25_bytes = b"fake-bm25-index-data";
        let blocks = IndexBlocks {
            hnsw: Some(hnsw_bytes),
            bm25: Some(bm25_bytes),
        };
        let mut header = FileHeader::new(2, 1, Distance::Cosine, &IndexType::hnsw());
        save_vectors(&path, &mut header, vectors.into_iter(), &blocks).unwrap();

        // Load and verify all index blocks survive CRC32 round-trip
        let (loaded_header, loaded_vectors, loaded_blocks) = load_vectors(&path).unwrap();
        assert_eq!(loaded_header.dimensions, 2);
        assert_eq!(loaded_vectors.len(), 1);
        assert_eq!(loaded_vectors[0].id, "v1");
        assert_eq!(loaded_blocks.hnsw.as_deref(), Some(hnsw_bytes.as_slice()));
        assert_eq!(loaded_blocks.bm25.as_deref(), Some(bm25_bytes.as_slice()));

        fs::remove_file(&path).ok();
    }
}
