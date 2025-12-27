//! Bindings C/FFI para minimemory.
//!
//! Estos bindings permiten usar minimemory desde cualquier lenguaje
//! que soporte FFI con C, incluyendo PHP, Ruby, etc.
//!
//! ## Uso en PHP
//!
//! ```php
//! <?php
//! $ffi = FFI::cdef("
//!     typedef struct MiniMemoryDB MiniMemoryDB;
//!     typedef struct SearchResult {
//!         char* id;
//!         float distance;
//!     } SearchResult;
//!
//!     MiniMemoryDB* mmdb_new(uint32_t dimensions, const char* distance, const char* index_type);
//!     void mmdb_free(MiniMemoryDB* db);
//!     int mmdb_insert(MiniMemoryDB* db, const char* id, const float* vector, uint32_t len);
//!     SearchResult* mmdb_search(MiniMemoryDB* db, const float* query, uint32_t len, uint32_t k, uint32_t* result_count);
//!     void mmdb_free_results(SearchResult* results, uint32_t count);
//!     int mmdb_save(MiniMemoryDB* db, const char* path);
//!     MiniMemoryDB* mmdb_load(const char* path);
//! ", "libminimemory.so");
//!
//! // Crear base de datos
//! $db = $ffi->mmdb_new(384, "cosine", "flat");
//!
//! // Insertar vector
//! $vector = FFI::new("float[384]");
//! for ($i = 0; $i < 384; $i++) $vector[$i] = 0.1;
//! $ffi->mmdb_insert($db, "doc1", $vector, 384);
//!
//! // Buscar
//! $count = FFI::new("uint32_t");
//! $results = $ffi->mmdb_search($db, $vector, 384, 10, FFI::addr($count));
//!
//! // Limpiar
//! $ffi->mmdb_free_results($results, $count->cdata);
//! $ffi->mmdb_free($db);
//! ?>
//! ```

use std::ffi::{c_char, c_float, c_int, CStr, CString};
use std::ptr;
use std::sync::Arc;

use crate::{
    Config as RustConfig,
    Distance as RustDistance,
    IndexType as RustIndexType,
    VectorDB as RustVectorDB,
};

/// Estructura opaca para la base de datos
pub struct MiniMemoryDB {
    inner: Arc<RustVectorDB>,
}

/// Resultado de búsqueda para FFI
#[repr(C)]
pub struct SearchResult {
    pub id: *mut c_char,
    pub distance: c_float,
}

/// Crea una nueva base de datos.
///
/// # Argumentos
/// * `dimensions` - Número de dimensiones
/// * `distance` - "cosine", "euclidean", o "dot"
/// * `index_type` - "flat" o "hnsw"
///
/// # Retorna
/// Puntero a la base de datos o NULL si hay error.
#[no_mangle]
pub extern "C" fn mmdb_new(
    dimensions: u32,
    distance: *const c_char,
    index_type: *const c_char,
) -> *mut MiniMemoryDB {
    let dist_str = unsafe {
        if distance.is_null() {
            "cosine"
        } else {
            match CStr::from_ptr(distance).to_str() {
                Ok(s) => s,
                Err(_) => return ptr::null_mut(),
            }
        }
    };

    let idx_str = unsafe {
        if index_type.is_null() {
            "flat"
        } else {
            match CStr::from_ptr(index_type).to_str() {
                Ok(s) => s,
                Err(_) => return ptr::null_mut(),
            }
        }
    };

    let dist = match dist_str {
        "cosine" | "cos" => RustDistance::Cosine,
        "euclidean" | "l2" => RustDistance::Euclidean,
        "dot" | "dot_product" => RustDistance::DotProduct,
        _ => return ptr::null_mut(),
    };

    let idx = match idx_str {
        "flat" => RustIndexType::Flat,
        "hnsw" => RustIndexType::HNSW { m: 16, ef_construction: 200 },
        _ => return ptr::null_mut(),
    };

    let config = RustConfig::new(dimensions as usize)
        .with_distance(dist)
        .with_index(idx);

    match RustVectorDB::new(config) {
        Ok(db) => Box::into_raw(Box::new(MiniMemoryDB { inner: Arc::new(db) })),
        Err(_) => ptr::null_mut(),
    }
}

/// Libera la memoria de la base de datos.
#[no_mangle]
pub extern "C" fn mmdb_free(db: *mut MiniMemoryDB) {
    if !db.is_null() {
        unsafe {
            drop(Box::from_raw(db));
        }
    }
}

/// Inserta un vector en la base de datos.
///
/// # Retorna
/// 0 si éxito, -1 si error.
#[no_mangle]
pub extern "C" fn mmdb_insert(
    db: *mut MiniMemoryDB,
    id: *const c_char,
    vector: *const c_float,
    len: u32,
) -> c_int {
    if db.is_null() || id.is_null() || vector.is_null() {
        return -1;
    }

    let db = unsafe { &*db };
    let id_str = unsafe {
        match CStr::from_ptr(id).to_str() {
            Ok(s) => s,
            Err(_) => return -1,
        }
    };

    let vec: Vec<f32> = unsafe {
        std::slice::from_raw_parts(vector, len as usize).to_vec()
    };

    match db.inner.insert(id_str, &vec, None) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Busca los k vectores más similares.
///
/// # Argumentos
/// * `db` - Base de datos
/// * `query` - Vector de consulta
/// * `len` - Longitud del vector
/// * `k` - Número de resultados
/// * `result_count` - Puntero donde se escribirá el número de resultados
///
/// # Retorna
/// Array de SearchResult. Debe liberarse con mmdb_free_results.
#[no_mangle]
pub extern "C" fn mmdb_search(
    db: *mut MiniMemoryDB,
    query: *const c_float,
    len: u32,
    k: u32,
    result_count: *mut u32,
) -> *mut SearchResult {
    if db.is_null() || query.is_null() || result_count.is_null() {
        return ptr::null_mut();
    }

    let db = unsafe { &*db };
    let query_vec: Vec<f32> = unsafe {
        std::slice::from_raw_parts(query, len as usize).to_vec()
    };

    match db.inner.search(&query_vec, k as usize) {
        Ok(results) => {
            let count = results.len();
            unsafe { *result_count = count as u32 };

            if count == 0 {
                return ptr::null_mut();
            }

            let mut ffi_results: Vec<SearchResult> = results
                .into_iter()
                .map(|r| {
                    let id_cstring = CString::new(r.id).unwrap_or_default();
                    SearchResult {
                        id: id_cstring.into_raw(),
                        distance: r.distance,
                    }
                })
                .collect();

            let ptr = ffi_results.as_mut_ptr();
            std::mem::forget(ffi_results);
            ptr
        }
        Err(_) => {
            unsafe { *result_count = 0 };
            ptr::null_mut()
        }
    }
}

/// Libera los resultados de búsqueda.
#[no_mangle]
pub extern "C" fn mmdb_free_results(results: *mut SearchResult, count: u32) {
    if results.is_null() || count == 0 {
        return;
    }

    unsafe {
        let slice = std::slice::from_raw_parts_mut(results, count as usize);
        for result in slice.iter() {
            if !result.id.is_null() {
                drop(CString::from_raw(result.id));
            }
        }
        drop(Vec::from_raw_parts(results, count as usize, count as usize));
    }
}

/// Obtiene un vector por su ID.
///
/// # Retorna
/// Puntero al vector (debe liberarse con mmdb_free_vector) o NULL si no existe.
#[no_mangle]
pub extern "C" fn mmdb_get(
    db: *mut MiniMemoryDB,
    id: *const c_char,
    len: *mut u32,
) -> *mut c_float {
    if db.is_null() || id.is_null() || len.is_null() {
        return ptr::null_mut();
    }

    let db = unsafe { &*db };
    let id_str = unsafe {
        match CStr::from_ptr(id).to_str() {
            Ok(s) => s,
            Err(_) => return ptr::null_mut(),
        }
    };

    match db.inner.get(id_str) {
        Ok(Some((vector, _))) => {
            unsafe { *len = vector.len() as u32 };
            let mut boxed = vector.into_boxed_slice();
            let ptr = boxed.as_mut_ptr();
            std::mem::forget(boxed);
            ptr
        }
        _ => {
            unsafe { *len = 0 };
            ptr::null_mut()
        }
    }
}

/// Libera un vector obtenido con mmdb_get.
#[no_mangle]
pub extern "C" fn mmdb_free_vector(vector: *mut c_float, len: u32) {
    if !vector.is_null() && len > 0 {
        unsafe {
            drop(Vec::from_raw_parts(vector, len as usize, len as usize));
        }
    }
}

/// Elimina un vector por su ID.
///
/// # Retorna
/// 1 si fue eliminado, 0 si no existía, -1 si error.
#[no_mangle]
pub extern "C" fn mmdb_delete(db: *mut MiniMemoryDB, id: *const c_char) -> c_int {
    if db.is_null() || id.is_null() {
        return -1;
    }

    let db = unsafe { &*db };
    let id_str = unsafe {
        match CStr::from_ptr(id).to_str() {
            Ok(s) => s,
            Err(_) => return -1,
        }
    };

    match db.inner.delete(id_str) {
        Ok(true) => 1,
        Ok(false) => 0,
        Err(_) => -1,
    }
}

/// Verifica si un vector existe.
///
/// # Retorna
/// 1 si existe, 0 si no.
#[no_mangle]
pub extern "C" fn mmdb_contains(db: *mut MiniMemoryDB, id: *const c_char) -> c_int {
    if db.is_null() || id.is_null() {
        return 0;
    }

    let db = unsafe { &*db };
    let id_str = unsafe {
        match CStr::from_ptr(id).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        }
    };

    if db.inner.contains(id_str) { 1 } else { 0 }
}

/// Guarda la base de datos a un archivo.
///
/// # Retorna
/// 0 si éxito, -1 si error.
#[no_mangle]
pub extern "C" fn mmdb_save(db: *mut MiniMemoryDB, path: *const c_char) -> c_int {
    if db.is_null() || path.is_null() {
        return -1;
    }

    let db = unsafe { &*db };
    let path_str = unsafe {
        match CStr::from_ptr(path).to_str() {
            Ok(s) => s,
            Err(_) => return -1,
        }
    };

    match db.inner.save(path_str) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Carga una base de datos desde archivo.
///
/// # Retorna
/// Puntero a la base de datos o NULL si error.
#[no_mangle]
pub extern "C" fn mmdb_load(path: *const c_char) -> *mut MiniMemoryDB {
    if path.is_null() {
        return ptr::null_mut();
    }

    let path_str = unsafe {
        match CStr::from_ptr(path).to_str() {
            Ok(s) => s,
            Err(_) => return ptr::null_mut(),
        }
    };

    match RustVectorDB::open(path_str) {
        Ok(db) => Box::into_raw(Box::new(MiniMemoryDB { inner: Arc::new(db) })),
        Err(_) => ptr::null_mut(),
    }
}

/// Obtiene el número de vectores.
#[no_mangle]
pub extern "C" fn mmdb_len(db: *mut MiniMemoryDB) -> u32 {
    if db.is_null() {
        return 0;
    }
    let db = unsafe { &*db };
    db.inner.len() as u32
}

/// Obtiene las dimensiones.
#[no_mangle]
pub extern "C" fn mmdb_dimensions(db: *mut MiniMemoryDB) -> u32 {
    if db.is_null() {
        return 0;
    }
    let db = unsafe { &*db };
    db.inner.dimensions() as u32
}

/// Limpia todos los vectores.
#[no_mangle]
pub extern "C" fn mmdb_clear(db: *mut MiniMemoryDB) {
    if !db.is_null() {
        let db = unsafe { &*db };
        db.inner.clear();
    }
}
