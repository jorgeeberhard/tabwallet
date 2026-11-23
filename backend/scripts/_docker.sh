#!/usr/bin/env bash
# Shared by check.sh, load-check.sh and dev-api.sh: finds Docker, resolves the
# back-end directory and starts ONE throwaway PostgreSQL container (data in
# tmpfs) that is removed when the calling script ends, even on Ctrl-C.
#
# After `source`-ing this file and calling `start_postgres`:
#   $backend_dir  the back-end directory, as a path Docker Desktop accepts
#   $pg_port      the host port of the throwaway PostgreSQL
#   $pg_name      its container name
# `start_tls_postgres` adds a second one with TLS on ($tls_pg_port).

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
tls_pg_name="tabwallet-check-pg-tls-$$-$RANDOM"
cleanup() { docker rm -f "$pg_name" "$tls_pg_name" >/dev/null 2>&1 || true; }

# Waits until the container answers over TCP: during first start the temporary
# init server only listens on a socket. Prints the host port of PostgreSQL.
wait_for_postgres() {
  local name="$1" ready=0
  for _ in $(seq 1 90); do
    if docker exec "$name" pg_isready -h 127.0.0.1 -U postgres -q 2>/dev/null; then ready=1; break; fi
    sleep 1
  done
  if [ "$ready" != 1 ]; then echo "PostgreSQL test container $name did not become ready" >&2; exit 1; fi
  docker port "$name" 5432/tcp | head -n 1 | sed 's/.*://'
}

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

  pg_port="$(wait_for_postgres "$pg_name")"
}

# A second throwaway PostgreSQL with TLS on (self-signed certificate made at
# startup), like the managed database in production. Sets $tls_pg_port. Call it
# after `start_postgres`, which installs the cleanup trap.
start_tls_postgres() {
  docker run -d --name "$tls_pg_name" --label tabwallet-check=1 \
    -e POSTGRES_PASSWORD=postgres -p 127.0.0.1::5432 \
    --tmpfs /var/lib/postgresql/data \
    --entrypoint sh postgres:17-alpine -c '
      apk add --no-cache openssl >/dev/null
      openssl req -x509 -nodes -newkey rsa:2048 -days 1 -subj /CN=localhost \
        -keyout /tmp/server.key -out /tmp/server.crt 2>/dev/null
      chown postgres:postgres /tmp/server.key /tmp/server.crt
      chmod 600 /tmp/server.key
      exec docker-entrypoint.sh postgres -c fsync=off -c ssl=on \
        -c ssl_cert_file=/tmp/server.crt -c ssl_key_file=/tmp/server.key' >/dev/null

  tls_pg_port="$(wait_for_postgres "$tls_pg_name")"
}
