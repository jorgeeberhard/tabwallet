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

### Variáveis de ambiente

| Variável | Obrigatória | Descrição |
| --- | --- | --- |
| `DATABASE_URL` | sim | Conexão com o PostgreSQL. |
| `JWT_SECRET` | sim | Segredo que assina os tokens de sessão (HS256), com pelo menos 32 caracteres. Gere um por ambiente (`openssl rand -base64 48`) e nunca versione. |
| `COOKIE_SECURE` | não | O cookie de sessão é `Secure` (só HTTPS) a menos que seja `false`. Só use `false` no desenvolvimento local, sem HTTPS. |
| `BIND_ADDR` | não | Endereço de escuta (padrão `0.0.0.0:8080`). |
| `APP_ENV` | não | `development` ou `production` (padrão). Só em `development` a API aceita subir sem provedor de e-mail e assume `APP_URL=http://localhost:3000`. |
| `APP_URL` | em produção | Endereço público do front-end, usado no link do e-mail de recuperação de senha. |
| `EMAIL_PROVIDER` | em produção | `resend` (API HTTP no formato do Resend). Sem ele, em desenvolvimento, o e-mail só é escrito no log. |
| `EMAIL_API_KEY`, `EMAIL_FROM` | com `EMAIL_PROVIDER` | Chave da API (nunca versione) e remetente, por exemplo `TabWallet <no-reply@seu-dominio>`. |
| `EMAIL_API_URL` | não | Endereço da API de e-mail (padrão `https://api.resend.com/emails`). |

## Autenticação (UC01 e UC02)

| Rota | Efeito |
| --- | --- |
| `POST /auth/signup` `{email, password}` | Cria a conta (senha com 8 a 128 caracteres, e-mail único sem diferenciar maiúsculas) e já inicia a sessão. `201`. |
| `POST /auth/login` `{email, password}` | Inicia a sessão. Erro igual para e-mail desconhecido e senha errada. |
| `POST /auth/logout` | Expira o cookie. `204`, mesmo sem sessão. |
| `GET /auth/me` | Rota protegida: devolve o usuário da sessão. |

A senha é guardada com Argon2id. A sessão é um JWT (7 dias) no cookie `tabwallet_session`: `HttpOnly`, `SameSite=Lax`, `Path=/` e `Secure` conforme `COOKIE_SECURE`. O front-end acessa a API pela mesma origem (rewrite do Next), o que evita CORS e CSRF entre sites.

Erros têm sempre a forma `{"error": {"code", "message", "fields"?}}`; o front-end usa o `code` estável (`email_taken`, `invalid_credentials`, `unauthenticated`, `validation_failed`, `invalid_request`).

## Perfil e recuperação de senha (UC14 e UC03)

| Rota | Efeito |
| --- | --- |
| `PATCH /profile` (protegida) `{name?, email?, new_password?, current_password?}` | Edita só o que vier no corpo e devolve `{user}`. Trocar `email` ou `new_password` exige `current_password`: ausente dá `422` (`fields.current_password = "required"`), errada dá `403` (`invalid_current_password`). Trocar a senha cancela os links de recuperação pendentes. `GET /auth/me` devolve também `name` (vazio se não definido). |
| `POST /auth/password-reset/request` `{email}` | Envia por e-mail um link de redefinição para o e-mail de login. Responde `202` igual com ou sem conta, e também se o provedor falhar (só vai ao log), para não revelar quais e-mails existem. Um novo pedido invalida o link anterior. |
| `POST /auth/password-reset/confirm` `{token, password}` | Define a nova senha. `204` em caso de sucesso (não inicia sessão); `400 invalid_reset_token` para link desconhecido, expirado ou já usado. |

O link é `APP_URL/reset-password?token=...`, vale 60 minutos e só funciona uma vez. O token tem 64 caracteres hexadecimais aleatórios e o banco guarda apenas o SHA-256 dele. O envio passa pelo trait `EmailSender` (`src/email.rs`): `HttpEmailSender` em produção, `LogEmailSender` (escreve o link no log, nível info) em desenvolvimento sem provedor, e um fake que grava as mensagens nos testes. Não há verificação de e-mail no cadastro.

**Limitação conhecida.** A sessão é um JWT sem estado, que não pode ser revogado: depois de trocar a senha (no perfil ou pela recuperação), os navegadores já logados continuam logados até o token expirar (7 dias). O envio do e-mail é aguardado antes de responder, então o tempo de resposta pode, em tese, diferir para e-mails cadastrados; a resposta em si é idêntica.

**Rotas protegidas.** Registre a rota em `protected` em `src/app.rs`: o middleware `require_auth` devolve `401` sem sessão válida e o handler recebe o usuário como argumento `AuthUser`. Todo repositório recebe o `UserId` (`auth_user.id`) e filtra por ele, para que dados de um usuário nunca apareçam para outro.

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

Os argumentos extras vão para o `cargo test`. A cada execução o script sobe um
único PostgreSQL descartável (dados em memória) e o remove ao terminar, mesmo
com Ctrl-C; os testes o recebem por `TEST_DATABASE_URL`, sem testcontainers, que
deixava um contêiner por binário de teste para trás. O registro e o `target` do
cargo ficam em volumes nomeados (`tabwallet-cargo-registry` e
`tabwallet-cargo-target`), então as execuções seguintes são rápidas. Execuções
simultâneas dividem o mesmo `target`, então uma espera a outra compilar.

## CI

O workflow `.github/workflows/ci.yml` roda a cada PR e a cada push na `main`.
O merge na `main` deve exigir o check `backend`.
