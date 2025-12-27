# minimemory - PHP

Embedded vector database for PHP via FFI. Like SQLite, but for vector similarity search.

## Requirements

- PHP 7.4+ with FFI extension enabled
- The compiled `libminimemory` shared library

## Installation

### 1. Build the shared library

```bash
# From the project root
cargo build --release --features ffi

# The library will be at:
# Linux: target/release/libminimemory.so
# macOS: target/release/libminimemory.dylib
# Windows: target/release/minimemory.dll
```

### 2. Copy the library

```bash
# Linux
cp target/release/libminimemory.so /usr/local/lib/
ldconfig

# Or set LD_LIBRARY_PATH
export LD_LIBRARY_PATH=/path/to/target/release:$LD_LIBRARY_PATH
```

### 3. Enable FFI in PHP

Edit `php.ini`:
```ini
extension=ffi
ffi.enable=true
```

## Usage

### Direct FFI

```php
<?php
$ffi = FFI::cdef("
    typedef struct MiniMemoryDB MiniMemoryDB;
    typedef struct SearchResult {
        char* id;
        float distance;
    } SearchResult;

    MiniMemoryDB* mmdb_new(uint32_t dimensions, const char* distance, const char* index_type);
    void mmdb_free(MiniMemoryDB* db);
    int mmdb_insert(MiniMemoryDB* db, const char* id, const float* vector, uint32_t len);
    SearchResult* mmdb_search(MiniMemoryDB* db, const float* query, uint32_t len, uint32_t k, uint32_t* result_count);
    void mmdb_free_results(SearchResult* results, uint32_t count);
    float* mmdb_get(MiniMemoryDB* db, const char* id, uint32_t* len);
    void mmdb_free_vector(float* vector, uint32_t len);
    int mmdb_delete(MiniMemoryDB* db, const char* id);
    int mmdb_contains(MiniMemoryDB* db, const char* id);
    int mmdb_save(MiniMemoryDB* db, const char* path);
    MiniMemoryDB* mmdb_load(const char* path);
    uint32_t mmdb_len(MiniMemoryDB* db);
    uint32_t mmdb_dimensions(MiniMemoryDB* db);
    void mmdb_clear(MiniMemoryDB* db);
", "libminimemory.so");

// Create database
$db = $ffi->mmdb_new(384, "cosine", "hnsw");

// Insert vector
$vector = FFI::new("float[384]");
for ($i = 0; $i < 384; $i++) {
    $vector[$i] = 0.1;
}
$ffi->mmdb_insert($db, "doc-1", $vector, 384);

// Search
$count = FFI::new("uint32_t");
$results = $ffi->mmdb_search($db, $vector, 384, 10, FFI::addr($count));

for ($i = 0; $i < $count->cdata; $i++) {
    $id = FFI::string($results[$i]->id);
    $distance = $results[$i]->distance;
    echo "ID: $id, Distance: $distance\n";
}

// Cleanup
$ffi->mmdb_free_results($results, $count->cdata);
$ffi->mmdb_free($db);
?>
```

### Using the Wrapper Class

Copy `MiniMemory.php` to your project:

```php
<?php
require_once 'MiniMemory.php';

use MiniMemory\VectorDB;

// Create database
$db = new VectorDB(384, 'cosine', 'hnsw');

// Insert vectors
$embedding = array_fill(0, 384, 0.1);
$db->insert('doc-1', $embedding);

// Search
$results = $db->search($embedding, 10);
foreach ($results as $result) {
    echo "ID: {$result['id']}, Distance: {$result['distance']}\n";
}

// Save and load
$db->save('my_vectors.mmdb');
$db2 = VectorDB::load('my_vectors.mmdb');

// Other operations
echo "Count: " . $db->count() . "\n";
echo "Contains: " . ($db->contains('doc-1') ? 'yes' : 'no') . "\n";

$db->delete('doc-1');
$db->clear();
?>
```

## API Reference

### FFI Functions

| Function | Description |
|----------|-------------|
| `mmdb_new(dimensions, distance, index_type)` | Create new database |
| `mmdb_free(db)` | Free database memory |
| `mmdb_insert(db, id, vector, len)` | Insert vector (returns 0 on success) |
| `mmdb_search(db, query, len, k, count)` | Search k nearest |
| `mmdb_free_results(results, count)` | Free search results |
| `mmdb_get(db, id, len)` | Get vector by ID |
| `mmdb_free_vector(vector, len)` | Free vector memory |
| `mmdb_delete(db, id)` | Delete vector (1=deleted, 0=not found) |
| `mmdb_contains(db, id)` | Check if exists (1=yes, 0=no) |
| `mmdb_save(db, path)` | Save to file |
| `mmdb_load(path)` | Load from file |
| `mmdb_len(db)` | Get vector count |
| `mmdb_dimensions(db)` | Get dimensions |
| `mmdb_clear(db)` | Clear all vectors |

### Distance Types

- `"cosine"` or `"cos"` - Cosine distance
- `"euclidean"` or `"l2"` - Euclidean distance
- `"dot"` or `"dot_product"` - Dot product distance

### Index Types

- `"flat"` - Exact brute-force search
- `"hnsw"` - Approximate HNSW search

## Troubleshooting

### FFI not enabled
```
Fatal error: Uncaught FFI\Exception: FFI API is restricted by "ffi.enable" configuration directive
```
Set `ffi.enable=true` in php.ini.

### Library not found
```
Fatal error: Uncaught FFI\Exception: Failed loading 'libminimemory.so'
```
Check library path and LD_LIBRARY_PATH.

### Preloading for production

For production, use FFI preloading in `php.ini`:
```ini
ffi.enable=preload
opcache.preload=/path/to/preload.php
```

```php
<?php
// preload.php
FFI::load('/path/to/minimemory.h');
?>
```
