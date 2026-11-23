#!/usr/bin/env bash
# Shared by check.sh, load-check.sh and dev-api.sh: finds Docker, resolves the
# back-end directory and starts ONE throwaway PostgreSQL container (data in
# tmpfs) that is removed when the calling script ends, even on Ctrl-C.
#
# After `source`-ing this file and calling `start_postgres`:
#   $backend_dir  the back-end directory, as a path Docker Desktop accepts
#   $pg_port      the host port of the throwaway PostgreSQL
#   $pg_name      its container name

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

pg_name="tabwallet-check-pg-$$-$RANDOM"
cleanup() { docker rm -f "$pg_name" >/dev/null 2>&1 || true; }

# Extra arguments go to `docker run` (for instance `--network`).
start_postgres() {
  trap cleanup EXIT INT TERM

  # Remove databases left behind by runs that were killed hard (older than 1 hour).
  docker ps -q --filter label=tabwallet-check=1 --format '{{.ID}} {{.RunningFor}}' 2>/dev/null \
    | awk '/hour|day/ {print $1}' | xargs -r docker rm -f >/dev/null 2>&1 || true

  docker run -d --name "$pg_name" --label tabwallet-check=1 "$@" \
    -e POSTGRES_PASSWORD=postgres -p 127.0.0.1::5432 \
    --tmpfs /var/lib/postgresql/data \
    postgres:17-alpine -c fsync=off -c max_connections=200 >/dev/null

  # Check over TCP: during first start the temporary init server only listens on a socket.
  local ready=0
  for _ in $(seq 1 90); do
    if docker exec "$pg_name" pg_isready -h 127.0.0.1 -U postgres -q 2>/dev/null; then ready=1; break; fi
    sleep 1
  done
  if [ "$ready" != 1 ]; then echo "PostgreSQL test container did not become ready" >&2; exit 1; fi
  pg_port="$(docker port "$pg_name" 5432/tcp | head -n 1 | sed 's/.*://')"
}
