#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "==> Building ui (npm run build)"
(cd "$ROOT/ui" && npm run build)

echo "==> Building album (cargo build --release)"
(cd "$ROOT/album" && cargo build --release)

echo "==> Done: $ROOT/album/target/release/album"