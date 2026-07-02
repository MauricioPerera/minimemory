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
use std::panic::AssertUnwindSafe;
use std::ptr;
use std::sync::Arc;

use crate::{
    Config as RustConfig, Distance as RustDistance, IndexType as RustIndexType,
    VectorDB as RustVectorDB,
};

/// Ejecuta `f` capturando cualquier pánico de Rust para que no cruce la frontera
/// FFI (comportamiento indefinido). Si `f` panica, devuelve `default`.
///
/// `AssertUnwindSafe` es sound aquí: tras un pánico no se reanuda la ejecución
/// dentro de `f` ni se observa estado parcial de sus capturas — se descartan y se
/// devuelve `default`. Las capturas son punteros/referencias locales a la llamada
/// FFI; el `Arc<VectorDB>` compartido puede quedar con cerrojos envenenados por un
/// pánico a mitad de una mutación, pero eso es un estado lógico (las operaciones
/// posteriores retornan error), no inseguridad de memoria.
fn catch_panic<F, R>(default: R, f: F) -> R
where
    F: FnOnce() -> R,
{
    match std::panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(r) => r,
        Err(_) => default,
    }
}

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
///
/// # Safety
///
/// `distance` e `index_type` deben ser punteros válidos a strings C nul-terminados
/// codificados en UTF-8, o NULL (se usan los valores por defecto "cosine"/"flat").
/// El puntero retornado (no-NULL) debe liberarse con `mmdb_free`; no debe
/// liberarse con `free` de C ni combinarse con `mmdb_free_vector`/`mmdb_free_results`.
#[no_mangle]
pub extern "C" fn mmdb_new(
    dimensions: u32,
    distance: *const c_char,
    index_type: *const c_char,
) -> *mut MiniMemoryDB {
    catch_panic(ptr::null_mut(), || {
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
            "hnsw" => RustIndexType::HNSW {
                m: 16,
                ef_construction: 200,
            },
            _ => return ptr::null_mut(),
        };

        let config = RustConfig::new(dimensions as usize)
            .with_distance(dist)
            .with_index(idx);

        match RustVectorDB::new(config) {
            Ok(db) => Box::into_raw(Box::new(MiniMemoryDB {
                inner: Arc::new(db),
            })),
            Err(_) => ptr::null_mut(),
        }
    })
}

/// Libera la memoria de la base de datos.
///
/// # Safety
///
/// `db` debe ser un puntero retornado por `mmdb_new` o `mmdb_load`, o NULL (no-op).
/// Tras la llamada, `db` no debe usarse de nuevo (use-after-free). Llamar dos veces
/// con el mismo puntero no-NULL es uso indefinido.
#[no_mangle]
pub extern "C" fn mmdb_free(db: *mut MiniMemoryDB) {
    catch_panic((), || {
        if !db.is_null() {
            unsafe {
                drop(Box::from_raw(db));
            }
        }
    })
}

/// Inserta un vector en la base de datos.
///
/// # Retorna
/// 0 si éxito, -1 si error.
///
/// # Safety
///
/// `db` debe ser un puntero válido retornado por `mmdb_new`/`mmdb_load` (no NULL).
/// `id` debe apuntar a un string C nul-terminado codificado en UTF-8 (no NULL).
/// `vector` debe apuntar a un buffer de `len` elementos `f32` válidos (no NULL);
/// `len` debe ser exactamente la longitud del buffer y coincidir con las
/// dimensiones de la DB. Ninguno de estos punteros se retiene tras la llamada.
#[no_mangle]
pub extern "C" fn mmdb_insert(
    db: *mut MiniMemoryDB,
    id: *const c_char,
    vector: *const c_float,
    len: u32,
) -> c_int {
    catch_panic(-1, || {
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

        let vec: Vec<f32> = unsafe { std::slice::from_raw_parts(vector, len as usize).to_vec() };

        match db.inner.insert(id_str, &vec, None) {
            Ok(_) => 0,
            Err(_) => -1,
        }
    })
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
///
/// # Safety
///
/// `db`, `query` y `result_count` deben ser no-NULL. `db` debe ser válido (de
/// `mmdb_new`/`mmdb_load`). `query` apunta a `len` elementos `f32` y `len` debe
/// coincidir con las dimensiones de la DB. `result_count` apunta a una `u32`
/// escribible donde se escribe el número de resultados. El puntero retornado
/// (no-NULL) es un array de `SearchResult` de longitud `*result_count` y debe
/// liberarse con `mmdb_free_results(results, *result_count)` usando exactamente
/// ese count. Con NULL (sin resultados o error), `*result_count` es 0.
#[no_mangle]
pub extern "C" fn mmdb_search(
    db: *mut MiniMemoryDB,
    query: *const c_float,
    len: u32,
    k: u32,
    result_count: *mut u32,
) -> *mut SearchResult {
    catch_panic(ptr::null_mut(), || {
        if db.is_null() || query.is_null() || result_count.is_null() {
            return ptr::null_mut();
        }

        let db = unsafe { &*db };
        let query_vec: Vec<f32> =
            unsafe { std::slice::from_raw_parts(query, len as usize).to_vec() };

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
    })
}

/// Libera los resultados de búsqueda.
///
/// # Safety
///
/// `results` debe ser NULL o un puntero retornado por `mmdb_search`. `count` debe
/// ser exactamente el valor escrito en `*result_count` por la llamada a
/// `mmdb_search` que produjo `results`; un count erróneo provoca UB (dealloc con
/// layout incorrecto). Tras la llamada, `results` no debe usarse de nuevo.
#[no_mangle]
pub extern "C" fn mmdb_free_results(results: *mut SearchResult, count: u32) {
    catch_panic((), || {
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
    })
}

/// Obtiene un vector por su ID.
///
/// # Retorna
/// Puntero al vector (debe liberarse con mmdb_free_vector) o NULL si no existe.
///
/// # Safety
///
/// `db`, `id` y `len` deben ser no-NULL. `db` debe ser válido (de
/// `mmdb_new`/`mmdb_load`). `id` es un string C nul-terminado UTF-8. `len` apunta
/// a una `u32` escribible. El puntero retornado (no-NULL) es un buffer de `*len`
/// elementos `f32` y debe liberarse con `mmdb_free_vector(ptr, *len)`. Si el
/// documento no existe o es metadata-only (sin vector), retorna NULL y escribe 0
/// en `*len`.
#[no_mangle]
pub extern "C" fn mmdb_get(
    db: *mut MiniMemoryDB,
    id: *const c_char,
    len: *mut u32,
) -> *mut c_float {
    catch_panic(ptr::null_mut(), || {
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
            Ok(Some((Some(vector), _))) => {
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
    })
}

/// Libera un vector obtenido con mmdb_get.
///
/// # Safety
///
/// `vector` debe ser NULL o un puntero retornado por `mmdb_get`. `len` debe
/// coincidir con el `*len` escrito por `mmdb_get` al obtener el vector; un len
/// erróneo provoca UB (dealloc con layout incorrecto). Tras la llamada, `vector`
/// no debe usarse de nuevo.
#[no_mangle]
pub extern "C" fn mmdb_free_vector(vector: *mut c_float, len: u32) {
    catch_panic((), || {
        if !vector.is_null() && len > 0 {
            unsafe {
                drop(Vec::from_raw_parts(vector, len as usize, len as usize));
            }
        }
    })
}

/// Elimina un vector por su ID.
///
/// # Retorna
/// 1 si fue eliminado, 0 si no existía, -1 si error.
///
/// # Safety
///
/// `db` e `id` deben ser no-NULL. `db` debe ser válido (de `mmdb_new`/`mmdb_load`).
/// `id` es un string C nul-terminado UTF-8.
#[no_mangle]
pub extern "C" fn mmdb_delete(db: *mut MiniMemoryDB, id: *const c_char) -> c_int {
    catch_panic(-1, || {
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
    })
}

/// Verifica si un vector existe.
///
/// # Retorna
/// 1 si existe, 0 si no.
///
/// # Safety
///
/// `db` e `id` deben ser no-NULL. `db` debe ser válido (de `mmdb_new`/`mmdb_load`).
/// `id` es un string C nul-terminado UTF-8.
#[no_mangle]
pub extern "C" fn mmdb_contains(db: *mut MiniMemoryDB, id: *const c_char) -> c_int {
    catch_panic(0, || {
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

        if db.inner.contains(id_str) {
            1
        } else {
            0
        }
    })
}

/// Guarda la base de datos a un archivo.
///
/// # Retorna
/// 0 si éxito, -1 si error.
///
/// # Safety
///
/// `db` y `path` deben ser no-NULL. `db` debe ser válido (de `mmdb_new`/`mmdb_load`).
/// `path` es un string C nul-terminado UTF-8 con una ruta escribible.
#[no_mangle]
pub extern "C" fn mmdb_save(db: *mut MiniMemoryDB, path: *const c_char) -> c_int {
    catch_panic(-1, || {
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
    })
}

/// Carga una base de datos desde archivo.
///
/// # Retorna
/// Puntero a la base de datos o NULL si error.
///
/// # Safety
///
/// `path` debe apuntar a un string C nul-terminado UTF-8 (no NULL) con una ruta
/// legible a un snapshot válido de minimemory. El puntero retornado (no-NULL)
/// debe liberarse con `mmdb_free`.
#[no_mangle]
pub extern "C" fn mmdb_load(path: *const c_char) -> *mut MiniMemoryDB {
    catch_panic(ptr::null_mut(), || {
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
            Ok(db) => Box::into_raw(Box::new(MiniMemoryDB {
                inner: Arc::new(db),
            })),
            Err(_) => ptr::null_mut(),
        }
    })
}

/// Obtiene el número de vectores.
///
/// # Safety
///
/// `db` debe ser NULL o un puntero válido de `mmdb_new`/`mmdb_load`. Con NULL
/// retorna 0.
#[no_mangle]
pub extern "C" fn mmdb_len(db: *mut MiniMemoryDB) -> u32 {
    catch_panic(0, || {
        if db.is_null() {
            return 0;
        }
        let db = unsafe { &*db };
        db.inner.len() as u32
    })
}

/// Obtiene las dimensiones.
///
/// # Safety
///
/// `db` debe ser NULL o un puntero válido de `mmdb_new`/`mmdb_load`. Con NULL
/// retorna 0.
#[no_mangle]
pub extern "C" fn mmdb_dimensions(db: *mut MiniMemoryDB) -> u32 {
    catch_panic(0, || {
        if db.is_null() {
            return 0;
        }
        let db = unsafe { &*db };
        db.inner.dimensions() as u32
    })
}

/// Limpia todos los vectores.
///
/// # Safety
///
/// `db` debe ser NULL o un puntero válido de `mmdb_new`/`mmdb_load`. Con NULL es
/// no-op.
#[no_mangle]
pub extern "C" fn mmdb_clear(db: *mut MiniMemoryDB) {
    catch_panic((), || {
        if !db.is_null() {
            let db = unsafe { &*db };
            db.inner.clear();
        }
    })
}