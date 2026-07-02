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

use bincode::Options;

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
            quantized: stored.quantized,
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
///
/// Endurece la lectura contra archivos hostiles/truncados: toda alocación
/// derivada del contenido se acota al tamaño real del archivo, las entradas
/// se deserializan con un límite de bytes y los archivos v3+ exigen footer
/// válido con CRC verificado.
pub fn load_vectors<P: AsRef<Path>>(
    path: P,
) -> Result<(FileHeader, Vec<StoredVector>, LoadedIndexBlocks)> {
    let file = File::open(path.as_ref())?;
    let file_len = file.metadata()?.len();
    let mut reader = BufReader::with_capacity(256 * 1024, file);

    // Leer header
    let header = FileHeader::read_from(&mut reader)?;

    // Bytes restantes tras el header: límite superior para toda alocación
    // derivada del contenido del archivo.
    let header_pos = reader.stream_position().unwrap_or(super::format::HEADER_SIZE as u64);
    if file_len < header_pos {
        return Err(Error::Serialization(format!(
            "corrupt or malicious file: file size {} smaller than header end {}",
            file_len, header_pos
        )));
    }
    let mut remaining = file_len - header_pos;

    // Tamaño mínimo plausible de una entrada: solo el prefijo u32 de longitud.
    // Cualquier entrada real ocupa más, así que este umbral nunca rechaza un
    // archivo legítimo y a la vez acota un num_vectors hostil.
    const MIN_ENTRY_SIZE: u64 = 4;
    let max_plausible_vectors = remaining / MIN_ENTRY_SIZE;
    if header.num_vectors > max_plausible_vectors {
        return Err(Error::Serialization(format!(
            "corrupt or malicious file: header claims {} vectors but file can hold at most {}",
            header.num_vectors, max_plausible_vectors
        )));
    }

    // CRC32 hasher for verification
    let mut hasher = crc32fast::Hasher::new();

    // Pre-asignar capacidad acotada por el tamaño del archivo (no por el header).
    let cap = std::cmp::min(header.num_vectors as usize, max_plausible_vectors as usize);
    let mut vectors = Vec::with_capacity(cap);
    let mut buf4 = [0u8; 4];
    // Reuse a single buffer across iterations to avoid per-vector heap allocations
    let mut data: Vec<u8> = Vec::with_capacity(4096);

    for _ in 0..header.num_vectors {
        // Leer longitud
        reader.read_exact(&mut buf4)?;
        let len = u32::from_le_bytes(buf4) as u64;
        remaining = remaining.checked_sub(4).ok_or_else(|| {
            Error::Serialization(
                "corrupt or malicious file: entry length prefix beyond end of file".into(),
            )
        })?;

        if len > remaining {
            return Err(Error::Serialization(format!(
                "corrupt or malicious file: entry length {} exceeds remaining {} bytes",
                len, remaining
            )));
        }

        hasher.update(&buf4);

        // Leer datos (reuse buffer, only grows if needed)
        let len_us = len as usize;
        data.resize(len_us, 0);
        reader.read_exact(&mut data[..len_us])?;
        remaining -= len;

        hasher.update(&data[..len_us]);

        // Deserializar con límite derivado del len ya validado (no aloca Vec
        // hostiles dentro de bincode). Fixint + allow_trailing_bytes mantiene el
        // formato usado por `bincode::serialize` en save.
        let entry: VectorEntry = bincode::options()
            .with_limit(len_us as u64)
            .with_fixint_encoding()
            .allow_trailing_bytes()
            .deserialize(&data[..len_us])?;

        vectors.push(StoredVector {
            id: entry.id,
            vector: entry.vector,
            metadata: entry.metadata,
            quantized: entry.quantized,
        });
    }

    // Read index data from tagged blocks (v2+)
    let mut index_blocks = LoadedIndexBlocks::default();
    if header.index_offset > 0 {
        loop {
            // Read 4-byte tag
            if reader.read_exact(&mut buf4).is_err() {
                break;
            }
            hasher.update(&buf4);
            remaining = match remaining.checked_sub(4) {
                Some(r) => r,
                None => break,
            };

            // End marker (4 zero bytes)
            if buf4 == [0, 0, 0, 0] {
                break;
            }
            // Read block length
            let mut len_buf = [0u8; 4];
            if reader.read_exact(&mut len_buf).is_err() {
                break;
            }
            hasher.update(&len_buf);
            remaining = match remaining.checked_sub(4) {
                Some(r) => r,
                None => break,
            };

            let block_len = u32::from_le_bytes(len_buf) as u64;
            if block_len > remaining {
                return Err(Error::Serialization(format!(
                    "corrupt or malicious file: index block length {} exceeds remaining {} bytes",
                    block_len, remaining
                )));
            }
            // Read block data
            let mut block_data = vec![0u8; block_len as usize];
            if reader.read_exact(&mut block_data).is_err() {
                break;
            }
            hasher.update(&block_data);
            remaining -= block_len;

            match &buf4 {
                b"HNSW" => index_blocks.hnsw = Some(block_data),
                b"BM25" => index_blocks.bm25 = Some(block_data),
                _ => {} // Skip unknown blocks
            }
        }
    }

    // Verify CRC32 checksum (footer: 4 bytes checksum + 4 bytes "END!")
    let current_pos = reader.stream_position().unwrap_or(0);
    if header.version >= 3 {
        // v3+: save siempre escribe CRC real, así que el footer es obligatorio
        // y el checksum 0 ya no se acepta como "sin verificar".
        if current_pos + 8 > file_len {
            return Err(Error::Serialization(
                "corrupt or malicious file: missing CRC32 footer".into(),
            ));
        }
        let mut checksum_buf = [0u8; 4];
        let mut end_marker = [0u8; 4];
        reader.read_exact(&mut checksum_buf)?;
        reader.read_exact(&mut end_marker)?;
        if &end_marker != b"END!" {
            return Err(Error::Serialization(
                "corrupt or malicious file: bad footer marker".into(),
            ));
        }
        let stored_checksum = u32::from_le_bytes(checksum_buf);
        let computed = hasher.finalize();
        if computed != stored_checksum {
            return Err(Error::InvalidConfig(format!(
                "CRC32 checksum mismatch: expected {:08x}, got {:08x}. File may be corrupted.",
                stored_checksum, computed
            )));
        }
    } else {
        // v1/v2: verificación best-effort. checksum 0 = sin verificar (archivos
        // legados), y un archivo sin footer se acepta.
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
                quantized: None,
            },
            StoredVector {
                id: "b".to_string(),
                vector: Some(vec![4.0, 5.0, 6.0]),
                metadata: Some(Metadata::new()),
                quantized: None,
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
                quantized: None,
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
                quantized: None,
            },
            StoredVector {
                id: "vec-1".to_string(),
                vector: Some(vec![1.0, 2.0]),
                metadata: None,
                quantized: None,
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
                quantized: None,
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
                quantized: None,
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
                quantized: None,
        }];
        let mut header = FileHeader::new(1, 1, Distance::Cosine, &IndexType::Flat);
        save_vectors(&path, &mut header, vectors1.into_iter(), &IndexBlocks::none()).unwrap();

        // Second save (overwrites atomically)
        let vectors2 = vec![StoredVector {
            id: "updated".to_string(),
            vector: Some(vec![2.0]),
            metadata: None,
                quantized: None,
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
                quantized: None,
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

    // --- Hardening tests (hostile / truncated / footer-less .mmdb files) ---

    /// Build a raw 64-byte v3 header matching `FileHeader::write_to` layout.
    fn raw_v3_header(dimensions: u32, num_vectors: u64, index_offset: u64) -> Vec<u8> {
        use crate::storage::format::{HEADER_SIZE, MAGIC, VERSION};
        let mut buf = Vec::with_capacity(HEADER_SIZE);
        buf.extend_from_slice(MAGIC);
        buf.extend_from_slice(&VERSION.to_le_bytes());
        buf.extend_from_slice(&dimensions.to_le_bytes());
        buf.extend_from_slice(&num_vectors.to_le_bytes());
        buf.push(0); // distance_type: Cosine
        buf.push(0); // index_type: Flat
        buf.extend_from_slice(&0u16.to_le_bytes()); // hnsw_m
        buf.extend_from_slice(&0u16.to_le_bytes()); // hnsw_ef
        buf.extend_from_slice(&(HEADER_SIZE as u64).to_le_bytes()); // data_offset
        buf.extend_from_slice(&index_offset.to_le_bytes());
        buf.push(0); // quantization_type: None
        buf.extend_from_slice(&[0u8; HEADER_SIZE - 43]); // padding
        assert_eq!(buf.len(), HEADER_SIZE);
        buf
    }

    #[test]
    fn test_hostile_num_vectors_does_not_oom() {
        // Header claims u32::MAX vectors but file has no vector data.
        let path = temp_path();
        fs::write(&path, raw_v3_header(3, u32::MAX as u64, 0)).unwrap();

        let result = load_vectors(&path);
        assert!(result.is_err(), "expected error for hostile num_vectors");
        let msg = format!("{}", result.unwrap_err());
        assert!(
            msg.contains("corrupt or malicious file"),
            "expected corruption message, got: {}",
            msg
        );

        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_hostile_entry_len_does_not_oom() {
        // One entry whose length prefix (u32::MAX) exceeds the remaining bytes.
        let path = temp_path();
        let mut bytes = raw_v3_header(3, 1, 0);
        bytes.extend_from_slice(&u32::MAX.to_le_bytes()); // entry length prefix, no payload
        fs::write(&path, &bytes).unwrap();

        let result = load_vectors(&path);
        assert!(result.is_err(), "expected error for hostile entry len");
        let msg = format!("{}", result.unwrap_err());
        assert!(
            msg.contains("corrupt or malicious file"),
            "expected corruption message, got: {}",
            msg
        );

        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_v3_truncated_mid_data_errors_not_partial() {
        let path = temp_path();

        let vectors = vec![
            StoredVector { id: "a".to_string(), vector: Some(vec![1.0, 2.0, 3.0]), metadata: None, quantized: None },
            StoredVector { id: "b".to_string(), vector: Some(vec![4.0, 5.0, 6.0]), metadata: None, quantized: None },
            StoredVector { id: "c".to_string(), vector: Some(vec![7.0, 8.0, 9.0]), metadata: None, quantized: None },
        ];
        let mut header = FileHeader::new(3, 3, Distance::Cosine, &IndexType::Flat);
        save_vectors(&path, &mut header, vectors.into_iter(), &IndexBlocks::none()).unwrap();

        // Keep header + exactly the first entry (cut the second entry's length prefix).
        let full = fs::read(&path).unwrap();
        let first_entry_len = u32::from_le_bytes(full[64..68].try_into().unwrap()) as usize;
        let keep = 64 + 4 + first_entry_len;
        assert!(keep < full.len(), "cut point must be before end of file");
        fs::write(&path, &full[..keep]).unwrap();

        let result = load_vectors(&path);
        assert!(result.is_err(), "truncated file must not load as a partial database");
        // Must NOT have returned the first vector as a silent partial Ok.
        if let Ok((_, loaded, _)) = load_vectors(&path) {
            panic!("truncated file loaded as Ok with {} vectors", loaded.len());
        }

        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_v3_missing_footer_errors() {
        let path = temp_path();

        let vectors = vec![StoredVector { id: "a".to_string(), vector: Some(vec![1.0, 2.0, 3.0]), metadata: None, quantized: None }];
        let mut header = FileHeader::new(3, 1, Distance::Cosine, &IndexType::Flat);
        save_vectors(&path, &mut header, vectors.into_iter(), &IndexBlocks::none()).unwrap();

        // Strip the 8-byte footer (checksum + "END!").
        let full = fs::read(&path).unwrap();
        let cut = full.len() - 8;
        fs::write(&path, &full[..cut]).unwrap();

        let result = load_vectors(&path);
        assert!(result.is_err(), "v3 file without footer must not load");
        let msg = format!("{}", result.unwrap_err());
        assert!(
            msg.contains("missing CRC32 footer"),
            "expected missing-footer error, got: {}",
            msg
        );

        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_roundtrip_with_quantization_and_ivf_index() {
        use crate::quantization::{QuantizedVector, ScalarQuantParams};

        let path = temp_path();

        let params = ScalarQuantParams::default();
        let quant = QuantizedVector::Int8 { data: vec![1i8, 2, 3], params };
        let vectors = vec![StoredVector {
            id: "q1".to_string(),
            vector: None, // quantized-only document
            metadata: None,
            quantized: Some(quant.clone()),
        }];

        let hnsw_bytes = b"fake-ivf-serialized-index";
        let blocks = IndexBlocks { hnsw: Some(hnsw_bytes), bm25: None };
        let mut header = FileHeader::new(3, 1, Distance::Cosine, &IndexType::ivf())
            .with_quantization(crate::quantization::QuantizationType::Int8);
        save_vectors(&path, &mut header, vectors.into_iter(), &blocks).unwrap();

        let (loaded_header, loaded_vectors, loaded_blocks) = load_vectors(&path).unwrap();
        assert_eq!(loaded_header.version, 3);
        assert_eq!(loaded_header.dimensions, 3);
        assert_eq!(loaded_header.index_type, 2, "IVF index_type must round-trip");
        assert_eq!(loaded_header.quantization_type, 1, "Int8 quantization must round-trip");
        assert_eq!(loaded_vectors.len(), 1);
        assert_eq!(loaded_vectors[0].id, "q1");
        assert_eq!(loaded_vectors[0].vector, None);
        let loaded_quant = loaded_vectors[0].quantized.as_ref().expect("quantized survived");
        match loaded_quant {
            QuantizedVector::Int8 { data, .. } => assert_eq!(data, &vec![1i8, 2, 3]),
            other => panic!("expected Int8 quantization, got {:?}", other),
        }
        assert_eq!(loaded_blocks.hnsw.as_deref(), Some(hnsw_bytes.as_slice()));
        assert_eq!(loaded_blocks.bm25, None);

        fs::remove_file(&path).ok();
    }
}
