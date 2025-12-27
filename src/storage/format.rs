//! Formato de archivo para persistencia de la base de datos vectorial.
//!
//! Formato `.mmdb` (MiniMemory DataBase):
//!
//! ```text
//! [Header: 64 bytes]
//! [Vector Data Section]
//! [Index Section]
//! [Footer: 8 bytes]
//! ```

use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

use crate::distance::Distance;
use crate::error::{Error, Result};
use crate::index::IndexType;

/// Magic bytes para identificar archivos .mmdb
pub const MAGIC: &[u8; 4] = b"MMDB";

/// Versión actual del formato
pub const VERSION: u32 = 1;

/// Tamaño del header en bytes
pub const HEADER_SIZE: usize = 64;

/// Header del archivo .mmdb
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileHeader {
    /// Número de dimensiones
    pub dimensions: u32,
    /// Número de vectores
    pub num_vectors: u64,
    /// Tipo de distancia (0=Cosine, 1=Euclidean, 2=DotProduct)
    pub distance_type: u8,
    /// Tipo de índice (0=Flat, 1=HNSW)
    pub index_type: u8,
    /// Parámetro M para HNSW (0 si es Flat)
    pub hnsw_m: u16,
    /// Parámetro ef_construction para HNSW (0 si es Flat)
    pub hnsw_ef: u16,
    /// Offset donde comienzan los datos de vectores
    pub data_offset: u64,
    /// Offset donde comienza el índice
    pub index_offset: u64,
}

impl FileHeader {
    /// Crea un nuevo header
    pub fn new(
        dimensions: usize,
        num_vectors: usize,
        distance: Distance,
        index: &IndexType,
    ) -> Self {
        let (index_type, hnsw_m, hnsw_ef) = match index {
            IndexType::Flat => (0, 0, 0),
            IndexType::HNSW { m, ef_construction } => (1, *m as u16, *ef_construction as u16),
        };

        Self {
            dimensions: dimensions as u32,
            num_vectors: num_vectors as u64,
            distance_type: distance.to_u8(),
            index_type,
            hnsw_m,
            hnsw_ef,
            data_offset: HEADER_SIZE as u64,
            index_offset: 0, // Se actualiza después
        }
    }

    /// Escribe el header a un writer
    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        // Magic bytes
        writer.write_all(MAGIC)?;

        // Version
        writer.write_all(&VERSION.to_le_bytes())?;

        // Dimensions
        writer.write_all(&self.dimensions.to_le_bytes())?;

        // Num vectors
        writer.write_all(&self.num_vectors.to_le_bytes())?;

        // Distance type
        writer.write_all(&[self.distance_type])?;

        // Index type
        writer.write_all(&[self.index_type])?;

        // HNSW params
        writer.write_all(&self.hnsw_m.to_le_bytes())?;
        writer.write_all(&self.hnsw_ef.to_le_bytes())?;

        // Data offset
        writer.write_all(&self.data_offset.to_le_bytes())?;

        // Index offset
        writer.write_all(&self.index_offset.to_le_bytes())?;

        // Reserved (padding to 64 bytes)
        // 4 + 4 + 4 + 8 + 1 + 1 + 2 + 2 + 8 + 8 = 42 bytes used
        let padding = vec![0u8; HEADER_SIZE - 42];
        writer.write_all(&padding)?;

        Ok(())
    }

    /// Lee el header desde un reader
    pub fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;

        if &magic != MAGIC {
            return Err(Error::InvalidConfig("Invalid file format: bad magic bytes".into()));
        }

        let mut buf4 = [0u8; 4];
        let mut buf8 = [0u8; 8];
        let mut buf2 = [0u8; 2];
        let mut buf1 = [0u8; 1];

        // Version
        reader.read_exact(&mut buf4)?;
        let version = u32::from_le_bytes(buf4);
        if version != VERSION {
            return Err(Error::InvalidConfig(format!(
                "Unsupported file version: {}",
                version
            )));
        }

        // Dimensions
        reader.read_exact(&mut buf4)?;
        let dimensions = u32::from_le_bytes(buf4);

        // Num vectors
        reader.read_exact(&mut buf8)?;
        let num_vectors = u64::from_le_bytes(buf8);

        // Distance type
        reader.read_exact(&mut buf1)?;
        let distance_type = buf1[0];

        // Index type
        reader.read_exact(&mut buf1)?;
        let index_type = buf1[0];

        // HNSW params
        reader.read_exact(&mut buf2)?;
        let hnsw_m = u16::from_le_bytes(buf2);

        reader.read_exact(&mut buf2)?;
        let hnsw_ef = u16::from_le_bytes(buf2);

        // Data offset
        reader.read_exact(&mut buf8)?;
        let data_offset = u64::from_le_bytes(buf8);

        // Index offset
        reader.read_exact(&mut buf8)?;
        let index_offset = u64::from_le_bytes(buf8);

        // Skip reserved bytes
        let mut reserved = vec![0u8; HEADER_SIZE - 42];
        reader.read_exact(&mut reserved)?;

        Ok(Self {
            dimensions,
            num_vectors,
            distance_type,
            index_type,
            hnsw_m,
            hnsw_ef,
            data_offset,
            index_offset,
        })
    }

    /// Obtiene el tipo de distancia
    pub fn get_distance(&self) -> Distance {
        Distance::from_u8(self.distance_type)
    }

    /// Obtiene el tipo de índice
    pub fn get_index_type(&self) -> IndexType {
        match self.index_type {
            1 => IndexType::HNSW {
                m: self.hnsw_m as usize,
                ef_construction: self.hnsw_ef as usize,
            },
            _ => IndexType::Flat,
        }
    }
}

/// Entrada de documento en el archivo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorEntry {
    pub id: String,
    /// Vector embedding (None for metadata-only documents)
    pub vector: Option<Vec<f32>>,
    pub metadata: Option<crate::types::Metadata>,
}

impl Distance {
    /// Convierte a u8 para serialización
    pub fn to_u8(&self) -> u8 {
        match self {
            Distance::Cosine => 0,
            Distance::Euclidean => 1,
            Distance::DotProduct => 2,
        }
    }

    /// Convierte desde u8
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Distance::Euclidean,
            2 => Distance::DotProduct,
            _ => Distance::Cosine,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_header_roundtrip() {
        let header = FileHeader::new(
            384,
            1000,
            Distance::Cosine,
            &IndexType::HNSW {
                m: 16,
                ef_construction: 200,
            },
        );

        let mut buffer = Vec::new();
        header.write_to(&mut buffer).unwrap();

        assert_eq!(buffer.len(), HEADER_SIZE);

        let mut cursor = Cursor::new(buffer);
        let read_header = FileHeader::read_from(&mut cursor).unwrap();

        assert_eq!(read_header.dimensions, 384);
        assert_eq!(read_header.num_vectors, 1000);
        assert_eq!(read_header.distance_type, 0); // Cosine
        assert_eq!(read_header.index_type, 1); // HNSW
        assert_eq!(read_header.hnsw_m, 16);
        assert_eq!(read_header.hnsw_ef, 200);
    }

    #[test]
    fn test_invalid_magic() {
        let buffer = vec![0u8; HEADER_SIZE];
        let mut cursor = Cursor::new(buffer);
        let result = FileHeader::read_from(&mut cursor);
        assert!(result.is_err());
    }
}
