/**
 * minimemory-service API Test
 * Run: node demo/test-api.mjs
 * (Server must be running: npm run dev)
 */

const API = 'http://localhost:3000/api/v1';
const API_KEY = 'mm_dev_key_12345';
const NAMESPACE = 'demo';

async function request(method, path, body = null) {
  const options = {
    method,
    headers: {
      'X-API-Key': API_KEY,
      'X-Namespace': NAMESPACE,
      'Content-Type': 'application/json',
    },
  };
  if (body) {
    options.body = JSON.stringify(body);
  }
  const res = await fetch(`${API}${path}`, options);
  return res.json();
}

async function createNamespace() {
  const res = await fetch(`${API}/namespaces`, {
    method: 'POST',
    headers: {
      'X-API-Key': API_KEY,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ name: NAMESPACE, dimensions: 5 }),
  });
  return res.json();
}

async function main() {
  console.log('======================================');
  console.log('  minimemory-service API Test');
  console.log('======================================\n');

  // 0. Create namespace (5 dimensions for demo)
  console.log('0. Creating Namespace (5 dimensions)');
  const nsResult = await createNamespace();
  if (nsResult.success) {
    console.log(`  ✓ Created namespace: ${nsResult.namespace.name} (${nsResult.namespace.dimensions}D)`);
  } else {
    console.log(`  ℹ Namespace exists or error: ${nsResult.error || 'already exists'}`);
  }
  console.log();

  // 1. Health check
  console.log('1. Health Check');
  const health = await fetch('http://localhost:3000/health').then(r => r.json());
  console.log(health);
  console.log();

  // 2. Initial stats
  console.log('2. Initial Stats');
  let stats = await request('GET', '/stats');
  console.log(stats);
  console.log();

  // 3. Remember memories
  console.log('3. Storing Memories...');

  const memories = [
    {
      content: 'User asked about machine learning features',
      embedding: [0.9, 0.1, 0.2, 0.1, 0.0],
      type: 'episodic',
      importance: 0.8,
      metadata: { topic: 'ml', userId: 'user-1' }
    },
    {
      content: 'User works at AI Research Lab',
      embedding: [0.85, 0.15, 0.25, 0.1, 0.0],
      type: 'semantic',
      importance: 0.9,
      metadata: { category: 'user_info', userId: 'user-1' }
    },
    {
      content: 'User prefers Python over JavaScript',
      embedding: [0.8, 0.2, 0.3, 0.15, 0.0],
      type: 'semantic',
      importance: 0.7,
      metadata: { category: 'preferences', userId: 'user-1' }
    },
    {
      content: 'User requested neural network documentation',
      embedding: [0.1, 0.9, 0.1, 0.8, 0.7],
      type: 'episodic',
      importance: 0.6,
      metadata: { topic: 'docs', userId: 'user-1' }
    },
    {
      content: 'Deep learning and neural networks are key interests',
      embedding: [0.15, 0.85, 0.15, 0.75, 0.65],
      type: 'semantic',
      importance: 0.85,
      metadata: { category: 'interests', userId: 'user-1' }
    }
  ];

  const storedMemories = [];
  for (const mem of memories) {
    const result = await request('POST', '/remember', mem);
    storedMemories.push(result.memory);
    console.log(`  ✓ Stored: ${mem.content.substring(0, 40)}...`);
  }
  console.log();

  // 4. Stats after storing
  console.log('4. Stats After Storing');
  stats = await request('GET', '/stats');
  console.log(`  Total: ${stats.stats.total}`);
  console.log(`  By type: episodic=${stats.stats.byType.episodic}, semantic=${stats.stats.byType.semantic}`);
  console.log();

  // 5. Vector search
  console.log('5. Vector Search (ML-related query)');
  const vectorResults = await request('POST', '/recall', {
    embedding: [0.88, 0.12, 0.22, 0.12, 0.0],
    mode: 'vector',
    limit: 3
  });
  console.log(`  Found ${vectorResults.count} memories:`);
  for (const r of vectorResults.results) {
    console.log(`    - [${r.type}] ${r.content.substring(0, 50)}... (score: ${r.score.toFixed(3)})`);
  }
  console.log();

  // 6. Keyword search
  console.log('6. Keyword Search (neural networks)');
  const keywordResults = await request('POST', '/recall', {
    keywords: 'neural networks deep learning',
    mode: 'keyword',
    limit: 3
  });
  console.log(`  Found ${keywordResults.count} memories:`);
  for (const r of keywordResults.results) {
    console.log(`    - [${r.type}] ${r.content.substring(0, 50)}... (score: ${r.score.toFixed(3)})`);
  }
  console.log();

  // 7. Hybrid search
  console.log('7. Hybrid Search (vector + keywords)');
  const hybridResults = await request('POST', '/recall', {
    embedding: [0.5, 0.5, 0.2, 0.4, 0.3],
    keywords: 'machine learning user',
    mode: 'hybrid',
    alpha: 0.6,
    limit: 5
  });
  console.log(`  Found ${hybridResults.count} memories:`);
  for (const r of hybridResults.results) {
    console.log(`    - [${r.type}] ${r.content.substring(0, 45)}...`);
    console.log(`      score: ${r.score.toFixed(4)}, vector: ${r.vectorSimilarity?.toFixed(3) || 'N/A'}, keyword: ${r.keywordScore?.toFixed(3) || 'N/A'}`);
  }
  console.log();

  // 8. Filter by type
  console.log('8. Search with Type Filter (semantic only)');
  const filteredResults = await request('POST', '/recall', {
    keywords: 'user',
    mode: 'keyword',
    type: 'semantic',
    limit: 10
  });
  console.log(`  Found ${filteredResults.count} semantic memories:`);
  for (const r of filteredResults.results) {
    console.log(`    - ${r.content.substring(0, 50)}...`);
  }
  console.log();

  // 9. Get specific memory
  console.log('9. Get Memory by ID');
  const memory = await request('GET', `/memory/${storedMemories[0].id}`);
  console.log(`  ID: ${memory.memory.id}`);
  console.log(`  Content: ${memory.memory.content}`);
  console.log(`  Type: ${memory.memory.type}`);
  console.log();

  // 10. Update memory
  console.log('10. Update Memory');
  const updated = await request('PATCH', `/memory/${storedMemories[0].id}`, {
    importance: 0.95,
    metadata: { topic: 'ml', userId: 'user-1', updated: true }
  });
  console.log(`  Updated importance: ${updated.memory.importance}`);
  console.log();

  // 11. Export
  console.log('11. Export All Memories');
  const exported = await request('POST', '/export');
  console.log(`  Exported ${exported.data.memories.length} memories`);
  console.log();

  // 12. Clear
  console.log('12. Clear All Memories');
  const cleared = await request('DELETE', '/clear');
  console.log(`  ${cleared.message}`);
  console.log();

  // 13. Final stats
  console.log('13. Final Stats (after clear)');
  stats = await request('GET', '/stats');
  console.log(`  Total: ${stats.stats.total}`);
  console.log();

  console.log('======================================');
  console.log('  Test Complete!');
  console.log('======================================');
}

main().catch(console.error);
