#!/usr/bin/env bash
set +e

export PATH="/cargo-tools/bin:$PATH"

./scripts/ensure-dev-tools.sh
tools_rc=$?

if [ "$tools_rc" -ne 0 ]; then
  echo
  echo "dev tool setup failed with exit code $tools_rc"
  echo "waiting for file changes..."
  exit 0
fi

set -euo pipefail

cargo fetch 2>/dev/null || true

exec cargo watch -w src -w Cargo.toml -w migrations -w ../lib -s 'bash scripts/dev-run.sh'
