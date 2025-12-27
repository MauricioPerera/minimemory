# Minimemory Demo Video Script

## Dataset Info
- 10 documents with 128-dimension vectors
- Categories: AI (4), Programming (3), Databases (3)
- Authors: John Smith, Jane Doe, Bob Wilson, Alice Brown, Charlie Davis

## Demo Flow

### 1. Create Database (0:00 - 0:30)
**Operation:** Create Database
- Database Name: `knowledge-base`
- Dimensions: `128`
- Distance Metric: `Cosine`
- Index Type: `Flat`

### 2. Insert Many (0:30 - 1:30)
**Setup:** Read the sample-vectors.json file first
**Operation:** Insert Many
- Database Name: `knowledge-base`
- ID Field: `id`
- Vector Field: `embedding`
- Metadata Fields: `content,title,category,author`

### 3. Vector Search (1:30 - 2:30)
**Operation:** Search
- Database Name: `knowledge-base`
- Search Mode: `Vector Only`
- Query Vector: (use query-vector.json embedding)
- Number of Results: `5`

**Expected:** Returns AI-related documents (doc-001, doc-002, doc-006, doc-007) at top

### 4. Keyword Search - BM25 (2:30 - 3:30)
**Operation:** Search
- Database Name: `knowledge-base`
- Search Mode: `Keyword Only (BM25)`
- Keywords: `machine learning artificial intelligence`
- Text Fields: `content,title`
- Number of Results: `5`

**Expected:** Returns docs with those keywords

### 5. Hybrid Search (3:30 - 4:30)
**Operation:** Search
- Database Name: `knowledge-base`
- Search Mode: `Hybrid`
- Query Vector: (same as before)
- Keywords: `neural networks deep learning`
- Hybrid Alpha: `0.5`
- Fusion Method: `RRF`

**Expected:** Combines vector similarity + keyword matching

### 6. Filter by Metadata (4:30 - 5:30)
**Operation:** Search
- Use Metadata Filter: `true`
- Metadata Filter: `{"category": "Programming"}`

**Expected:** Only returns Programming docs (doc-003, doc-004, doc-008)

### 7. Export Database (5:30 - 6:00)
**Operation:** Export Database
- Database Name: `knowledge-base`

**Show:** JSON output with all data

### 8. Persist to Workflow (6:00 - 6:30)
**Operation:** Persist to Workflow
- Database Name: `knowledge-base`

**Explain:** Data survives n8n restarts

### 9. Get Info (6:30 - 7:00)
**Operation:** Get Info
- Database Name: `knowledge-base`

**Show:** Stats (vector count, dimensions, etc.)

## Key Points to Mention
- 100% serverless - no external database needed
- Works offline
- Fast similarity search
- Hybrid search combines semantic + keyword
- Persist to workflow for data that survives restarts
- Export/Import for backup and sharing
