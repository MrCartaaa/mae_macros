#!/usr/bin/env bash
set -euo pipefail

# Centralized SQLx migration entrypoint.
# Always run migrations via MIGRATE_DATABASE_URL so we use the dedicated
# MIGRATOR user instead of sqlx env autodetection.
echo "[boot] Running SQLx migrations..."
sqlx migrate run --database-url "${MIGRATE_DATABASE_URL}" --source migrations
