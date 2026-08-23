#!/usr/bin/env bash
set -euo pipefail

# Production / staging entrypoint — run migrations via shared /bin/migrate.sh,
# then start the service binary.
source /bin/migrate.sh

exec ./ru_api_service
