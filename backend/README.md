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

## Categorias (UC09)

Todas as rotas exigem sessão e só enxergam as categorias do próprio usuário (uma categoria de outro usuário responde como inexistente, `404`). Uma categoria é `{id, name, type, active, is_charges, created_at}`, com `type` igual a `income` ou `expense`.

| Rota | Efeito |
| --- | --- |
| `GET /categories?type=&active=` | Lista as categorias (ativas e desativadas); os dois filtros são opcionais. Um formulário de lançamento usa `?type=expense&active=true`. |
| `POST /categories` `{name, type}` | Cria uma categoria ativa. `201`. |
| `PATCH /categories/{id}` `{name}` | Renomeia. |
| `POST /categories/{id}/deactivate` | Desativa (nunca exclui); repetir não muda nada. |
| `POST /categories/{id}/reactivate` | Reativa. |

O nome é aparado, tem de 1 a 60 caracteres e é único por tipo e por usuário sem diferenciar maiúsculas, entre ativas e desativadas. Erros de regra (`409`): `category_name_taken` (já existe uma ativa) e `category_inactive_exists` (já existe uma desativada; a resposta traz `error.category_id`, para o front-end oferecer a reativação). Também `category_has_charges` e `category_not_found` (`404`).

Todo usuário nasce com as categorias padrão (função SQL `seed_default_categories`, chamada na transação do cadastro; a migration `0003` também a aplica aos usuários que já existiam). "Juros e encargos" é marcada pela coluna `is_charges`, não pelo nome. A regra "não desativar enquanto houver Encargos" tem um ponto único, `CategoryRepository::has_charges`, que hoje devolve `false` porque ainda não há Encargos; o ticket que criar os Encargos preenche essa consulta.

## Transações (UC04, UC06, UC07 e UC08)

Receitas e Despesas fora do cartão. Todas as rotas exigem sessão e só enxergam as Transações do próprio usuário (a de outro usuário responde `404 transaction_not_found`). Uma Transação é `{id, type, amount_cents, date, description, category: {id, name, type, active}, is_projection, created_at}`: o valor é sempre um inteiro de centavos maior que zero (o `type`, `income` ou `expense`, dá o sinal), `date` é só o dia (`AAAA-MM-DD`, sem horário) e `is_projection` é verdadeiro quando a data é depois de "hoje" (America/Sao_Paulo, pelo relógio injetável). A Projeção é calculada a cada leitura e nunca gravada.

| Rota | Efeito |
| --- | --- |
| `POST /transactions` `{type, amount_cents, date, description, category_id}` | Cria. `201 {transaction}`. |
| `GET /transactions?from=&to=&category_id=&type=&limit=&offset=` | Lista do mais novo para o mais antigo (data, depois criação, depois id: ordem total). `from` e `to` são dias inclusivos; `limit` vai de 1 a 200 (padrão 50). Devolve `{items, total, limit, offset}`; `total` conta tudo que casa com os filtros. |
| `PATCH /transactions/{id}` `{amount_cents?, date?, description?, category_id?}` | Edita só o que vier no corpo. O tipo não muda. `200 {transaction}`. |
| `DELETE /transactions/{id}` | Exclui. `204`. |

Regras (erros `422 validation_failed`, com `error.fields`): `amount_cents` entre 1 e 100.000.000.000 (`amount_range`); `description` aparada, de 1 a 200 caracteres (`description_length`); a Categoria (`category_id`) precisa ser do usuário (`category_not_found`), do mesmo tipo da Transação (`category_type_mismatch`) e ativa (`category_inactive`). Ao editar, só uma Categoria *diferente* da atual é revalidada: uma Transação cuja Categoria foi desativada continua editável. Na listagem, `from` depois de `to` dá `fields.to = period_order`, e `limit`/`offset` fora da faixa dão `limit_range`/`offset_range`. Data que não é um dia (`2026-02-30`, com horário) ou valor não inteiro dão `400 invalid_request`.

**Pronto para os próximos tickets.** Cada entrada da listagem tem `kind` (`"transaction"`, `"card_purchase"` ou `"occurrence"`, ver Cartões de crédito e Transações fixas), e o use case devolve um enum `StatementItem`: Pagamentos de fatura entram como nova variante, com a sua tabela e o seu formato, sem forçar a tabela `transactions` a carregar campos de cartão. O filtro por cartão (`credit_card_id`) já está em `TransactionFilter`/`ListParams` e deixa as Ocorrências de fora. A tabela `transactions` referencia `(category_id, type)`, então o banco também recusa uma Categoria do outro tipo.

## Saldo atual e Saldo inicial (UC10, parcial, e UC22)

Rotas protegidas; cada usuário só enxerga o próprio saldo. Valores em centavos inteiros.

| Rota | Efeito |
| --- | --- |
| `GET /balance` | Devolve `{current_balance_cents, opening_balance_set, opening_balance_cents}`. `opening_balance_cents` é `null` enquanto o Saldo inicial não foi informado. |
| `POST /balance/opening` `{amount_cents}` | Informa o Saldo inicial, uma única vez. `201` com o mesmo corpo do `GET /balance`. Zero e negativo valem (mesmo zero conta como informado); `amount_cents` fora de ±100.000.000.000 dá `422` (`fields.amount_cents = amount_range`) e não gasta a única chance. Uma segunda tentativa dá `409 opening_balance_already_set` e o valor guardado não muda. |

**Regra.** Saldo atual = Saldo inicial (0 se não informado) + Receitas - Despesas com data até hoje (America/Sao_Paulo, pelo relógio injetável). Projeções (data depois de hoje) ficam de fora até o dia chegar, sem nenhum job: a cada leitura o saldo é calculado de novo (ADR-0002). O saldo pode ficar negativo e nada bloqueia um lançamento por isso. O Saldo inicial fica na coluna `users.opening_balance_cents` (`NULL` = não informado); gravá-lo é um único `UPDATE ... WHERE opening_balance_cents IS NULL`, então dois pedidos simultâneos não passam os dois.

**Módulo de cálculo (`src/ledger.rs`).** Puro: não conhece HTTP nem SQL. Os repositórios entregam os fatos de um usuário (`LedgerFacts`: Saldo inicial e `CashFlow`s por dia e tipo) e o use case pergunta com um "hoje": `Ledger::new(facts).current_balance(today)`. O `TransactionRepository::cash_flows` soma por dia e tipo; qual dia conta é decisão do ledger. Os próximos tickets (Transações fixas, Faturas, Saldo projetado, Orçamentos, relatórios) ampliam `LedgerFacts` e acrescentam perguntas ao `Ledger`, sem mudar quem já o usa. Os testes ficam em `tests/balance.rs`, pela API, movendo o "hoje" com `set_today`.

## Cartões de crédito, compras e Faturas (UC15, UC16 e a parte de cartão de UC06, UC07 e UC08)

Rotas protegidas; cada usuário só enxerga os próprios cartões e compras (o de outro usuário responde `404`, ou `422` com `credit_card_not_found` ao lançar uma compra). Valores em centavos inteiros.

| Rota | Efeito |
| --- | --- |
| `POST /credit-cards` `{name, closing_day, due_day}` | Cadastra o cartão (nome de 1 a 60 caracteres, único por usuário sem diferenciar maiúsculas: `409 credit_card_name_taken`; dias de 1 a 31: `fields.* = day_range`). `201 {credit_card}`. |
| `GET /credit-cards` | `{credit_cards}`, ativos primeiro. Cada um é `{id, name, closing_day, due_day, active, created_at}`; os dias são os que o cartão tem daqui para frente. |
| `PATCH /credit-cards/{id}` `{name?, closing_day?, due_day?}` | Renomeia e/ou muda os dias. Os dias novos valem só para Faturas ainda abertas. |
| `POST /credit-cards/{id}/deactivate` | Desativa (marcação, nunca exclusão). `409 credit_card_has_remaining_invoice` se alguma Fatura tem valor restante. Repetir não muda nada. |
| `POST /credit-cards/{id}/reactivate` | Reativa. |
| `GET /credit-cards/{id}/invoices` | `{invoices}`, da mais antiga para a mais nova: as que têm compras e a primeira aberta (mesmo vazia). |
| `GET /credit-cards/{id}/invoices/{closing_month}` | `{invoice, purchases}` de uma Fatura (`AAAA-MM`, o mês em que ela fecha); `404 invoice_not_found` se não existe. |
| `POST /card-purchases` `{credit_card_id, amount_cents, date, description, category_id}` | Lança uma Compra no cartão. `201 {card_purchase}`. |
| `PATCH /card-purchases/{id}` `{amount_cents?, date?, description?, category_id?}` | Edita; uma nova data realoca a compra. O cartão não muda. |
| `DELETE /card-purchases/{id}` | Exclui. `204`. |
| `GET /transactions?credit_card_id=` | O extrato ganha itens `kind: "card_purchase"` e o filtro por cartão (que deixa só as compras dele; `type=income` exclui as compras). |

Uma Fatura é `{credit_card_id, closing_month, closing_date, due_date, reference_month, state, total_cents, remaining_cents}`, com `state` igual a `open`, `closed` ou `paid`. Uma compra é `{id, amount_cents, date, description, category, credit_card: {id, name, active}, invoice: {closing_month, closing_date, due_date, reference_month, state}, reference_month, is_projection, created_at}`. `reference_month` (`AAAA-MM`) é o Mês de referência: o mês do vencimento da Fatura.

**Regras.**

- Compra: valor entre 1 e 100.000.000.000, descrição de 1 a 200 caracteres, Categoria ativa de Despesa (`category_*`), cartão do usuário e ativo (`fields.credit_card_id = credit_card_not_found | credit_card_inactive`).
- Fechamento: a compra feita até o Dia de fechamento (inclusive) entra na Fatura que fecha no mês; depois dele, na do mês seguinte. Em mês mais curto vale o último dia (fechamento 31 em fevereiro fecha dia 28).
- Vencimento: o primeiro dia depois do fechamento que cai no Dia de vencimento (vencimento depois do fechamento: no mesmo mês; senão, no mês seguinte).
- Estado derivado das datas, nunca gravado: `open` até o fechamento (inclusive), `closed` depois dele até o vencimento, `paid` a partir do vencimento. O `remaining_cents` é o total enquanto a Fatura não está paga e zero depois. Até o ticket de Pagamento de fatura, é o vencimento que quita; os pagamentos entram então no `Ledger::invoices`.
- Compra de Fatura `paid` não pode ser editada nem excluída (`409 invoice_paid`), e uma data que a colocaria numa Fatura paga é recusada (`422`, `fields.date = invoice_paid`), tanto ao lançar quanto ao editar.
- A Compra no cartão não entra no Saldo atual (`GET /balance` não muda).
- Os dias do cartão têm histórico (`credit_card_schedules`): uma mudança começa na primeira Fatura ainda aberta, ou na seguinte se o novo Dia de fechamento já tiver passado para ela (assim nenhuma compra muda de Fatura). Faturas fechadas e pagas mantêm as datas que tinham.

**Módulo de cálculo.** `src/ledger/cards.rs` (puro, como o resto do `ledger`): `LedgerFacts.cards` leva, por cartão, o histórico de dias (`CardSchedule`) e os gastos por dia (`CardSpend`). `Ledger::invoices(card, hoje)`, `Ledger::invoice(card, mês, hoje)`, `Ledger::invoice_for_purchase(card, data, hoje)` e `Ledger::has_remaining_invoice(card, hoje)` respondem sobre as Faturas; o Mês de referência é `InvoiceCycle::reference_month`. A Compra no cartão fica de fora de `cash_flows`, então `current_balance` continua ignorando-a. O extrato (`StatementRepository`) ordena e pagina por `UNION ALL` das tabelas `transactions` e `card_purchases`; uma nova variante de `StatementItem` entra com mais um ramo da consulta. As Ocorrências de Transações fixas são calculadas no use case e juntadas a essa lista em memória; com `credit_card_id` elas ficam de fora.

## Transações fixas e Ocorrências (UC05, UC06 e UC07)

Gastos fixos e Receitas fixas. Todas as rotas exigem sessão e só enxergam o que é do próprio usuário (id de outro usuário ou malformado responde `404 recurring_transaction_not_found`). Uma Transação fixa é `{id, type, amount_cents, day_of_month, description, category: {id, name, type, active}, start_month, end_month, ended, next_date, created_at}`: `amount_cents` é o valor em vigor no mês atual (o do primeiro ou do último mês se ainda não começou ou já terminou), `start_month` e `end_month` são `AAAA-MM` e `next_date` é o dia da próxima Ocorrência depois de hoje, ou `null`.

| Rota | Efeito |
| --- | --- |
| `POST /recurring-transactions` `{type, amount_cents, day_of_month, description, category_id, start_month?}` | Cadastra. `201 {recurring_transaction}`. `day_of_month` vai de 1 a 31; `start_month` (`AAAA-MM`) é o mês da primeira Ocorrência, por padrão o mês de hoje, e um mês anterior dá `422` (`fields.start_month = start_month_past`). |
| `GET /recurring-transactions` | Lista as Transações fixas (encerradas também) da mais antiga para a mais nova: `{recurring_transactions}`. |
| `PATCH /recurring-transactions/{id}` `{amount_cents?, description?, category_id?}` | Um valor novo vale **a partir do mês da alteração** (o mês de hoje, ou o de início se ainda não começou) e não mexe em meses anteriores, nem na Ocorrência que o usuário editou à mão; trocar de novo no mesmo mês substitui a alteração. Descrição e Categoria valem para todas as Ocorrências (não mudam o saldo). O tipo e o dia não mudam. Encerrada: `409 recurring_transaction_ended`. |
| `POST /recurring-transactions/{id}/end` `{last_month?}` | Encerra: as Projeções futuras somem e as Ocorrências que já caíram ficam. Sem corpo, a última Ocorrência é a do mês de hoje se o dia dela já chegou, senão a do mês anterior. `last_month` (`AAAA-MM`) escolhe um mês final posterior; um anterior à última Ocorrência já ocorrida dá `422` (`fields.last_month = last_month_before_history`). Encerrar de novo dá `409 recurring_transaction_ended`. |
| `PATCH /recurring-transactions/{id}/occurrences/{month}` `{amount_cents}` | Edita só o valor da Ocorrência desse mês (`AAAA-MM`). `200 {occurrence}`. |
| `DELETE /recurring-transactions/{id}/occurrences/{month}` | Exclui só a Ocorrência desse mês. `204`. |

Mês sem Ocorrência (antes do início, depois do fim, já excluída ou malformado) responde `404 occurrence_not_found`. Validações como as de `/transactions`: `amount_range`, `description_length`, `day_of_month_range` e as regras de Categoria (`category_not_found`, `category_type_mismatch`, `category_inactive`).

**Ocorrência.** `{recurring_id, month, type, amount_cents, date, description, category, is_projection, is_edited}`. Cai em `day_of_month`, ou no último dia do mês quando ele é mais curto (28 ou 29 em fevereiro, 30 em abril). `is_projection` é verdadeiro se a data é depois de hoje; `is_edited`, se o usuário mudou o valor só desse mês.

**Nada é gravado pelas Ocorrências (ADR-0002).** O banco guarda a definição (`recurring_transactions`), as versões de valor (`recurring_amounts`, uma por mês de alteração, a primeira no mês de início) e as exceções (`recurring_exceptions`: um mês excluído ou com valor editado). Cada leitura recalcula as Ocorrências; avançar o relógio já as faz entrar no Saldo atual, sem job.

**Listagem.** `GET /transactions` passa a devolver também Ocorrências, como itens `{"kind": "occurrence", ...}` com o formato acima (a variante `StatementItem::Occurrence`). Os filtros `from`, `to`, `category_id` e `type` valem para elas, e a paginação junta Transações e Ocorrências numa ordem só (mais novas primeiro). Sem `to`, a lista vai até o fim do mês 12 meses à frente (`ledger::projection_horizon`); um `to` além disso é cortado ali. O `total` conta as Ocorrências dessa janela. Um filtro por cartão (ticket de cartões) deve excluir as Ocorrências: elas nunca passam por Fatura.

**Módulo de cálculo.** `LedgerFacts` ganhou `recurring: Vec<RecurringDefinition>` (dia, mês de início e de fim, versões de valor e exceções; `src/ledger/recurring.rs`). `Ledger::current_balance` soma as Ocorrências até hoje, e `Ledger::occurrences(from, to)` e `Ledger::occurrence(id, month)` respondem à listagem e às ações sobre uma Ocorrência. Os testes ficam em `tests/recurring.rs` (pela API, avançando o relógio por vários meses) e, para a regra do último dia do mês, nos testes unitários de `ledger/recurring.rs`.

## Histórico do saldo e Saldo projetado (UC10)

Rotas protegidas; cada usuário só enxerga o próprio saldo. Valores em centavos inteiros. As duas devolvem uma série de pontos `{date, balance_cents}`: o saldo no **fim** do dia `date`.

| Rota | Efeito |
| --- | --- |
| `GET /balance/history?from=&to=&granularity=day\|month` | Histórico do Saldo atual até hoje. `granularity` é `day` (padrão) ou `month`; `to` (padrão: hoje) não pode ser depois de hoje; `from` (padrão: 30 dias, ou 12 meses, até `to`). Devolve `{granularity, from, to, points}`. |
| `GET /balance/projection?to=&granularity=day\|month` | Saldo projetado de hoje até `to` (padrão: o horizonte). Devolve `{granularity, today, to, current_balance_cents, points}`. |

**Pontos.** Por dia, um ponto para cada dia de `from` a `to` (na projeção, de hoje a `to`: o primeiro ponto é o Saldo atual). Por mês, um ponto para cada mês do período, no último dia do mês, cortado em `to`: o mês que contém `to` mostra o saldo em `to` (no histórico, o mês corrente mostra o saldo de hoje); na projeção o primeiro ponto é o fim do mês corrente.

**Regras.** O histórico é o Saldo atual que a pessoa tinha no fim de cada dia ou mês: Saldo inicial mais Receitas e Despesas fora do cartão com data até ali, incluindo as Ocorrências que já tinham caído. O Saldo projetado em uma data é o Saldo atual mais Receitas e Despesas fora do cartão com data depois de hoje até ela, com Projeções e Ocorrências futuras (inclusive edições e exclusões de uma só Ocorrência). Compras no cartão ainda não lançadas não entram. As Faturas ainda não entram: o ticket de pagamento de fatura vai subtrair, em cada ponto, o valor restante das Faturas com vencimento até a data (ponto único de extensão: `Ledger::projected_balances_at`), sem mudar a rota.

**Horizonte e erros.** O horizonte é o fim do mês 12 meses depois do mês de hoje (`ledger::projection_horizon`, o mesmo da listagem). Erros `422 validation_failed` com `fields`: `to = history_in_future` (histórico além de hoje), `to = projection_in_past` (projeção antes de hoje), `to = beyond_horizon` (projeção além do horizonte), `to = period_order` (`from` depois de `to`), `from = period_too_long` (mais de 1.000 pontos). Data inválida ou `granularity` desconhecida dão `400 invalid_request`.

**Módulo de cálculo.** `src/ledger/series.rs` acrescenta `Ledger::balance_history(from, to, granularity, today)` e `Ledger::projected_balance(to, granularity, today)`, ambos varrendo uma só lista ordenada de fluxos e Ocorrências (sem consulta por ponto). Os testes ficam em `tests/balance_series.rs`, pela API, movendo o "hoje" com `set_today`.

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
