#!/usr/bin/env bash
# Runs the back-end CI checks (fmt, clippy, tests) inside a Linux Rust
# container, for machines without the MSVC C++ build tools.
#
# Usage (from anywhere, in Git Bash):
#   backend/scripts/check.sh                 # fmt --check, clippy, all tests
#   backend/scripts/check.sh health          # same, but `cargo test health`
#   backend/scripts/check.sh -- --nocapture  # extra args go to `cargo test`
#
# Requires Docker running. Each run starts ONE throwaway PostgreSQL container
# (data in tmpfs) and removes it when the script ends, even on Ctrl-C. Tests get
# it through TEST_DATABASE_URL, so testcontainers is not used here: it started a
# container per test binary and never removed them, which piled up and slowed
# the machine down. Cargo registry and build output live in named volumes
# (tabwallet-cargo-registry, tabwallet-cargo-target), so reruns are fast.
# Remove them with `docker volume rm` to start clean.
set -euo pipefail

if ! command -v docker >/dev/null 2>&1; then
  export PATH="$PATH:/c/Users/jomig/AppData/Local/Programs/DockerDesktop/resources/bin"
fi

# Stop Git Bash from rewriting container paths such as /work.
export MSYS_NO_PATHCONV=1

backend_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# Docker Desktop wants a Windows path (C:\...) for bind mounts.
if backend_win="$(cd "$backend_dir" && pwd -W 2>/dev/null)"; then
  backend_dir="$backend_win"
fi

# Drop an optional leading `--` separator.
if [ "${1:-}" = "--" ]; then shift; fi

pg_name="tabwallet-check-pg-$$-$RANDOM"
cleanup() { docker rm -f "$pg_name" >/dev/null 2>&1 || true; }
trap cleanup EXIT INT TERM

# Remove databases left behind by runs that were killed hard (older than 1 hour).
docker ps -q --filter label=tabwallet-check=1 --format '{{.ID}} {{.RunningFor}}' 2>/dev/null \
  | awk '/hour|day/ {print $1}' | xargs -r docker rm -f >/dev/null 2>&1 || true

docker run -d --name "$pg_name" --label tabwallet-check=1 \
  -e POSTGRES_PASSWORD=postgres -p 127.0.0.1::5432 \
  --tmpfs /var/lib/postgresql/data \
  postgres:17-alpine -c fsync=off -c max_connections=200 >/dev/null

# Check over TCP: during first start the temporary init server only listens on a socket.
ready=0
for _ in $(seq 1 90); do
  if docker exec "$pg_name" pg_isready -h 127.0.0.1 -U postgres -q 2>/dev/null; then ready=1; break; fi
  sleep 1
done
if [ "$ready" != 1 ]; then echo "PostgreSQL test container did not become ready" >&2; exit 1; fi
pg_port="$(docker port "$pg_name" 5432/tcp | head -n 1 | sed 's/.*://')"

docker run --rm --network host \
  -v "$backend_dir:/work" -w /work \
  -v tabwallet-cargo-registry:/usr/local/cargo/registry \
  -v tabwallet-cargo-target:/target \
  -e CARGO_TARGET_DIR=/target \
  -e "TEST_DATABASE_URL=postgres://postgres:postgres@host.docker.internal:${pg_port}/postgres" \
  rust:1-bookworm \
  bash -euxc 'rustup component add clippy rustfmt >/dev/null 2>&1
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings
    cargo test "$@"' check "$@"
