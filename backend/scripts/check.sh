#!/usr/bin/env bash
# Runs the back-end CI checks (fmt, clippy, tests) inside a Linux Rust
# container, for machines without the MSVC C++ build tools.
#
# Usage (from anywhere, in Git Bash):
#   backend/scripts/check.sh                 # fmt --check, clippy, all tests
#   backend/scripts/check.sh health          # same, but `cargo test health`
#   backend/scripts/check.sh -- --nocapture  # extra args go to `cargo test`
#
# Requires Docker running. The host Docker socket is mounted so that
# testcontainers can start PostgreSQL. Cargo registry and build output live in
# named volumes (tabwallet-cargo-registry, tabwallet-cargo-target), so reruns
# are fast. Remove them with `docker volume rm` to start clean.
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

exec docker run --rm --network host \
  -v "$backend_dir:/work" -w /work \
  -v tabwallet-cargo-registry:/usr/local/cargo/registry \
  -v tabwallet-cargo-target:/target \
  -v //var/run/docker.sock:/var/run/docker.sock \
  -e CARGO_TARGET_DIR=/target \
  -e TESTCONTAINERS_HOST_OVERRIDE=host.docker.internal \
  rust:1-bookworm \
  bash -euxc 'rustup component add clippy rustfmt >/dev/null 2>&1
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings
    cargo test "$@"' check "$@"
