# syntax=docker/dockerfile:1
#
# Production image of the API. Build context: this directory (`backend/`).
#   docker build -t tabwallet-api backend
# Configuration comes from environment variables only (DATABASE_URL,
# JWT_SECRET, APP_URL, ...; see backend/.env.example): no `.env` is needed or
# copied. The binary applies the sqlx migrations itself on startup.

FROM rust:1-bookworm AS builder
WORKDIR /app

# Dependencies first, against a stub crate, so this slow layer is reused until
# Cargo.toml or Cargo.lock change.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src \
    && echo 'fn main() {}' > src/main.rs \
    && touch src/lib.rs \
    && cargo build --release --locked --bin tabwallet_backend \
    && rm -rf src

# The real sources. `sqlx::migrate!` embeds ./migrations at compile time, so
# the directory must be here. `touch` makes Cargo see the sources as newer than
# the stub build above.
COPY migrations ./migrations
COPY src ./src
RUN touch src/main.rs src/lib.rs \
    && cargo build --release --locked --bin tabwallet_backend

FROM debian:bookworm-slim AS runtime

# CA certificates: the API calls the e-mail provider over HTTPS (reqwest with
# rustls uses the system trust store). The PostgreSQL TLS connection does not
# need them: sqlx bundles its own roots.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --no-create-home --shell /usr/sbin/nologin tabwallet

COPY --from=builder /app/target/release/tabwallet_backend /usr/local/bin/tabwallet_backend

USER tabwallet
ENV BIND_ADDR=0.0.0.0:8080 \
    RUST_LOG=info
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/tabwallet_backend"]
