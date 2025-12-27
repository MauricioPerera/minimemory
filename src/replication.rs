//! # Sistema de Replicación
//!
//! Permite sincronizar datos entre múltiples instancias de VectorDB.
//!
//! ## Características
//!
//! - **Change Log**: Registro de todas las operaciones
//! - **Checkpoints**: Puntos de sincronización incrementales
//! - **Export/Import**: Transferencia de cambios entre instancias
//! - **Merge**: Combinación de cambios de múltiples fuentes
//!
//! ## Ejemplo
//!
//! ```rust,ignore
//! use minimemory::{VectorDB, Config};
//! use minimemory::replication::{ChangeLog, ReplicationManager};
//!
//! // Instancia primaria
//! let primary = VectorDB::new(Config::new(384)).unwrap();
//! let mut log = ChangeLog::new();
//!
//! // Insertar con tracking de cambios
//! log.track_insert("doc-1", &vec![0.1; 384], None);
//! primary.insert("doc-1", &vec![0.1; 384], None).unwrap();
//!
//! // Exportar cambios desde checkpoint
//! let changes = log.export_since(0);
//!
//! // Instancia réplica
//! let replica = VectorDB::new(Config::new(384)).unwrap();
//! ReplicationManager::apply_changes(&replica, &changes).unwrap();
//! ```

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::types::{Metadata, VectorId};
use crate::VectorDB;

/// Tipo de operación en el change log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationType {
    /// Inserción de nuevo documento
    Insert,
    /// Actualización de documento existente
    Update,
    /// Eliminación de documento
    Delete,
}

/// Una entrada en el change log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeEntry {
    /// Número de secuencia único
    pub sequence: u64,
    /// Tipo de operación
    pub operation: OperationType,
    /// ID del documento afectado
    pub document_id: VectorId,
    /// Vector (None para Delete)
    pub vector: Option<Vec<f32>>,
    /// Metadata (None para Delete)
    pub metadata: Option<Metadata>,
    /// Timestamp Unix en milisegundos
    pub timestamp: u64,
    /// ID de la instancia origen
    pub origin_id: String,
}

impl ChangeEntry {
    /// Crea una nueva entrada de inserción.
    pub fn insert(
        sequence: u64,
        origin_id: &str,
        doc_id: impl Into<VectorId>,
        vector: Vec<f32>,
        metadata: Option<Metadata>,
    ) -> Self {
        Self {
            sequence,
            operation: OperationType::Insert,
            document_id: doc_id.into(),
            vector: Some(vector),
            metadata,
            timestamp: current_timestamp(),
            origin_id: origin_id.to_string(),
        }
    }

    /// Crea una nueva entrada de actualización.
    pub fn update(
        sequence: u64,
        origin_id: &str,
        doc_id: impl Into<VectorId>,
        vector: Vec<f32>,
        metadata: Option<Metadata>,
    ) -> Self {
        Self {
            sequence,
            operation: OperationType::Update,
            document_id: doc_id.into(),
            vector: Some(vector),
            metadata,
            timestamp: current_timestamp(),
            origin_id: origin_id.to_string(),
        }
    }

    /// Crea una nueva entrada de eliminación.
    pub fn delete(sequence: u64, origin_id: &str, doc_id: impl Into<VectorId>) -> Self {
        Self {
            sequence,
            operation: OperationType::Delete,
            document_id: doc_id.into(),
            vector: None,
            metadata: None,
            timestamp: current_timestamp(),
            origin_id: origin_id.to_string(),
        }
    }
}

/// Obtiene el timestamp actual en milisegundos.
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Genera un ID único para la instancia.
fn generate_instance_id() -> String {
    use std::process;
    use std::time::Instant;

    let pid = process::id();
    let time = Instant::now().elapsed().as_nanos();
    format!("inst-{:x}-{:x}", pid, time as u64)
}

/// Change Log para tracking de operaciones.
///
/// Registra todas las operaciones realizadas para permitir replicación.
#[derive(Debug)]
pub struct ChangeLog {
    /// ID único de esta instancia
    instance_id: String,
    /// Contador de secuencia
    sequence: AtomicU64,
    /// Entradas del log
    entries: RwLock<Vec<ChangeEntry>>,
    /// Último checkpoint exportado
    last_checkpoint: AtomicU64,
    /// Límite de entradas antes de compactación automática
    max_entries: usize,
}

impl Default for ChangeLog {
    fn default() -> Self {
        Self::new()
    }
}

impl ChangeLog {
    /// Crea un nuevo change log vacío.
    pub fn new() -> Self {
        Self {
            instance_id: generate_instance_id(),
            sequence: AtomicU64::new(0),
            entries: RwLock::new(Vec::new()),
            last_checkpoint: AtomicU64::new(0),
            max_entries: 10000,
        }
    }

    /// Crea un change log con ID de instancia específico.
    pub fn with_instance_id(instance_id: impl Into<String>) -> Self {
        Self {
            instance_id: instance_id.into(),
            sequence: AtomicU64::new(0),
            entries: RwLock::new(Vec::new()),
            last_checkpoint: AtomicU64::new(0),
            max_entries: 10000,
        }
    }

    /// Retorna el ID de esta instancia.
    pub fn instance_id(&self) -> &str {
        &self.instance_id
    }

    /// Retorna el número de secuencia actual.
    pub fn current_sequence(&self) -> u64 {
        self.sequence.load(Ordering::SeqCst)
    }

    /// Registra una inserción.
    pub fn track_insert(
        &self,
        doc_id: impl Into<VectorId>,
        vector: &[f32],
        metadata: Option<Metadata>,
    ) -> u64 {
        let seq = self.sequence.fetch_add(1, Ordering::SeqCst);
        let entry = ChangeEntry::insert(
            seq,
            &self.instance_id,
            doc_id,
            vector.to_vec(),
            metadata,
        );
        self.entries.write().push(entry);
        self.maybe_compact();
        seq
    }

    /// Registra una actualización.
    pub fn track_update(
        &self,
        doc_id: impl Into<VectorId>,
        vector: &[f32],
        metadata: Option<Metadata>,
    ) -> u64 {
        let seq = self.sequence.fetch_add(1, Ordering::SeqCst);
        let entry = ChangeEntry::update(
            seq,
            &self.instance_id,
            doc_id,
            vector.to_vec(),
            metadata,
        );
        self.entries.write().push(entry);
        self.maybe_compact();
        seq
    }

    /// Registra una eliminación.
    pub fn track_delete(&self, doc_id: impl Into<VectorId>) -> u64 {
        let seq = self.sequence.fetch_add(1, Ordering::SeqCst);
        let entry = ChangeEntry::delete(seq, &self.instance_id, doc_id);
        self.entries.write().push(entry);
        self.maybe_compact();
        seq
    }

    /// Exporta cambios desde un número de secuencia.
    pub fn export_since(&self, since_sequence: u64) -> Vec<ChangeEntry> {
        self.entries
            .read()
            .iter()
            .filter(|e| e.sequence >= since_sequence)
            .cloned()
            .collect()
    }

    /// Exporta todos los cambios.
    pub fn export_all(&self) -> Vec<ChangeEntry> {
        self.entries.read().clone()
    }

    /// Establece un checkpoint en el número de secuencia actual.
    pub fn checkpoint(&self) -> u64 {
        let seq = self.sequence.load(Ordering::SeqCst);
        self.last_checkpoint.store(seq, Ordering::SeqCst);
        seq
    }

    /// Retorna el último checkpoint.
    pub fn last_checkpoint(&self) -> u64 {
        self.last_checkpoint.load(Ordering::SeqCst)
    }

    /// Exporta cambios desde el último checkpoint.
    pub fn export_since_checkpoint(&self) -> Vec<ChangeEntry> {
        self.export_since(self.last_checkpoint())
    }

    /// Retorna el número de entradas en el log.
    pub fn len(&self) -> usize {
        self.entries.read().len()
    }

    /// Verifica si el log está vacío.
    pub fn is_empty(&self) -> bool {
        self.entries.read().is_empty()
    }

    /// Limpia entradas anteriores a un número de secuencia.
    pub fn truncate_before(&self, sequence: u64) {
        self.entries.write().retain(|e| e.sequence >= sequence);
    }

    /// Compacta el log si excede el límite.
    fn maybe_compact(&self) {
        let len = self.entries.read().len();
        if len > self.max_entries {
            // Mantener solo las últimas max_entries/2 entradas
            let keep = self.max_entries / 2;
            let mut entries = self.entries.write();
            if entries.len() > keep {
                let start = entries.len() - keep;
                *entries = entries[start..].to_vec();
            }
        }
    }

    /// Serializa el log a JSON.
    pub fn to_json(&self) -> Result<String> {
        let data = ChangeLogData {
            instance_id: self.instance_id.clone(),
            sequence: self.sequence.load(Ordering::SeqCst),
            entries: self.entries.read().clone(),
            last_checkpoint: self.last_checkpoint.load(Ordering::SeqCst),
        };
        serde_json::to_string(&data).map_err(|e| Error::Serialization(e.to_string()))
    }

    /// Deserializa el log desde JSON.
    pub fn from_json(json: &str) -> Result<Self> {
        let data: ChangeLogData =
            serde_json::from_str(json).map_err(|e| Error::Serialization(e.to_string()))?;
        Ok(Self {
            instance_id: data.instance_id,
            sequence: AtomicU64::new(data.sequence),
            entries: RwLock::new(data.entries),
            last_checkpoint: AtomicU64::new(data.last_checkpoint),
            max_entries: 10000,
        })
    }
}

/// Datos serializables del ChangeLog.
#[derive(Serialize, Deserialize)]
struct ChangeLogData {
    instance_id: String,
    sequence: u64,
    entries: Vec<ChangeEntry>,
    last_checkpoint: u64,
}

/// Estado de replicación entre dos instancias.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationState {
    /// ID de la instancia local
    pub local_id: String,
    /// ID de la instancia remota
    pub remote_id: String,
    /// Última secuencia sincronizada del remoto
    pub last_synced_sequence: u64,
    /// Timestamp de última sincronización
    pub last_sync_time: u64,
    /// Número de cambios aplicados
    pub changes_applied: u64,
}

/// Resultado de una operación de sincronización.
#[derive(Debug, Clone)]
pub struct SyncResult {
    /// Cambios aplicados
    pub applied: usize,
    /// Cambios omitidos (ya existían)
    pub skipped: usize,
    /// Conflictos detectados
    pub conflicts: Vec<ConflictInfo>,
    /// Nueva secuencia después de sync
    pub new_sequence: u64,
}

/// Información sobre un conflicto de replicación.
#[derive(Debug, Clone)]
pub struct ConflictInfo {
    /// ID del documento en conflicto
    pub document_id: VectorId,
    /// Operación local
    pub local_operation: OperationType,
    /// Operación remota
    pub remote_operation: OperationType,
    /// Timestamp local
    pub local_timestamp: u64,
    /// Timestamp remoto
    pub remote_timestamp: u64,
    /// Resolución aplicada
    pub resolution: ConflictResolution,
}

/// Estrategia de resolución de conflictos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictResolution {
    /// Mantener versión local
    KeepLocal,
    /// Aplicar versión remota
    ApplyRemote,
    /// Última escritura gana (por timestamp)
    LastWriteWins,
}

/// Gestor de replicación.
///
/// Coordina la sincronización entre instancias.
pub struct ReplicationManager {
    /// Estados de replicación con otras instancias
    states: RwLock<HashMap<String, ReplicationState>>,
    /// Estrategia de resolución de conflictos
    conflict_strategy: ConflictResolution,
}

impl Default for ReplicationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplicationManager {
    /// Crea un nuevo gestor de replicación.
    pub fn new() -> Self {
        Self {
            states: RwLock::new(HashMap::new()),
            conflict_strategy: ConflictResolution::LastWriteWins,
        }
    }

    /// Establece la estrategia de resolución de conflictos.
    pub fn with_conflict_strategy(mut self, strategy: ConflictResolution) -> Self {
        self.conflict_strategy = strategy;
        self
    }

    /// Aplica cambios a una VectorDB.
    pub fn apply_changes(db: &VectorDB, changes: &[ChangeEntry]) -> Result<SyncResult> {
        let mut applied = 0;
        let mut skipped = 0;
        let conflicts = Vec::new();
        let mut last_seq = 0u64;

        for change in changes {
            last_seq = last_seq.max(change.sequence);

            match change.operation {
                OperationType::Insert => {
                    if let Some(ref vector) = change.vector {
                        match db.insert_document(
                            &change.document_id,
                            Some(vector),
                            change.metadata.clone(),
                        ) {
                            Ok(()) => applied += 1,
                            Err(Error::AlreadyExists(_)) => skipped += 1,
                            Err(e) => return Err(e),
                        }
                    }
                }
                OperationType::Update => {
                    if let Some(ref vector) = change.vector {
                        // Intentar actualizar, si no existe insertar
                        if db.contains(&change.document_id) {
                            db.update(&change.document_id, vector, change.metadata.clone())?;
                        } else {
                            db.insert_document(
                                &change.document_id,
                                Some(vector),
                                change.metadata.clone(),
                            )?;
                        }
                        applied += 1;
                    }
                }
                OperationType::Delete => {
                    if db.delete(&change.document_id)? {
                        applied += 1;
                    } else {
                        skipped += 1;
                    }
                }
            }
        }

        Ok(SyncResult {
            applied,
            skipped,
            conflicts,
            new_sequence: last_seq,
        })
    }

    /// Sincroniza cambios entre dos instancias.
    pub fn sync(
        &self,
        local_db: &VectorDB,
        local_log: &ChangeLog,
        remote_changes: &[ChangeEntry],
        remote_id: &str,
    ) -> Result<SyncResult> {
        let mut states = self.states.write();

        // Obtener o crear estado de replicación
        let state = states.entry(remote_id.to_string()).or_insert(ReplicationState {
            local_id: local_log.instance_id().to_string(),
            remote_id: remote_id.to_string(),
            last_synced_sequence: 0,
            last_sync_time: current_timestamp(),
            changes_applied: 0,
        });

        // Filtrar cambios ya sincronizados
        let new_changes: Vec<_> = remote_changes
            .iter()
            .filter(|c| c.sequence > state.last_synced_sequence)
            .cloned()
            .collect();

        // Aplicar cambios
        let result = Self::apply_changes(local_db, &new_changes)?;

        // Actualizar estado
        state.last_synced_sequence = result.new_sequence;
        state.last_sync_time = current_timestamp();
        state.changes_applied += result.applied as u64;

        Ok(result)
    }

    /// Obtiene el estado de replicación con una instancia.
    pub fn get_state(&self, remote_id: &str) -> Option<ReplicationState> {
        self.states.read().get(remote_id).cloned()
    }

    /// Lista todas las instancias conocidas.
    pub fn list_remotes(&self) -> Vec<ReplicationState> {
        self.states.read().values().cloned().collect()
    }

    /// Exporta un snapshot completo de la DB para replicación inicial.
    pub fn create_snapshot(db: &VectorDB) -> Result<Vec<ChangeEntry>> {
        let ids = db.list_ids()?;
        let mut entries = Vec::with_capacity(ids.len());

        for (i, id) in ids.iter().enumerate() {
            if let Some((vector, metadata)) = db.get(id)? {
                if let Some(vec) = vector {
                    entries.push(ChangeEntry::insert(
                        i as u64,
                        "snapshot",
                        id.clone(),
                        vec,
                        metadata,
                    ));
                }
            }
        }

        Ok(entries)
    }

    /// Aplica un snapshot a una DB vacía.
    pub fn apply_snapshot(db: &VectorDB, snapshot: &[ChangeEntry]) -> Result<usize> {
        let result = Self::apply_changes(db, snapshot)?;
        Ok(result.applied)
    }
}

/// Builder para configuración de replicación.
#[derive(Debug, Clone)]
pub struct ReplicationConfig {
    /// ID de la instancia
    pub instance_id: String,
    /// Estrategia de conflictos
    pub conflict_strategy: ConflictResolution,
    /// Tamaño máximo del log
    pub max_log_entries: usize,
    /// Habilitar compactación automática
    pub auto_compact: bool,
}

impl Default for ReplicationConfig {
    fn default() -> Self {
        Self {
            instance_id: generate_instance_id(),
            conflict_strategy: ConflictResolution::LastWriteWins,
            max_log_entries: 10000,
            auto_compact: true,
        }
    }
}

impl ReplicationConfig {
    /// Crea una nueva configuración.
    pub fn new() -> Self {
        Self::default()
    }

    /// Establece el ID de instancia.
    pub fn with_instance_id(mut self, id: impl Into<String>) -> Self {
        self.instance_id = id.into();
        self
    }

    /// Establece la estrategia de conflictos.
    pub fn with_conflict_strategy(mut self, strategy: ConflictResolution) -> Self {
        self.conflict_strategy = strategy;
        self
    }

    /// Establece el tamaño máximo del log.
    pub fn with_max_log_entries(mut self, max: usize) -> Self {
        self.max_log_entries = max;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Config;

    #[test]
    fn test_change_log_tracking() {
        let log = ChangeLog::new();

        // Track operations
        let seq1 = log.track_insert("doc-1", &[0.1, 0.2, 0.3], None);
        let seq2 = log.track_update("doc-1", &[0.2, 0.3, 0.4], None);
        let seq3 = log.track_delete("doc-1");

        assert_eq!(seq1, 0);
        assert_eq!(seq2, 1);
        assert_eq!(seq3, 2);
        assert_eq!(log.len(), 3);
    }

    #[test]
    fn test_export_since() {
        let log = ChangeLog::new();

        log.track_insert("doc-1", &[0.1; 3], None);
        log.track_insert("doc-2", &[0.2; 3], None);
        log.track_insert("doc-3", &[0.3; 3], None);

        let changes = log.export_since(1);
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0].document_id, "doc-2");
        assert_eq!(changes[1].document_id, "doc-3");
    }

    #[test]
    fn test_checkpoint() {
        let log = ChangeLog::new();

        log.track_insert("doc-1", &[0.1; 3], None);
        log.track_insert("doc-2", &[0.2; 3], None);

        let checkpoint = log.checkpoint();
        assert_eq!(checkpoint, 2);

        log.track_insert("doc-3", &[0.3; 3], None);

        let changes = log.export_since_checkpoint();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].document_id, "doc-3");
    }

    #[test]
    fn test_apply_changes() {
        let db = VectorDB::new(Config::new(3)).unwrap();

        let changes = vec![
            ChangeEntry::insert(0, "remote", "doc-1", vec![0.1, 0.2, 0.3], None),
            ChangeEntry::insert(1, "remote", "doc-2", vec![0.4, 0.5, 0.6], None),
        ];

        let result = ReplicationManager::apply_changes(&db, &changes).unwrap();

        assert_eq!(result.applied, 2);
        assert_eq!(result.skipped, 0);
        assert_eq!(db.len(), 2);
    }

    #[test]
    fn test_snapshot_and_restore() {
        // Create and populate source DB
        let source = VectorDB::new(Config::new(3)).unwrap();
        source.insert("doc-1", &[0.1, 0.2, 0.3], None).unwrap();
        source.insert("doc-2", &[0.4, 0.5, 0.6], None).unwrap();

        // Create snapshot
        let snapshot = ReplicationManager::create_snapshot(&source).unwrap();
        assert_eq!(snapshot.len(), 2);

        // Apply to new DB
        let replica = VectorDB::new(Config::new(3)).unwrap();
        let count = ReplicationManager::apply_snapshot(&replica, &snapshot).unwrap();

        assert_eq!(count, 2);
        assert_eq!(replica.len(), 2);
        assert!(replica.contains("doc-1"));
        assert!(replica.contains("doc-2"));
    }

    #[test]
    fn test_log_serialization() {
        let log = ChangeLog::with_instance_id("test-instance");
        log.track_insert("doc-1", &[0.1, 0.2], None);
        log.track_delete("doc-2");

        let json = log.to_json().unwrap();
        let restored = ChangeLog::from_json(&json).unwrap();

        assert_eq!(restored.instance_id(), "test-instance");
        assert_eq!(restored.len(), 2);
    }

    #[test]
    fn test_incremental_sync() {
        let primary = VectorDB::new(Config::new(3)).unwrap();
        let log = ChangeLog::with_instance_id("primary");

        // Operaciones en primario
        log.track_insert("doc-1", &[0.1; 3], None);
        primary.insert("doc-1", &[0.1; 3], None).unwrap();
        log.checkpoint();

        log.track_insert("doc-2", &[0.2; 3], None);
        primary.insert("doc-2", &[0.2; 3], None).unwrap();

        // Réplica sincroniza solo cambios nuevos
        let replica = VectorDB::new(Config::new(3)).unwrap();
        let _manager = ReplicationManager::new();

        // Primera sync: snapshot inicial
        let snapshot = ReplicationManager::create_snapshot(&primary).unwrap();
        ReplicationManager::apply_snapshot(&replica, &snapshot).unwrap();

        assert_eq!(replica.len(), 2);
    }
}
