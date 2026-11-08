#!/usr/bin/env bash
# Formats the back-end with `cargo fmt` inside a Linux Rust container, for
# machines that cannot build Rust natively (check.sh only verifies the format).
#
# Usage (from anywhere, in Git Bash):
#   backend/scripts/fmt.sh
set -euo pipefail

# shellcheck source=_docker.sh
source "$(dirname "${BASH_SOURCE[0]}")/_docker.sh"

docker run --rm -v "$backend_dir:/work" -w /work rust:1-bookworm \
  bash -c 'rustup component add rustfmt >/dev/null 2>&1; cargo fmt'
