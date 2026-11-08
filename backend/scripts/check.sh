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
# (data in tmpfs, see _docker.sh) and removes it when the script ends, even on
# Ctrl-C. Tests get it through TEST_DATABASE_URL, so testcontainers is not used
# here: it started a container per test binary and never removed them, which
# piled up and slowed the machine down. Cargo registry and build output live in
# named volumes (tabwallet-cargo-registry, tabwallet-cargo-target), so reruns
# are fast. Remove them with `docker volume rm` to start clean.
set -euo pipefail

# shellcheck source=_docker.sh
source "$(dirname "${BASH_SOURCE[0]}")/_docker.sh"

# Drop an optional leading `--` separator.
if [ "${1:-}" = "--" ]; then shift; fi

start_postgres

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
