<?php
/**
 * MiniMemory - Embedded Vector Database for PHP
 *
 * Like SQLite, but for vector similarity search.
 *
 * @package MiniMemory
 * @version 0.1.0
 */

namespace MiniMemory;

use FFI;
use FFI\CData;
use RuntimeException;

/**
 * Vector Database class
 *
 * @example
 * $db = new VectorDB(384, 'cosine', 'hnsw');
 * $db->insert('doc-1', array_fill(0, 384, 0.1));
 * $results = $db->search(array_fill(0, 384, 0.1), 10);
 */
class VectorDB
{
    private FFI $ffi;
    private CData $db;
    private int $dimensions;
    private bool $freed = false;

    private const FFI_DEF = "
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
    ";

    /**
     * Create a new vector database
     *
     * @param int $dimensions Number of dimensions
     * @param string $distance Distance metric: "cosine", "euclidean", or "dot"
     * @param string $indexType Index type: "flat" or "hnsw"
     * @param string|null $libraryPath Path to libminimemory shared library
     * @throws RuntimeException If library cannot be loaded or database creation fails
     */
    public function __construct(
        int $dimensions,
        string $distance = 'cosine',
        string $indexType = 'flat',
        ?string $libraryPath = null
    ) {
        $this->dimensions = $dimensions;
        $this->ffi = $this->loadLibrary($libraryPath);

        $this->db = $this->ffi->mmdb_new($dimensions, $distance, $indexType);
        if (FFI::isNull($this->db)) {
            throw new RuntimeException("Failed to create vector database");
        }
    }

    /**
     * Load database from file
     *
     * @param string $path Path to .mmdb file
     * @param string|null $libraryPath Path to libminimemory shared library
     * @return self
     * @throws RuntimeException If file cannot be loaded
     */
    public static function load(string $path, ?string $libraryPath = null): self
    {
        $instance = new self(1, 'cosine', 'flat', $libraryPath);
        $instance->ffi->mmdb_free($instance->db);

        $instance->db = $instance->ffi->mmdb_load($path);
        if (FFI::isNull($instance->db)) {
            throw new RuntimeException("Failed to load database from: $path");
        }

        $instance->dimensions = $instance->ffi->mmdb_dimensions($instance->db);
        return $instance;
    }

    /**
     * Insert a vector
     *
     * @param string $id Unique identifier
     * @param array $vector Array of floats
     * @throws RuntimeException If insertion fails
     */
    public function insert(string $id, array $vector): void
    {
        $this->checkNotFreed();

        if (count($vector) !== $this->dimensions) {
            throw new RuntimeException(
                "Vector dimension mismatch: expected {$this->dimensions}, got " . count($vector)
            );
        }

        $cVector = $this->arrayToFloatPtr($vector);
        $result = $this->ffi->mmdb_insert($this->db, $id, $cVector, count($vector));

        if ($result !== 0) {
            throw new RuntimeException("Failed to insert vector with id: $id");
        }
    }

    /**
     * Search for k nearest neighbors
     *
     * @param array $query Query vector
     * @param int $k Number of results
     * @return array Array of ['id' => string, 'distance' => float]
     */
    public function search(array $query, int $k): array
    {
        $this->checkNotFreed();

        if (count($query) !== $this->dimensions) {
            throw new RuntimeException(
                "Query dimension mismatch: expected {$this->dimensions}, got " . count($query)
            );
        }

        $cQuery = $this->arrayToFloatPtr($query);
        $count = FFI::new("uint32_t");

        $results = $this->ffi->mmdb_search($this->db, $cQuery, count($query), $k, FFI::addr($count));

        $output = [];
        $resultCount = $count->cdata;

        if ($resultCount > 0 && !FFI::isNull($results)) {
            for ($i = 0; $i < $resultCount; $i++) {
                $output[] = [
                    'id' => FFI::string($results[$i]->id),
                    'distance' => $results[$i]->distance,
                ];
            }
            $this->ffi->mmdb_free_results($results, $resultCount);
        }

        return $output;
    }

    /**
     * Get a vector by ID
     *
     * @param string $id Vector ID
     * @return array|null Vector array or null if not found
     */
    public function get(string $id): ?array
    {
        $this->checkNotFreed();

        $len = FFI::new("uint32_t");
        $vector = $this->ffi->mmdb_get($this->db, $id, FFI::addr($len));

        if (FFI::isNull($vector)) {
            return null;
        }

        $result = [];
        $length = $len->cdata;
        for ($i = 0; $i < $length; $i++) {
            $result[] = $vector[$i];
        }

        $this->ffi->mmdb_free_vector($vector, $length);
        return $result;
    }

    /**
     * Delete a vector by ID
     *
     * @param string $id Vector ID
     * @return bool True if deleted, false if not found
     */
    public function delete(string $id): bool
    {
        $this->checkNotFreed();
        return $this->ffi->mmdb_delete($this->db, $id) === 1;
    }

    /**
     * Check if a vector exists
     *
     * @param string $id Vector ID
     * @return bool
     */
    public function contains(string $id): bool
    {
        $this->checkNotFreed();
        return $this->ffi->mmdb_contains($this->db, $id) === 1;
    }

    /**
     * Save database to file
     *
     * @param string $path File path
     * @throws RuntimeException If save fails
     */
    public function save(string $path): void
    {
        $this->checkNotFreed();

        $result = $this->ffi->mmdb_save($this->db, $path);
        if ($result !== 0) {
            throw new RuntimeException("Failed to save database to: $path");
        }
    }

    /**
     * Clear all vectors
     */
    public function clear(): void
    {
        $this->checkNotFreed();
        $this->ffi->mmdb_clear($this->db);
    }

    /**
     * Get number of vectors
     *
     * @return int
     */
    public function count(): int
    {
        $this->checkNotFreed();
        return $this->ffi->mmdb_len($this->db);
    }

    /**
     * Get dimensions
     *
     * @return int
     */
    public function dimensions(): int
    {
        return $this->dimensions;
    }

    /**
     * Free resources
     */
    public function __destruct()
    {
        if (!$this->freed && isset($this->ffi) && isset($this->db)) {
            $this->ffi->mmdb_free($this->db);
            $this->freed = true;
        }
    }

    private function loadLibrary(?string $path): FFI
    {
        $paths = $path ? [$path] : $this->getDefaultLibraryPaths();

        foreach ($paths as $libPath) {
            if (file_exists($libPath)) {
                return FFI::cdef(self::FFI_DEF, $libPath);
            }
        }

        // Try system library path
        try {
            return FFI::cdef(self::FFI_DEF, $this->getLibraryName());
        } catch (\FFI\Exception $e) {
            throw new RuntimeException(
                "Could not load minimemory library. Tried: " . implode(', ', $paths) .
                ". Build with: cargo build --release --features ffi"
            );
        }
    }

    private function getDefaultLibraryPaths(): array
    {
        $libName = $this->getLibraryName();
        return [
            __DIR__ . "/../../target/release/$libName",
            __DIR__ . "/$libName",
            "/usr/local/lib/$libName",
            "/usr/lib/$libName",
        ];
    }

    private function getLibraryName(): string
    {
        return match (PHP_OS_FAMILY) {
            'Windows' => 'minimemory.dll',
            'Darwin' => 'libminimemory.dylib',
            default => 'libminimemory.so',
        };
    }

    private function arrayToFloatPtr(array $array): CData
    {
        $count = count($array);
        $ptr = FFI::new("float[$count]");
        for ($i = 0; $i < $count; $i++) {
            $ptr[$i] = (float)$array[$i];
        }
        return $ptr;
    }

    private function checkNotFreed(): void
    {
        if ($this->freed) {
            throw new RuntimeException("VectorDB has been freed");
        }
    }
}
