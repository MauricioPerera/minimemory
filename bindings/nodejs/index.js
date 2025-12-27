/**
 * minimemory - Embedded vector database for Node.js
 * Like SQLite, but for vector similarity search.
 *
 * @example
 * const { VectorDB } = require('minimemory');
 *
 * const db = new VectorDB({ dimensions: 384, distance: 'cosine' });
 * db.insert('doc1', new Array(384).fill(0.1));
 * const results = db.search(new Array(384).fill(0.1), 10);
 */

const { existsSync } = require('fs');
const { join } = require('path');

// Try to load the native addon
let nativeBinding = null;
const loadErrors = [];

// List of possible binary locations
const possiblePaths = [
  // Local development build
  join(__dirname, '../../target/release/minimemory.node'),
  join(__dirname, '../../target/debug/minimemory.node'),
  // npm package structure
  join(__dirname, 'minimemory.node'),
  join(__dirname, `minimemory.${process.platform}-${process.arch}.node`),
];

for (const bindingPath of possiblePaths) {
  try {
    if (existsSync(bindingPath)) {
      nativeBinding = require(bindingPath);
      break;
    }
  } catch (err) {
    loadErrors.push({ path: bindingPath, error: err.message });
  }
}

if (!nativeBinding) {
  const errorMessage = `Failed to load minimemory native module.
Tried paths:
${possiblePaths.map(p => `  - ${p}`).join('\n')}

Build the native module with:
  cd bindings/nodejs && npm run build

Errors:
${loadErrors.map(e => `  ${e.path}: ${e.error}`).join('\n')}
`;
  throw new Error(errorMessage);
}

module.exports = nativeBinding;
