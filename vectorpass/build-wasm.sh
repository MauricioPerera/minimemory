#!/bin/bash
# Build minimemory WASM for Cloudflare Workers

set -e

echo "Building minimemory WASM..."

# Navigate to root of minimemory
cd "$(dirname "$0")/.."

# Build with wasm-pack for web target (compatible with Workers)
wasm-pack build --target web --features wasm --out-dir vectorpass/pkg

echo "WASM build complete! Output in vectorpass/pkg/"
echo ""
echo "Next steps:"
echo "  cd vectorpass"
echo "  npm install"
echo "  npm run dev"
