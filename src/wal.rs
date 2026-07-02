//! # Write-Ahead Log (WAL)
//!
//! Durabilidad por operación para minimemory. Hoy todo vive en RAM y solo
//! persiste con `save()` (reescritura completa del snapshot); el WAL registra
//! cada mutación de forma append-only (O(1) por op) y el snapshot pasa a ser
//! compactación: reescribe el estado completo y luego trunca el log.
//!
//! Este módulo es el WAL puro, sin integración con `VectorDB`. Otra tarea lo
//! enchufa en `db.rs`.
//!
//! ## Formato de archivo
//!
//! ```text
//! [Header: 8 bytes]   "MWAL" (magic) + versión u32 LE
//! [Entry 1][Entry 2] ... [Entry N]
//! ```
//!
//! Cada entrada (entry):
//!
//! ```text
//! [u32 LE: longitud del payload][payload: bincode de WalOp][u32 LE: CRC32 del payload]
//! ```
//!
//! El payload se serializa con bincode usando la misma configuración legacy
//! (fixint, little-endian) que `storage/disk.rs`, para consistencia con el
//! resto del crate. El CRC32 cubre solo el payload (no los prefijos de
//! longitud) y permite detectar escrituras parciales (torn writes) y
//! corrupción.
//!
//! ## Semántica de durabilidad (`WalConfig::fsync_on_append`)
//!
//! - `false` (default): cada `append` hace `flush` al OS (syscall `write`).
//!   Los datos sobreviven a un **crash del proceso** (están en el page cache
//!   del kernel) pero NO a un corte de energía.
//! - `true`: cada `append` hace además `fsync` explícito. Los datos sobreviven
//!   también a un **corte de energía** (se vacían al dispositivo). Es más
//!   lento y por eso es opt-in.
//!
//! ## Recovery de cola rota (torn write)
//!
//! Si un `append` se interrumpe a mitad (crash del proceso o corte de luz), la
//! última entrada queda incompleta: EOF antes de terminar payload/CRC, o su
//! CRC no cuadra. Eso NO es error: [`replay`] devuelve todas las entradas
//! válidas anteriores y reporta `truncated_tail = true`. Al reabrir con
//! [`WalWriter::open`] el archivo se trunca a la última entrada válida antes
//! de seguir appendeando, para no dejar basura en medio.
//!
//! La corrupción real en medio del log (CRC inválido en una entrada NO final)
//! sí es error explícito —no se hace replay parcial silencioso.
//!
//! ## Endurecimiento contra archivos hostiles
//!
//! Toda longitud leída del archivo se acota al tamaño real del archivo y a un
//! máximo absoluto [`MAX_ENTRY_PAYLOAD`] antes de alocar, igual que
//! `storage/disk.rs`: nada de `Vec::with_capacity` con un `u32` hostil.

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use bincode::Options;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::types::{Metadata, VectorId};

/// Magic bytes para identificar archivos WAL de minimemory.
pub const WAL_MAGIC: &[u8; 4] = b"MWAL";

/// Versión actual del formato WAL.
pub const WAL_VERSION: u32 = 1;

/// Tamaño del header en bytes (magic + versión u32).
pub const WAL_HEADER_SIZE: usize = 8;

/// Tamaño máximo admisible para el payload de una entrada.
///
/// Acota un `u32` hostil antes de alocar. Una entrada de WAL es una única
/// operación (un vector + metadata); incluso un embedding de 4096 dimensiones
/// ocupa ~16 KiB, así que 256 MiB es un techo holgado que nunca rechaza un
/// archivo legítimo y a la vez impide OOM.
pub const MAX_ENTRY_PAYLOAD: u64 = 256 * 1024 * 1024;

/// Capacidad del buffer de escritura (256 KiB, igual que `storage/disk.rs`).
const BUF_CAP: usize = 256 * 1024;

/// Operación registrada en el WAL.
///
/// Cada variante corresponde a una mutación de la base de datos. `vector` y
/// `metadata` son `Option` para soportar documentos solo-metadata y
/// actualizaciones parciales.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WalOp {
    /// Inserción de un nuevo documento.
    Insert {
        /// ID del documento.
        id: VectorId,
        /// Vector (None para documentos solo-metadata).
        vector: Option<Vec<f32>>,
        /// Metadata asociada.
        metadata: Option<Metadata>,
    },
    /// Actualización de un documento existente.
    Update {
        /// ID del documento.
        id: VectorId,
        /// Nuevo vector (None = dejar el existente).
        vector: Option<Vec<f32>>,
        /// Nueva metadata (None = dejar la existente).
        metadata: Option<Metadata>,
    },
    /// Borrado de un documento.
    Delete {
        /// ID del documento a borrar.
        id: VectorId,
    },
    /// Borrado completo de la base de datos.
    Clear,
}

/// `PartialEq` manual: [`Metadata`] no deriva `PartialEq` ( vive en `types.rs`
/// y no se modifica en esta tarea), pero su campo `fields` es un `HashMap` cuyos
/// valores sí implementan `PartialEq`, así que comparamos por ahí.
impl PartialEq for WalOp {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                WalOp::Insert { id, vector, metadata },
                WalOp::Insert {
                    id: oid,
                    vector: ov,
                    metadata: om,
                },
            ) => id == oid && vector == ov && meta_eq(metadata, om),
            (
                WalOp::Update { id, vector, metadata },
                WalOp::Update {
                    id: oid,
                    vector: ov,
                    metadata: om,
                },
            ) => id == oid && vector == ov && meta_eq(metadata, om),
            (WalOp::Delete { id }, WalOp::Delete { id: oid }) => id == oid,
            (WalOp::Clear, WalOp::Clear) => true,
            _ => false,
        }
    }
}

/// Compara dos `Option<Metadata>` por su campo `fields`.
fn meta_eq(a: &Option<Metadata>, b: &Option<Metadata>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => a.fields == b.fields,
        _ => false,
    }
}

/// Configuración de durabilidad del WAL.
///
/// Controla el trade-off entre rendimiento y supervivencia ante fallos:
///
/// - `fsync_on_append = false` (default): `append` hace flush al OS. Sobrevive
///   a crash del proceso; NO sobrevive a corte de energía.
/// - `fsync_on_append = true`: `append` hace `fsync` explícito. Sobrevive a
///   corte de energía, a costa de rendimiento.
#[derive(Debug, Clone, Copy, Default)]
pub struct WalConfig {
    /// Si `true`, cada `append` hace `fsync` explícito.
    pub fsync_on_append: bool,
}

impl WalConfig {
    /// Configuración por defecto (`fsync_on_append = false`).
    pub fn new() -> Self {
        Self::default()
    }

    /// Habilita `fsync` por `append` (sobrevive a corte de energía).
    pub fn with_fsync_on_append(mut self, on: bool) -> Self {
        self.fsync_on_append = on;
        self
    }
}

/// Resultado de [`replay`]: entradas válidas leídas + diagnóstico de cola rota.
#[derive(Debug, Clone)]
pub struct ReplayResult {
    /// Entradas válidas en orden, hasta la primera entrada incompleta/corrupta.
    pub ops: Vec<WalOp>,
    /// `true` si la última entrada estaba incompleta o su CRC no cuadraba
    /// (torn write tolerable). `false` si el log terminaba limpiamente.
    pub truncated_tail: bool,
    /// Offset (en bytes) del final de la última entrada válida confirmada.
    /// Incluye el header; para un log sin entradas vale `WAL_HEADER_SIZE`.
    pub valid_len: u64,
}

/// Escritor append-only sobre un archivo WAL.
pub struct WalWriter {
    writer: std::io::BufWriter<File>,
    config: WalConfig,
    path: PathBuf,
}

impl WalWriter {
    /// Abre (o crea) un WAL en `path` con configuración por defecto.
    ///
    /// Si el archivo existe, valida magic/versión y trunca la cola rota (torn
    /// write) a la última entrada válida antes de seguir appendeando. Si no
    /// existe, lo crea con header.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::open_with(path, WalConfig::default())
    }

    /// Abre (o crea) un WAL en `path` con la [`WalConfig`] dada.
    pub fn open_with<P: AsRef<Path>>(path: P, config: WalConfig) -> Result<Self> {
        let path = path.as_ref();
        ensure_file(path)?;
        let file = OpenOptions::new().append(true).create(true).open(path)?;
        Ok(Self {
            writer: std::io::BufWriter::with_capacity(BUF_CAP, file),
            config,
            path: path.to_path_buf(),
        })
    }

    /// Append de una operación al log. O(1) respecto al tamaño del log.
    ///
    /// La durabilidad depende de [`WalConfig::fsync_on_append`].
    pub fn append(&mut self, op: &WalOp) -> Result<()> {
        let payload = bincode::serialize(op)?;
        let len = payload.len() as u32;
        let crc = crc32fast::hash(&payload);
        self.writer.write_all(&len.to_le_bytes())?;
        self.writer.write_all(&payload)?;
        self.writer.write_all(&crc.to_le_bytes())?;
        if self.config.fsync_on_append {
            self.writer.flush()?;
            self.writer.get_ref().sync_all()?;
        } else {
            self.writer.flush()?;
        }
        Ok(())
    }

    /// Vacía el log pero reutiliza el archivo (reescrito con header intacto).
    ///
    /// Típico tras un checkpoint: el snapshot ya capturó el estado, el log
    /// completo ya no se necesita.
    ///
    /// El truncado se hace sobre un handle de escritura separado: en Windows,
    /// `set_len` sobre un handle append-only devuelve `PermissionDenied`, así
    /// que no reusamos el handle de append para truncar. Tras truncar, los
    /// siguientes `append` caen al final del archivo (== final del header) vía
    /// `O_APPEND`.
    pub fn truncate(&mut self) -> Result<()> {
        self.writer.flush()?;
        let mut f = OpenOptions::new().write(true).open(&self.path)?;
        f.set_len(0)?;
        f.seek(SeekFrom::Start(0))?;
        write_header_to(&mut f)?;
        f.sync_all()?;
        Ok(())
    }

    /// `fsync` explícito del archivo (independiente de `fsync_on_append`).
    pub fn sync(&mut self) -> Result<()> {
        self.writer.flush()?;
        self.writer.get_ref().sync_all()?;
        Ok(())
    }

    /// Flush del buffer interno al OS (no `fsync`).
    pub fn flush(&mut self) -> Result<()> {
        self.writer.flush()?;
        Ok(())
    }
}

/// Reconstruye el log leyendo todas las entradas válidas en orden.
///
/// Devuelve [`ReplayResult`]. Una cola rota (última entrada incompleta o con
/// CRC incorrecto) NO es error: se devuelven las entradas válidas anteriores y
/// `truncated_tail = true`. Una corrupción interna (CRC inválido en una
/// entrada NO final) sí devuelve `Err`.
pub fn replay<P: AsRef<Path>>(path: P) -> Result<ReplayResult> {
    let path = path.as_ref();
    let mut file = OpenOptions::new().read(true).open(path)?;
    let file_len = file.metadata()?.len();
    validate_header(&mut file, file_len)?;
    let scan = scan_entries(&mut file, file_len)?;
    Ok(ReplayResult {
        ops: scan.ops,
        truncated_tail: scan.truncated_tail,
        valid_len: scan.valid_len,
    })
}

// ---------------------------------------------------------------------------
// Internos
// ---------------------------------------------------------------------------

/// Header de archivo como bytes (magic + versión u32 LE).
fn wal_header() -> [u8; WAL_HEADER_SIZE] {
    let mut h = [0u8; WAL_HEADER_SIZE];
    h[..4].copy_from_slice(WAL_MAGIC);
    h[4..].copy_from_slice(&WAL_VERSION.to_le_bytes());
    h
}

/// Escribe el header en un writer.
fn write_header_to<W: Write>(w: &mut W) -> Result<()> {
    w.write_all(&wal_header())?;
    Ok(())
}

/// Lee y valida el header (magic + versión) desde la posición 0.
///
/// Tras regresar, el cursor queda justo después del header (offset
/// `WAL_HEADER_SIZE`).
fn validate_header(file: &mut File, file_len: u64) -> Result<()> {
    if file_len < WAL_HEADER_SIZE as u64 {
        return Err(Error::Serialization(format!(
            "WAL: file too small for header: {} bytes (need {})",
            file_len, WAL_HEADER_SIZE
        )));
    }
    file.seek(SeekFrom::Start(0))?;
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic)?;
    if &magic != WAL_MAGIC {
        return Err(Error::InvalidConfig(
            "WAL: invalid file format: bad magic bytes".into(),
        ));
    }
    let mut vbuf = [0u8; 4];
    file.read_exact(&mut vbuf)?;
    let version = u32::from_le_bytes(vbuf);
    if version != WAL_VERSION {
        return Err(Error::InvalidConfig(format!(
            "WAL: unsupported file version: {} (supported: {})",
            version, WAL_VERSION
        )));
    }
    Ok(())
}

/// Asegura que `path` exista con un header válido y cola íntegra.
///
/// - Si no existe: lo crea con header.
/// - Si existe y es más pequeño que el header: lo recrea con header limpio.
/// - Si existe y tiene header válido: escanea las entradas y, si detecta cola
///   rota, trunca el archivo a la última entrada válida.
/// - Cualquier corrupción interna (CRC inválido en entrada no final) propaga
///   error.
fn ensure_file(path: &Path) -> Result<()> {
    if !path.exists() {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        write_header_to(&mut file)?;
        file.sync_all()?;
        return Ok(());
    }

    let mut file = OpenOptions::new().read(true).write(true).open(path)?;
    let file_len = file.metadata()?.len();

    if file_len < WAL_HEADER_SIZE as u64 {
        // Header incompleto: recrear desde cero.
        file.set_len(0)?;
        file.seek(SeekFrom::Start(0))?;
        write_header_to(&mut file)?;
        file.sync_all()?;
        return Ok(());
    }

    validate_header(&mut file, file_len)?;
    let scan = scan_entries(&mut file, file_len)?;
    if scan.truncated_tail {
        file.set_len(scan.valid_len)?;
        file.sync_all()?;
    }
    Ok(())
}

/// Resultado interno del escaneo de entradas.
struct ScanOutcome {
    ops: Vec<WalOp>,
    /// Offset del final de la última entrada válida confirmada.
    valid_len: u64,
    /// `true` si la última entrada estaba incompleta o su CRC no cuadraba.
    truncated_tail: bool,
}

/// Escanea las entradas a partir del offset `WAL_HEADER_SIZE`.
///
/// Reglas:
/// - EOF limpio (0 bytes) entre entradas → fin normal.
/// - EOF parcial a mitad de una entrada (length/payload/crc) → `truncated_tail`
///   tolerable, se devuelven las entradas válidas anteriores.
/// - Longitud de payload > [`MAX_ENTRY_PAYLOAD`] → `Err` (hostil, sin OOM).
/// - Longitud que no cabe en el archivo restante → `truncated_tail` (torn write
///   con longitud plausible pero incompleta).
/// - CRC inválido en una entrada NO final (quedan bytes después) → `Err`
///   (corrupción interna).
/// - CRC inválido en la última entrada (sin bytes después) → `truncated_tail`
///   (torn write).
fn scan_entries(file: &mut File, file_len: u64) -> Result<ScanOutcome> {
    file.seek(SeekFrom::Start(WAL_HEADER_SIZE as u64))?;
    let mut last_valid = WAL_HEADER_SIZE as u64;
    let mut ops = Vec::new();
    let mut buf4 = [0u8; 4];
    let mut data: Vec<u8> = Vec::with_capacity(4096);

    loop {
        // Prefijo de longitud (u32 LE).
        let filled = read_min(file, &mut buf4)?;
        if filled == 0 {
            break; // EOF limpio entre entradas.
        }
        if filled < 4 {
            return Ok(tail(ops, last_valid));
        }
        let len = u32::from_le_bytes(buf4) as u64;

        // Anti-hostil: longitud absurda antes de alocar.
        if len > MAX_ENTRY_PAYLOAD {
            return Err(Error::Serialization(format!(
                "WAL: corrupt or malicious file: entry payload length {} exceeds maximum {}",
                len, MAX_ENTRY_PAYLOAD
            )));
        }

        let pos_after_len = file.stream_position()?;
        let remaining = file_len.saturating_sub(pos_after_len);
        // payload (len) + crc (4) deben caber; si no, torn write.
        if len + 4 > remaining {
            return Ok(tail(ops, last_valid));
        }

        // Payload.
        data.resize(len as usize, 0);
        let got = read_min(file, &mut data)? as u64;
        if got < len {
            return Ok(tail(ops, last_valid));
        }

        // CRC32 del payload.
        let got = read_min(file, &mut buf4)?;
        if got < 4 {
            return Ok(tail(ops, last_valid));
        }
        let stored_crc = u32::from_le_bytes(buf4);
        let computed = crc32fast::hash(&data);
        let pos_after_crc = file.stream_position()?;
        if computed != stored_crc {
            let bytes_after = file_len.saturating_sub(pos_after_crc);
            if bytes_after == 0 {
                // Última entrada con CRC roto → torn write tolerable.
                return Ok(tail(ops, last_valid));
            } else {
                // CRC roto en entrada NO final → corrupción interna.
                return Err(Error::Serialization(format!(
                    "WAL: internal corruption: CRC32 mismatch on non-final entry at offset {}",
                    pos_after_len - 4
                )));
            }
        }

        // Deserializar con límite = len (no aloca payloads hostiles dentro de
        // bincode). Fixint + allow_trailing_bytes matchea `bincode::serialize`.
        let op: WalOp = bincode::options()
            .with_limit(len)
            .with_fixint_encoding()
            .allow_trailing_bytes()
            .deserialize(&data)?;
        ops.push(op);
        last_valid = pos_after_crc;
    }

    Ok(ScanOutcome {
        ops,
        valid_len: last_valid,
        truncated_tail: false,
    })
}

/// Construye un `ScanOutcome` de cola rota con las entradas válidas acumuladas.
fn tail(ops: Vec<WalOp>, valid_len: u64) -> ScanOutcome {
    ScanOutcome {
        ops,
        valid_len,
        truncated_tail: true,
    }
}

/// Lee hasta `buf.len()` bytes, parando en EOF.
///
/// Devuelve la cantidad de bytes leídos. Solo puede ser menor que
/// `buf.len()` si se alcanzó EOF; nunca hace un `read` que deje el buffer a
/// medias por otra causa (reintenta hasta llenar o EOF).
fn read_min<R: Read>(r: &mut R, buf: &mut [u8]) -> Result<usize> {
    let mut filled = 0;
    while filled < buf.len() {
        let n = r.read(&mut buf[filled..])?;
        if n == 0 {
            break;
        }
        filled += n;
    }
    Ok(filled)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_path() -> PathBuf {
        let unique_id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let mut path = std::env::temp_dir();
        path.push(format!(
            "minimemory_wal_{}_{}.wal",
            std::process::id(),
            unique_id
        ));
        path
    }

    fn sample_ops() -> Vec<WalOp> {
        let mut meta = Metadata::new();
        meta.insert("title", "Hola mundo");
        meta.insert("score", 42i64);
        meta.insert("active", true);

        vec![
            WalOp::Insert {
                id: "doc-1".to_string(),
                vector: Some(vec![0.1, 0.2, 0.3]),
                metadata: Some(meta.clone()),
            },
            // Unicode en el id.
            WalOp::Insert {
                id: "doc-æøå-日本語".to_string(),
                vector: None,
                metadata: None,
            },
            WalOp::Update {
                id: "doc-1".to_string(),
                vector: Some(vec![0.9, 0.8, 0.7]),
                metadata: None,
            },
            WalOp::Delete {
                id: "doc-2".to_string(),
            },
            WalOp::Clear,
        ]
    }

    /// Bytes en disco de una entrada válida (len + payload + crc).
    fn entry_bytes(op: &WalOp) -> Vec<u8> {
        let payload = bincode::serialize(op).unwrap();
        let len = payload.len() as u32;
        let crc = crc32fast::hash(&payload);
        let mut buf = Vec::with_capacity(4 + payload.len() + 4);
        buf.extend_from_slice(&len.to_le_bytes());
        buf.extend_from_slice(&payload);
        buf.extend_from_slice(&crc.to_le_bytes());
        buf
    }

    /// Ecribe un archivo WAL crudo: header + entradas dadas.
    fn write_raw(path: &Path, entries: &[&WalOp]) {
        let mut buf = wal_header().to_vec();
        for e in entries {
            buf.extend_from_slice(&entry_bytes(e));
        }
        fs::write(path, &buf).unwrap();
    }

    #[test]
    fn test_roundtrip_varied_ops() {
        let path = temp_path();
        {
            let mut w = WalWriter::open(&path).unwrap();
            for op in sample_ops() {
                w.append(&op).unwrap();
            }
        }
        let res = replay(&path).unwrap();
        assert!(!res.truncated_tail, "no tail corruption expected");
        assert_eq!(res.ops, sample_ops());
        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_reopen_and_append() {
        let path = temp_path();
        {
            let mut w = WalWriter::open(&path).unwrap();
            w.append(&WalOp::Insert {
                id: "a".to_string(),
                vector: Some(vec![1.0]),
                metadata: None,
            })
            .unwrap();
        }
        // Reabrir, appendear más.
        {
            let mut w = WalWriter::open(&path).unwrap();
            w.append(&WalOp::Insert {
                id: "b".to_string(),
                vector: Some(vec![2.0]),
                metadata: None,
            })
            .unwrap();
        }
        let res = replay(&path).unwrap();
        assert!(!res.truncated_tail);
        assert_eq!(res.ops.len(), 2);
        assert_eq!(res.ops[0], WalOp::Insert {
            id: "a".to_string(),
            vector: Some(vec![1.0]),
            metadata: None
        });
        assert_eq!(res.ops[1], WalOp::Insert {
            id: "b".to_string(),
            vector: Some(vec![2.0]),
            metadata: None
        });
        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_empty_log_replays_clean() {
        let path = temp_path();
        {
            let _w = WalWriter::open(&path).unwrap();
        }
        let res = replay(&path).unwrap();
        assert!(!res.truncated_tail);
        assert!(res.ops.is_empty());
        assert_eq!(res.valid_len, WAL_HEADER_SIZE as u64);
        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_broken_tail_replays_valid_and_truncates_on_reopen() {
        let path = temp_path();
        let ops = sample_ops();
        let first_two: Vec<&WalOp> = ops.iter().take(2).collect();
        write_raw(&path, &first_two);

        // Appendear bytes basura truncados al final (torn write).
        {
            use std::io::Seek;
            let mut f = OpenOptions::new()
                .write(true)
                .append(true)
                .open(&path)
                .unwrap();
            // Mezcla: algunos bytes sueltos + un prefijo de longitud sin payload.
            let _ = f.seek(SeekFrom::End(0));
            f.write_all(&[0xAB, 0xCD, 0xEF]).unwrap(); // < 4 bytes → partial length
        }

        // replay devuelve las 2 válidas con truncated_tail=true.
        let res = replay(&path).unwrap();
        assert!(res.truncated_tail, "expected truncated tail");
        assert_eq!(res.ops.len(), 2);
        assert_eq!(res.ops, ops[..2]);

        // Reabrir con WalWriter trunca la cola rota.
        {
            let mut w = WalWriter::open(&path).unwrap();
            w.append(&WalOp::Delete {
                id: "doc-99".to_string(),
            })
            .unwrap();
        }

        // Replay posterior: limpio, sin cola rota, con la nueva entrada.
        let res = replay(&path).unwrap();
        assert!(!res.truncated_tail, "tail should be clean after reopen");
        assert_eq!(res.ops.len(), 3);
        assert_eq!(res.ops[2], WalOp::Delete {
            id: "doc-99".to_string()
        });

        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_broken_tail_garbage_length_prefix_is_tolerated() {
        // Cola rota con un prefijo de longitud plausible pero sin payload.
        let path = temp_path();
        let ops = sample_ops();
        let first: Vec<&WalOp> = ops.iter().take(1).collect();
        write_raw(&path, &first);
        {
            let mut f = OpenOptions::new()
                .write(true)
                .append(true)
                .open(&path)
                .unwrap();
            // Longitud que apunta a un payload que NO existe (torn write).
            f.write_all(&999u32.to_le_bytes()).unwrap();
        }
        let res = replay(&path).unwrap();
        assert!(res.truncated_tail);
        assert_eq!(res.ops.len(), 1);
        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_internal_corruption_is_error() {
        let path = temp_path();
        let ops = sample_ops();
        // Tres entradas; corruptemos la del MEDIO.
        let entries: Vec<&WalOp> = ops.iter().take(3).collect();
        write_raw(&path, &entries);

        // Calcular offset al payload de la 2da entrada y flipar un byte.
        let mut bytes = fs::read(&path).unwrap();
        let e0_len = 4 + bincode::serialize(entries[0]).unwrap().len() + 4;
        // header(8) + e0 + prefijo de longitud de e1 (4) = inicio payload e1
        let payload_e1_start = WAL_HEADER_SIZE + e0_len + 4;
        bytes[payload_e1_start] ^= 0xFF;
        fs::write(&path, &bytes).unwrap();

        let res = replay(&path);
        assert!(res.is_err(), "internal corruption must be an error");
        let msg = format!("{}", res.unwrap_err());
        assert!(
            msg.contains("internal corruption") || msg.contains("CRC32 mismatch"),
            "expected corruption error, got: {}",
            msg
        );
        // NO debe devolver replay parcial silencioso.
        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_truncate_empties_log_and_keeps_reusable() {
        let path = temp_path();
        {
            let mut w = WalWriter::open(&path).unwrap();
            for op in sample_ops() {
                w.append(&op).unwrap();
            }
            w.truncate().unwrap();
        }
        // Log vacío pero reutilizable.
        let res = replay(&path).unwrap();
        assert!(!res.truncated_tail);
        assert!(res.ops.is_empty());
        assert_eq!(res.valid_len, WAL_HEADER_SIZE as u64);

        // Se puede seguir appendeando sobre el log truncado.
        {
            let mut w = WalWriter::open(&path).unwrap();
            w.append(&WalOp::Insert {
                id: "after-truncate".to_string(),
                vector: Some(vec![0.0]),
                metadata: None,
            })
            .unwrap();
        }
        let res = replay(&path).unwrap();
        assert!(!res.truncated_tail);
        assert_eq!(res.ops.len(), 1);

        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_hostile_giant_length_no_oom() {
        let path = temp_path();
        // Header válido + prefijo de longitud gigante (u32::MAX), sin payload.
        let mut bytes = wal_header().to_vec();
        bytes.extend_from_slice(&u32::MAX.to_le_bytes());
        fs::write(&path, &bytes).unwrap();

        let res = replay(&path);
        assert!(res.is_err(), "hostile length must error, not OOM");
        let msg = format!("{}", res.unwrap_err());
        assert!(
            msg.contains("corrupt or malicious file"),
            "expected hostile-file error, got: {}",
            msg
        );

        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_hostile_length_via_enswriter_does_not_oom() {
        // Igual que el anterior, pero abriendo con WalWriter (ensure_file).
        let path = temp_path();
        let mut bytes = wal_header().to_vec();
        bytes.extend_from_slice(&u32::MAX.to_le_bytes());
        fs::write(&path, &bytes).unwrap();

        let res = WalWriter::open(&path);
        assert!(res.is_err(), "hostile length must error on open, not OOM");

        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_fsync_on_append_true_does_not_fail() {
        let path = temp_path();
        {
            let mut w =
                WalWriter::open_with(&path, WalConfig::new().with_fsync_on_append(true)).unwrap();
            for op in sample_ops() {
                w.append(&op).unwrap();
            }
            w.sync().unwrap();
        }
        let res = replay(&path).unwrap();
        assert!(!res.truncated_tail);
        assert_eq!(res.ops, sample_ops());
        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_bad_magic_is_error() {
        let path = temp_path();
        let mut bytes = b"XXXX".to_vec();
        bytes.extend_from_slice(&WAL_VERSION.to_le_bytes());
        bytes.extend_from_slice(&[0u8; 16]);
        fs::write(&path, &bytes).unwrap();

        assert!(replay(&path).is_err());
        assert!(WalWriter::open(&path).is_err());
        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_unsupported_version_is_error() {
        let path = temp_path();
        let mut bytes = WAL_MAGIC.to_vec();
        bytes.extend_from_slice(&999u32.to_le_bytes());
        bytes.extend_from_slice(&[0u8; 16]);
        fs::write(&path, &bytes).unwrap();

        assert!(replay(&path).is_err());
        assert!(WalWriter::open(&path).is_err());
        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_file_smaller_than_header_is_recreated() {
        let path = temp_path();
        fs::write(&path, b"ab").unwrap(); // header incompleto
        // ensure_file debe recrear el header limpio.
        let mut w = WalWriter::open(&path).unwrap();
        w.append(&WalOp::Clear).unwrap();
        drop(w);

        let res = replay(&path).unwrap();
        assert!(!res.truncated_tail);
        assert_eq!(res.ops, vec![WalOp::Clear]);
        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_crc_mismatch_on_last_entry_is_torn_tail() {
        // Una sola entrada con CRC corrupto y ningún byte después:
        // torn write tolerable, no error.
        let path = temp_path();
        let op = WalOp::Insert {
            id: "x".to_string(),
            vector: Some(vec![1.0, 2.0]),
            metadata: None,
        };
        let payload = bincode::serialize(&op).unwrap();
        let len = payload.len() as u32;
        let bad_crc = 0xDEAD_BEEFu32;
        let mut bytes = wal_header().to_vec();
        bytes.extend_from_slice(&len.to_le_bytes());
        bytes.extend_from_slice(&payload);
        bytes.extend_from_slice(&bad_crc.to_le_bytes());
        fs::write(&path, &bytes).unwrap();

        let res = replay(&path).unwrap();
        assert!(res.truncated_tail, "last-entry CRC mismatch is torn tail");
        assert!(res.ops.is_empty());

        fs::remove_file(&path).ok();
    }
}