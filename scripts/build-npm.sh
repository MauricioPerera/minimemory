#!/usr/bin/env bash
# Builds the npm package into pkg/ ready to publish as @rckflr/minimemory.
# wasm-pack regenerates pkg/package.json from Cargo.toml on every build
# (name "minimemory", no scope), so the scope rename and the npm README
# must be re-applied after each build — that is what this script does.
#
# Usage:   bash scripts/build-npm.sh
# Publish: cd pkg && npm publish --access public [--otp=XXXXXX]
set -euo pipefail
cd "$(dirname "$0")/.."

wasm-pack build --target web --release -- --features wasm

node -e "
const fs = require('fs');
const p = JSON.parse(fs.readFileSync('pkg/package.json', 'utf8'));
p.name = '@rckflr/minimemory';
p.repository.url = 'git+https://github.com/MauricioPerera/minimemory.git';
fs.writeFileSync('pkg/package.json', JSON.stringify(p, null, 2) + '\n');
"

cp npm-readme.md pkg/README.md

echo ''
echo 'pkg/ ready:'
node -e "const p=require('./pkg/package.json'); console.log('  ' + p.name + '@' + p.version)"
ls -la pkg/minimemory_bg.wasm | awk '{print "  wasm: " $5 " bytes"}'
echo '  publish: cd pkg && npm publish --access public'
