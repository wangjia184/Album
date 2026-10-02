#!/usr/bin/env bash
set -euo pipefail
set -m # job control: each background job gets its own process group

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [[ ! -d "$ROOT/ui/dist" ]]; then
  echo "warning: ui/dist missing — run (cd ui && npm run build) first if embed/tests need it" >&2
fi

pids=()
cleanup() {
  local code=$?
  trap - INT TERM EXIT
  local p
  for p in "${pids[@]:-}"; do
    [[ -n "$p" ]] || continue
    # Kill the job's whole process group (cargo→album, npm→vite), not just the subshell.
    kill -- "-$p" 2>/dev/null || kill "$p" 2>/dev/null || true
  done
  wait 2>/dev/null || true
  exit "$code"
}
trap cleanup INT TERM EXIT

echo "==> album: cargo run (http://127.0.0.1:3000)"
(cd "$ROOT/album" && cargo run) &
pids+=($!)

echo "==> ui: npm run dev (http://127.0.0.1:5173, /api -> 3000)"
(cd "$ROOT/ui" && npm run dev) &
pids+=($!)

# Exit (and tear down both) as soon as either process ends.
wait -n || true