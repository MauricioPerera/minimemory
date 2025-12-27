# minimemory - Python

Embedded vector database for Python. Like SQLite, but for vector similarity search.

## Installation

### From PyPI (when published)

```bash
pip install minimemory
```

### From source

```bash
# Requires Rust and maturin
pip install maturin
cd bindings/python
maturin develop --features python
```

## Usage

```python
from minimemory import VectorDB

# Create database with 384 dimensions (e.g., for sentence-transformers)
db = VectorDB(dimensions=384, distance="cosine", index_type="hnsw")

# Insert vectors
embedding = [0.1] * 384  # Your embedding here
db.insert("doc-1", embedding, {"title": "My Document", "score": 42})

# Search for similar vectors
query = [0.15] * 384
results = db.search(query, k=10)

for result in results:
    print(f"ID: {result.id}, Distance: {result.distance}")

# Save to disk
db.save("my_vectors.mmdb")

# Load from disk
db2 = VectorDB.load("my_vectors.mmdb")

# Other operations
print(f"Total vectors: {len(db)}")
print(f"Contains doc-1: {db.contains('doc-1')}")

vector, metadata = db.get("doc-1")
db.delete("doc-1")
db.clear()
```

## API Reference

### VectorDB

```python
VectorDB(
    dimensions: int,
    distance: str = "cosine",  # "cosine", "euclidean", "dot"
    index_type: str = "flat"   # "flat", "hnsw"
)
```

**Methods:**

- `insert(id: str, vector: List[float], metadata: dict = None)` - Insert a vector
- `search(query: List[float], k: int) -> List[SearchResult]` - Find k nearest neighbors
- `get(id: str) -> Tuple[List[float], dict]` - Get vector by ID
- `delete(id: str) -> bool` - Delete vector by ID
- `contains(id: str) -> bool` - Check if ID exists
- `update(id: str, vector: List[float], metadata: dict = None)` - Update existing vector
- `save(path: str)` - Save database to file
- `load(path: str) -> VectorDB` - Load database from file (class method)
- `clear()` - Remove all vectors
- `__len__()` - Get number of vectors

### SearchResult

```python
SearchResult.id: str        # Vector ID
SearchResult.distance: float  # Distance to query
```

## Distance Metrics

- **cosine**: Cosine similarity (1 - cos_sim). Best for text embeddings.
- **euclidean**: L2 distance. Best for normalized vectors.
- **dot**: Negative dot product. Best when magnitude matters.

## Index Types

- **flat**: Exact brute-force search. O(n) but 100% accurate.
- **hnsw**: Approximate search using HNSW algorithm. O(log n), very fast for large datasets.

## Performance Tips

1. Use `hnsw` index for datasets > 10,000 vectors
2. Use `cosine` distance for text embeddings
3. Batch inserts when possible
4. Consider using normalized vectors for faster computation
