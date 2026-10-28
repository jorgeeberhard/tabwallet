# TabWallet back-end

API em Rust/Axum sobre PostgreSQL. Organização em camadas (termo de abertura, seção 4.3):

- `src/handlers`: rotas HTTP; validam a entrada e chamam um use case.
- `src/use_cases`: um `*UseCase` por operação de negócio, sem conhecimento de HTTP.
- `src/repositories`: único código que fala com o PostgreSQL (`sqlx`).
- `src/clock.rs`: fonte de "hoje" (dia em America/Sao_Paulo), substituível nos testes.
- `migrations/`: migrations versionadas do `sqlx`, aplicadas na subida da API e nos testes.

## Pré-requisitos

- Rust (stable) e, no Windows, as ferramentas de build C++ do Visual Studio.
- Docker em execução (banco local e testes de integração).

## Rodar a API localmente

```bash
cd backend
docker compose up -d          # PostgreSQL em localhost:5432
cp .env.example .env
cargo run                     # aplica as migrations e sobe em 127.0.0.1:8080
curl http://127.0.0.1:8080/health
```

A resposta esperada é `{"status":"ok","database":"up","today":"AAAA-MM-DD"}`.

## Rodar os testes

```bash
cd backend
cargo test
```

Os testes de integração chamam a API HTTP contra um PostgreSQL real, iniciado
automaticamente em um contêiner via testcontainers (o Docker precisa estar
rodando). Cada teste recebe um banco novo e já migrado, e controla o "hoje"
com `TestApp::set_today` (ver `tests/common/`). Para usar um PostgreSQL já
existente em vez do contêiner, defina `TEST_DATABASE_URL` com um usuário que
possa executar `CREATE DATABASE`.

Antes de abrir um PR, o CI executa também:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

## Rodar as verificações sem as ferramentas de build C++ (Windows)

Em máquinas sem as ferramentas de build C++ do Visual Studio (o `cargo` nativo
falha ao linkar), use o script que executa tudo dentro de um contêiner Rust
Linux, no Git Bash, com o Docker em execução:

```bash
backend/scripts/check.sh            # fmt --check, clippy e todos os testes
backend/scripts/check.sh health     # mesmo, mas só os testes com "health" no nome
```

Os argumentos extras vão para o `cargo test`. O script monta o socket do Docker
do host (para o testcontainers subir o PostgreSQL) e guarda o registro e o
`target` do cargo em volumes nomeados (`tabwallet-cargo-registry` e
`tabwallet-cargo-target`), então as execuções seguintes são rápidas.

## CI

O workflow `.github/workflows/ci.yml` roda a cada PR e a cada push na `main`.
O merge na `main` deve exigir o check `backend`.
