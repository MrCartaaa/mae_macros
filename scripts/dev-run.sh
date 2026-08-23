#!/usr/bin/env bash
set +e

bash scripts/migrate.sh
migrate_rc=$?

if [ $migrate_rc -ne 0 ]; then
  echo
  echo "migrations failed with exit code $migrate_rc"
  echo "waiting for file changes..."
  exit 0
fi

cargo run
run_rc=$?

if [ $run_rc -ne 0 ]; then
  echo
  echo "cargo run failed with exit code $run_rc"
  echo "waiting for file changes..."
fi

exit 0
