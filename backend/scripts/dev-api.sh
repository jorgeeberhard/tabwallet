#!/usr/bin/env bash
# Runs the API in a Linux Rust container against a throwaway PostgreSQL, for
# machines that cannot build Rust natively (use `cargo run` elsewhere). Handy to
# try the front-end by hand or in a browser.
#
# Usage (from anywhere, in Git Bash):
#   backend/scripts/dev-api.sh [port]     # default port 8080, on 127.0.0.1
#
# Then, in frontend/: API_URL=http://127.0.0.1:<port> npm run dev
# Stop it with Ctrl-C: the container is removed and the data is lost.
# The API runs with APP_ENV=development and COOKIE_SECURE=false (plain HTTP),
# and a throwaway JWT_SECRET. E-mails are written to the log, not sent.
set -euo pipefail

# shellcheck source=_docker.sh
source "$(dirname "${BASH_SOURCE[0]}")/_docker.sh"

port="${1:-8080}"
api_name="tabwallet-dev-api-$$-$RANDOM"
cleanup_all() { docker rm -f "$api_name" >/dev/null 2>&1 || true; cleanup; }

start_postgres
trap cleanup_all EXIT INT TERM

docker run --rm --name "$api_name" --label tabwallet-check=1 \
  -p "127.0.0.1:${port}:8080" \
  -v "$backend_dir:/work" -w /work \
  -v tabwallet-cargo-registry:/usr/local/cargo/registry \
  -v tabwallet-cargo-target:/target \
  -e CARGO_TARGET_DIR=/target \
  -e "DATABASE_URL=postgres://postgres:postgres@host.docker.internal:${pg_port}/postgres" \
  -e BIND_ADDR=0.0.0.0:8080 \
  -e APP_ENV=development \
  -e COOKIE_SECURE=false \
  -e JWT_SECRET=dev-only-secret-dev-only-secret-0123456789 \
  rust:1-bookworm \
  cargo run
