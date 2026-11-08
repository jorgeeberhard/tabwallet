#!/usr/bin/env bash
# Load check for the performance requirement (95% of the API requests in up to
# 300 ms): seeds one realistic user (about 2,000 transactions, fixed
# transactions, four cards with purchases and installment purchases, budgets)
# and prints p50/p95/max per main endpoint, first with one client and then with
# several at once. See backend/tests/load_check.rs.
#
# Usage (from anywhere, in Git Bash):
#   backend/scripts/load-check.sh
#   LOAD_CHECK_TRANSACTIONS=10000 backend/scripts/load-check.sh   # bigger user
#   LOAD_CHECK_P95_MS=200 backend/scripts/load-check.sh           # stricter limit
#
# Release build in a Linux Rust container, plus ONE throwaway PostgreSQL, both
# on a private Docker network (so the API reaches the database directly, as it
# does when both run in the same data center, without Docker Desktop's port
# proxy); everything is removed at the end. The first run compiles in release
# mode, which takes several minutes; reruns are fast. The test fails when any
# endpoint's p95 is above the limit (300 ms by default). The server runs in the
# test process and is called over loopback HTTP: no TLS and no Next.js proxy in
# the path.
set -euo pipefail

# shellcheck source=_docker.sh
source "$(dirname "${BASH_SOURCE[0]}")/_docker.sh"

network="tabwallet-load-$$-$RANDOM"
cleanup_all() { cleanup; docker network rm "$network" >/dev/null 2>&1 || true; }

docker network create "$network" >/dev/null
trap cleanup_all EXIT INT TERM
start_postgres --network "$network" --network-alias pg
trap cleanup_all EXIT INT TERM

docker run --rm --network "$network" \
  -v "$backend_dir:/work" -w /work \
  -v tabwallet-cargo-registry:/usr/local/cargo/registry \
  -v tabwallet-cargo-target:/target \
  -e CARGO_TARGET_DIR=/target \
  -e "TEST_DATABASE_URL=postgres://postgres:postgres@pg:5432/postgres" \
  -e LOAD_CHECK_P95_MS -e LOAD_CHECK_TRANSACTIONS \
  rust:1-bookworm \
  cargo test --release --test load_check -- --ignored --nocapture
