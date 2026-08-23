#!/usr/bin/env bash
set -euo pipefail

export CARGO_INSTALL_ROOT=/cargo-tools
export PATH="/cargo-tools/bin:$PATH"

mkdir -p /cargo-tools/bin

# Resolve the sqlx version from the mounted Cargo.lock.
# This keeps the CLI aligned with the app dependency graph.
SQLX_VERSION="$(
  awk '
    $0 == "[[package]]" { in_pkg=1; name=""; version="" }
    in_pkg && $1 == "name" && $3 == "\"sqlx\"" { name="sqlx" }
    in_pkg && $1 == "version" { gsub(/"/, "", $3); version=$3 }
    in_pkg && name == "sqlx" && version != "" { print version; exit }
  ' Cargo.lock
)"

if [ -z "$SQLX_VERSION" ]; then
  echo "could not resolve sqlx version from Cargo.lock"
  exit 1
fi

if command -v sqlx >/dev/null 2>&1; then
  INSTALLED_VERSION="$(sqlx --version | awk '{print $2}')"

  if [ "$INSTALLED_VERSION" = "$SQLX_VERSION" ]; then
    echo "sqlx-cli $INSTALLED_VERSION already installed"
    exit 0
  fi

  echo "sqlx-cli version mismatch: installed=$INSTALLED_VERSION required=$SQLX_VERSION"
fi

echo "installing sqlx-cli $SQLX_VERSION from mounted project lock context"

cargo install sqlx-cli \
  --version "$SQLX_VERSION" \
  --no-default-features \
  --features native-tls,postgres \
  --root /cargo-tools
