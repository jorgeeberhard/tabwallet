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

A senha é guardada com Argon2id. A sessão é um JWT (7 dias) no cookie `tabwallet_session`: `HttpOnly`, `SameSite=Lax`, `Path=/` e `Secure` conforme `COOKIE_SECURE`. O front-end acessa a API pela mesma origem (rewrite do Next), o que evita CORS.

### Proteções (requisitos não funcionais)

- **CSRF.** Três camadas: o cookie é `SameSite=Lax` (o navegador não o envia em POST, PUT, PATCH ou DELETE vindos de outro site); não há CORS, então outro site não consegue chamar a API por script; e `src/csrf.rs` recusa com `403 forbidden_origin` qualquer requisição que altera dados (tudo que não é GET, HEAD ou OPTIONS) cujo `Origin` (ou, sem ele, `Referer`) não seja o endereço do front-end (`APP_URL`). Requisições sem nenhum dos dois (curl, testes) passam. Com `APP_URL` apontando para `localhost`, qualquer nome local (`127.0.0.1`, `*.localhost`) é aceito; em produção só a origem exata. **`APP_URL` precisa ser exatamente o endereço pelo qual as pessoas abrem o front-end.**
- **Limite de tentativas.** Login: 5 senhas erradas em 15 minutos por e-mail (vale também para e-mail sem conta, para não revelar quais existem); depois disso, `429 too_many_requests` com `Retry-After`, até para a senha certa, e um login certo zera a contagem. Pedidos de recuperação de senha: 3 por hora por e-mail, contados mesmo sem conta, sem enviar e-mail além do limite. As contagens ficam na memória da instância (`src/rate_limit.rs`).
- **SQL injection.** Todas as consultas usam parâmetros do `sqlx` (`.bind`); nenhum texto de usuário é concatenado em SQL (o `sqlx` 0.9 nem aceita SQL montado em tempo de execução sem `AssertSqlSafe`, usado só em testes).
- **XSS.** A API só devolve JSON e o e-mail de recuperação é texto puro.

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

Todo usuário nasce com as categorias padrão (função SQL `seed_default_categories`, chamada na transação do cadastro; a migration `0003` também a aplica aos usuários que já existiam). "Juros e encargos" é marcada pela coluna `is_charges`, não pelo nome. A regra "não desativar enquanto houver Encargos" tem um ponto único, `CategoryRepository::has_charges`, que consulta a tabela `invoice_charges` (Encargos, UC21): enquanto houver um Encargo na Categoria, a desativação dá `409 category_has_charges`.

## Transações (UC04, UC06, UC07 e UC08)

Receitas e Despesas fora do cartão. Todas as rotas exigem sessão e só enxergam as Transações do próprio usuário (a de outro usuário responde `404 transaction_not_found`). Uma Transação é `{id, type, amount_cents, date, description, category: {id, name, type, active}, is_projection, created_at}`: o valor é sempre um inteiro de centavos maior que zero (o `type`, `income` ou `expense`, dá o sinal), `date` é só o dia (`AAAA-MM-DD`, sem horário) e `is_projection` é verdadeiro quando a data é depois de "hoje" (America/Sao_Paulo, pelo relógio injetável). A Projeção é calculada a cada leitura e nunca gravada.

| Rota | Efeito |
| --- | --- |
| `POST /transactions` `{type, amount_cents, date, description, category_id}` | Cria. `201 {transaction}`. |
| `GET /transactions?from=&to=&category_id=&type=&limit=&offset=` | Lista do mais novo para o mais antigo (data, depois criação, depois id: ordem total). `from` e `to` são dias inclusivos; `limit` vai de 1 a 200 (padrão 50). Devolve `{items, total, limit, offset}`; `total` conta tudo que casa com os filtros. |
| `PATCH /transactions/{id}` `{amount_cents?, date?, description?, category_id?}` | Edita só o que vier no corpo. O tipo não muda. `200 {transaction}`. |
| `DELETE /transactions/{id}` | Exclui. `204`. |

Regras (erros `422 validation_failed`, com `error.fields`): `amount_cents` entre 1 e 100.000.000.000 (`amount_range`); `description` aparada, de 1 a 200 caracteres (`description_length`); a Categoria (`category_id`) precisa ser do usuário (`category_not_found`), do mesmo tipo da Transação (`category_type_mismatch`) e ativa (`category_inactive`). Ao editar, só uma Categoria *diferente* da atual é revalidada: uma Transação cuja Categoria foi desativada continua editável. Na listagem, `from` depois de `to` dá `fields.to = period_order`, e `limit`/`offset` fora da faixa dão `limit_range`/`offset_range`. Data que não é um dia (`2026-02-30`, com horário) ou valor não inteiro dão `400 invalid_request`.

**Itens do extrato.** Cada entrada da listagem tem `kind` (`"transaction"`, `"card_purchase"`, `"occurrence"` ou `"invoice_payment"`, ver Cartões de crédito, Transações fixas e Pagamento de fatura), e o use case devolve um enum `StatementItem`: cada tipo tem a sua tabela e o seu formato, sem forçar a tabela `transactions` a carregar campos de cartão. O filtro por cartão (`credit_card_id`) está em `TransactionFilter`/`ListParams`; deixa as Ocorrências de fora e mantém as compras e os Pagamentos de fatura do cartão. A tabela `transactions` referencia `(category_id, type)`, então o banco também recusa uma Categoria do outro tipo.

## Saldo atual e Saldo inicial (UC10, parcial, e UC22)

Rotas protegidas; cada usuário só enxerga o próprio saldo. Valores em centavos inteiros.

| Rota | Efeito |
| --- | --- |
| `GET /balance` | Devolve `{current_balance_cents, opening_balance_set, opening_balance_cents}`. `opening_balance_cents` é `null` enquanto o Saldo inicial não foi informado. |
| `POST /balance/opening` `{amount_cents}` | Informa o Saldo inicial, uma única vez. `201` com o mesmo corpo do `GET /balance`. Zero e negativo valem (mesmo zero conta como informado); `amount_cents` fora de ±100.000.000.000 dá `422` (`fields.amount_cents = amount_range`) e não gasta a única chance. Uma segunda tentativa dá `409 opening_balance_already_set` e o valor guardado não muda. |

**Regra.** Saldo atual = Saldo inicial (0 se não informado) + Receitas - Despesas - Pagamentos de fatura com data até hoje (America/Sao_Paulo, pelo relógio injetável). Projeções (data depois de hoje) ficam de fora até o dia chegar, sem nenhum job: a cada leitura o saldo é calculado de novo (ADR-0002). O saldo pode ficar negativo e nada bloqueia um lançamento por isso. O Saldo inicial fica na coluna `users.opening_balance_cents` (`NULL` = não informado); gravá-lo é um único `UPDATE ... WHERE opening_balance_cents IS NULL`, então dois pedidos simultâneos não passam os dois.

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

Uma Fatura é `{credit_card_id, closing_month, closing_date, due_date, reference_month, state, total_cents, paid_cents, remaining_cents}`, com `state` igual a `open`, `closed` ou `paid`. Uma compra é `{id, amount_cents, date, description, category, credit_card: {id, name, active}, invoice: {closing_month, closing_date, due_date, reference_month, state}, reference_month, is_projection, created_at}`. `reference_month` (`AAAA-MM`) é o Mês de referência: o mês do vencimento da Fatura.

**Regras.**

- Compra: valor entre 1 e 100.000.000.000, descrição de 1 a 200 caracteres, Categoria ativa de Despesa (`category_*`), cartão do usuário e ativo (`fields.credit_card_id = credit_card_not_found | credit_card_inactive`).
- Fechamento: a compra feita até o Dia de fechamento (inclusive) entra na Fatura que fecha no mês; depois dele, na do mês seguinte. Em mês mais curto vale o último dia (fechamento 31 em fevereiro fecha dia 28).
- Vencimento: o primeiro dia depois do fechamento que cai no Dia de vencimento (vencimento depois do fechamento: no mesmo mês; senão, no mês seguinte).
- Estado derivado das datas, nunca gravado: `open` até o fechamento (inclusive), `closed` depois dele até o vencimento, `paid` a partir do vencimento. O `remaining_cents` é o total menos os Pagamentos antecipados (nunca negativo) enquanto a Fatura não está paga, e zero depois: o Pagamento de fatura automático do vencimento quita o resto (ver "Pagamento de fatura"). Uma Fatura aberta ou fechada paga por inteiro antes do vencimento tem `remaining_cents = 0` mas continua `open`/`closed` até o vencimento, para que novas compras só aumentem o restante.
- Compra de Fatura `paid` não pode ser editada nem excluída (`409 invoice_paid`), e uma data que a colocaria numa Fatura paga é recusada (`422`, `fields.date = invoice_paid`), tanto ao lançar quanto ao editar.
- A Compra no cartão não entra no Saldo atual (`GET /balance` não muda).
- Os dias do cartão têm histórico (`credit_card_schedules`): uma mudança começa na primeira Fatura ainda aberta, ou na seguinte se o novo Dia de fechamento já tiver passado para ela (assim nenhuma compra muda de Fatura). Faturas fechadas e pagas mantêm as datas que tinham.

**Módulo de cálculo.** `src/ledger/cards.rs` (puro, como o resto do `ledger`): `LedgerFacts.cards` leva, por cartão, o histórico de dias (`CardSchedule`) e os gastos por dia (`CardSpend`). `Ledger::invoices(card, hoje)`, `Ledger::invoice(card, mês, hoje)`, `Ledger::invoice_for_purchase(card, data, hoje)` e `Ledger::has_remaining_invoice(card, hoje)` respondem sobre as Faturas; o Mês de referência é `InvoiceCycle::reference_month`, calculado num único lugar (`CardSchedule::reference_month`, o mês do vencimento), que Orçamentos e relatórios também usam. Toda conta de mês (`Month`: `plus`, `prev`, `next`, `last_day`, `parse` e o texto `AAAA-MM`) fica em `src/ledger/month.rs`. A Compra no cartão fica de fora de `cash_flows`, então `current_balance` continua ignorando-a. O extrato (`StatementRepository`) ordena e pagina por `UNION ALL` das tabelas `transactions` e `card_purchases`; uma nova variante de `StatementItem` entra com mais um ramo da consulta. As Ocorrências de Transações fixas são calculadas no use case e juntadas a essa lista em memória; com `credit_card_id` elas ficam de fora.

## Compras parceladas e Parcelas (UC18 e a parte parcelada de UC06, UC07 e UC08)

Uma Compra parcelada é uma Compra no cartão dividida em Parcelas. O usuário informa o **total já com juros** (o sistema nunca calcula nem grava juros) e o número de Parcelas (2 a 48). Não altera o Saldo atual.

| Rota | Efeito |
| --- | --- |
| `POST /installment-purchases` `{credit_card_id, total_cents, installment_count, date, description, category_id}` | Lança. `201 {installment_purchase}`. `fields.installment_count = installment_count_range`; `fields.total_cents = amount_range` ou `total_below_installments` (menos de 1 centavo por Parcela); `fields.date = invoice_paid` se a primeira Parcela cairia numa Fatura paga; `category_*` e `credit_card_*` como nas compras no cartão. |
| `PATCH /installment-purchases/{id}` `{total_cents?, installment_count?, date?, description?, category_id?}` | Sem Parcela em Fatura paga: recalcula e realoca tudo (as Parcelas de mesmo número continuam sendo o mesmo registro). Com alguma paga: só `category_id`; qualquer outro campo dá `409 installments_paid`. |
| `DELETE /installment-purchases/{id}` | Sem Parcela paga: exclui a compra inteira, `204`. Com alguma paga: cancela só as restantes e devolve `200 {installment_purchase}` (`cancelled: true`, só as pagas em `installments`). Se todas estão pagas, `409 invoice_paid`. `404 installment_purchase_not_found` para id desconhecido ou de outro usuário. |
| `GET /credit-cards/{id}/invoices/{closing_month}` | Ganha `installments`: as Parcelas da Fatura (`{id, installment_purchase_id, number, installment_count, amount_cents, total_cents, date, description, category, credit_card, invoice, reference_month, purchase_has_paid_installments}`), as linhas "3/10". O total da Fatura soma compras e Parcelas. |
| `GET /transactions` | O extrato ganha itens `kind: "installment_purchase"`, **um por compra** (pela data da compra), com todas as Parcelas; respeitam os filtros de período, Categoria, tipo e cartão. |

Uma compra parcelada é `{id, total_cents, installment_count, date, description, category, credit_card, cancelled, has_paid_installments, installments: [{id, number, amount_cents, invoice: {closing_month, closing_date, due_date, reference_month, state}, reference_month}], is_projection, created_at}`.

**Regras.**

- O total se divide em centavos exatos e o que sobra vai para a **primeira** Parcela (R$ 100,00 em 3x: 33,34 + 33,33 + 33,33).
- Cada Parcela vai para uma Fatura consecutiva, a primeira na que receberia a compra à vista (regra do fechamento). Cada Parcela conta no Mês de referência da sua Fatura.
- "Parcela paga" é Parcela numa Fatura `paid` (derivado do vencimento, como nas compras no cartão). Uma Fatura só com Parcelas aparece na lista de Faturas e conta para `credit_card_has_remaining_invoice`.
- Mudar os dias do cartão realoca as Parcelas de compras sem Parcela paga, como já acontece com as compras avulsas.

**Banco e módulo de cálculo.** `installment_purchases` guarda a compra (total, número de Parcelas, data, Categoria, `cancelled_at`) e `installments` guarda **uma linha por Parcela** com valor e `invoice_month` (primeiro dia do mês de fechamento da Fatura): assim a Antecipação de parcelas (ticket seguinte) pode mover uma Parcela de Fatura sem tocar nas outras. As Faturas continuam sem ser gravadas. No `ledger`, `CardFacts.installments` leva as `InstallmentCharge` (mês da Fatura e valor, lidas por `CreditCardRepository::installment_charges`), `Ledger::plan_installments` divide e aloca uma compra (`split_installments`, `PlannedInstallment`) e `Ledger::invoice_cycle` dá datas e estado de qualquer Fatura. O extrato ganha mais um ramo no `UNION ALL` de `StatementRepository`.

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

**Regras.** O histórico é o Saldo atual que a pessoa tinha no fim de cada dia ou mês: Saldo inicial mais Receitas e Despesas fora do cartão com data até ali, incluindo as Ocorrências que já tinham caído. O Saldo projetado em uma data é o Saldo atual mais Receitas e Despesas fora do cartão com data depois de hoje até ela, com Projeções e Ocorrências futuras (inclusive edições e exclusões de uma só Ocorrência). Compras no cartão ainda não lançadas não entram. As Faturas entram (ver "Pagamento de fatura"): o histórico desconta cada Pagamento de fatura no dia dele, e o Saldo projetado, em cada ponto, desconta o valor restante de toda Fatura que vence até a data (o Pagamento automático do vencimento) e os Pagamentos antecipados já feitos.

**Horizonte e erros.** O horizonte é o fim do mês 12 meses depois do mês de hoje (`ledger::projection_horizon`, o mesmo da listagem). Erros `422 validation_failed` com `fields`: `to = history_in_future` (histórico além de hoje), `to = projection_in_past` (projeção antes de hoje), `to = beyond_horizon` (projeção além do horizonte), `to = period_order` (`from` depois de `to`), `from = period_too_long` (mais de 1.000 pontos). Data inválida ou `granularity` desconhecida dão `400 invalid_request`.

**Módulo de cálculo.** `src/ledger/series.rs` acrescenta `Ledger::balance_history(from, to, granularity, today)` e `Ledger::projected_balance(to, granularity, today)`, ambos varrendo uma só lista ordenada de fluxos e Ocorrências (sem consulta por ponto). Os testes ficam em `tests/balance_series.rs`, pela API, movendo o "hoje" com `set_today`.

## Pagamento de fatura: antecipado e automático (UC17, ADR-0003)

Rotas protegidas; cada usuário só enxerga os próprios pagamentos. Valores em centavos inteiros.

| Rota | Efeito |
| --- | --- |
| `POST /credit-cards/{id}/invoices/{closing_month}/payments` `{amount_cents}` | Pagamento antecipado, total ou parcial, de uma Fatura aberta ou fechada. `201 {payment, invoice}`: o pagamento (`{id, origin: "early", amount_cents, date, is_projection}`) e a Fatura depois dele. A data do pagamento é sempre hoje (não há campo de data: não existe pagamento futuro). |
| `GET /credit-cards/{id}/invoices/{closing_month}` | Ganhou `payments`: os Pagamentos antecipados e, se algo ficar sem pagar, o automático do vencimento (`origin: "automatic"`, `id: null`), do mais antigo para o mais novo. |
| `GET /transactions` | Ganhou itens `kind: "invoice_payment"`: `{id, origin, amount_cents, date, is_projection, credit_card: {id, name, active}, invoice: {closing_month, closing_date, due_date, reference_month, state}, created_at}` (`id` e `created_at` são `null` no automático). |

**Regras.**

- Só Fatura `open` ou `closed`; uma `paid` dá `409 invoice_paid` (o mesmo código de editar compra de Fatura paga) e uma Fatura inexistente, `404 invoice_not_found`. `amount_cents` precisa estar entre 1 e 100.000.000.000 (`422`, `fields.amount_cents = amount_range`) e não passar do `remaining_cents` da Fatura (`422`, `fields.amount_cents = amount_exceeds_remaining`). A soma dos pagamentos antecipados de uma Fatura é verificada dentro de uma transação que trava o cartão, então dois pedidos simultâneos não passam juntos do total.
- Compras lançadas depois de um pagamento antecipado só aumentam o `remaining_cents` (o total sobe, o pago não muda).
- **A Fatura nunca fica com total menor do que os pagamentos antecipados já feitos nela** (senão o excedente sumiria do `remaining_cents`, que não é negativo). Toda operação que baixaria o total é recusada com `409 invoice_would_be_overpaid`, sem mudar nada: editar ou excluir uma compra no cartão (inclusive mudar a data para outra Fatura), editar, cancelar ou excluir uma compra parcelada, editar ou excluir um Encargo, mudar os dias do cartão (que realoca compras com data futura e Parcelas), antecipar Parcelas (a Fatura de origem perde a Parcela) e corrigir o pagamento de uma Fatura (o Saldo rolado da seguinte diminui). Não existe rota para desfazer um pagamento antecipado, então quem pagou uma Fatura por inteiro só consegue aumentá-la. A conta é feita pelo ledger antes de gravar: `CardFacts::after(&CardChange)` aplica a mudança aos fatos em memória, e `Ledger::overpaid_by` diz se alguma Fatura passaria a ter pago mais que o total (`CardFacts::overpaid_invoices`).
- **No Dia de vencimento** o ledger deriva o Pagamento automático: o que ainda não foi pago (`total - antecipados`, se positivo) sai do Saldo atual na data do vencimento e a Fatura passa a `paid` (`remaining_cents = 0`, `paid_cents = antecipados + automático`). Nada é gravado e nenhum job roda (ADR-0002): avançar o relógio basta. Hoje a Fatura é `paid` só a partir do vencimento; uma Fatura fechada paga por inteiro antes continua `closed` com restante zero até lá.
- O Pagamento de fatura reduz o Saldo atual, o histórico e o Saldo projetado, mas **não é Despesa**: fica fora de qualquer filtro de `type` ou `category_id` (o extrato com esses filtros não o traz), e a Compra no cartão continua sendo a única Despesa (relatórios, ticket futuro, contam as compras no Mês de referência e ignoram os pagamentos).
- Extrato: os Pagamentos antecipados vêm da tabela `invoice_payments` (um ramo a mais do `UNION ALL` do `StatementRepository`) e os automáticos são calculados na leitura e juntados em memória, como as Ocorrências, dentro de `from`/`to` e até o horizonte de projeção. Um automático com data depois de hoje tem `is_projection: true`. O filtro `credit_card_id` vale para ambos.
- **Saldo projetado:** em cada ponto, desconta o valor restante de toda Fatura com vencimento até a data (o automático do vencimento) e os antecipados já feitos (que já estão no Saldo atual). Compras ainda não lançadas continuam de fora.

**Módulo de cálculo.** `CardFacts` ganhou `payments: Vec<PaymentFact>` (os antecipados gravados). `Invoice` ganhou `paid_cents`. `CardFacts::payments()` devolve, por Fatura, os antecipados e o automático do vencimento como `InvoicePayment { card_id, closing_month, date, amount_cents, origin: Early(id) | Automatic }`, sem depender de "hoje". `Ledger::invoice_payments(from, to)` e `Ledger::payments_of_invoice(card, mês)` os expõem; `Ledger::current_balance` e `Ledger::balances_at` (base do histórico e do Saldo projetado) os somam como saídas datadas. Os testes ficam em `tests/invoice_payments.rs`, pela API, movendo o "hoje" com `set_today`. A correção do Pagamento de fatura (Saldo rolado) e os Encargos estão na seção seguinte.

## Pagamento corrigido, Saldo rolado e Encargos (UC20 e UC21, ADR-0002 e ADR-0003)

Quando a pessoa não pagou tudo no banco, ela corrige o Pagamento de fatura e registra os juros cobrados. Rotas protegidas; cada usuário só enxerga o que é seu. Valores em centavos inteiros.

| Rota | Efeito |
| --- | --- |
| `PUT /credit-cards/{id}/invoices/{closing_month}/payment` `{amount_cents}` | Corrige o pagamento do vencimento para o valor realmente pago (de `0` até o que os pagamentos antecipados deixaram sem pagar, `payment_correction.max_cents`). `200 {payment, invoice}`, com `payment.origin = "corrected"`. Pode ser repetida. |
| `POST /credit-cards/{id}/invoices/{closing_month}/charges` `{amount_cents, date, description?}` | Lança Encargos (juros, multa, IOF) numa Fatura. `201 {charge, invoice}`. A `description` é opcional (padrão "Juros e encargos"). |
| `PATCH /charges/{id}` `{amount_cents?, date?, description?}` | Edita um Encargo. `200 {charge, invoice}`. |
| `DELETE /charges/{id}` | Exclui. `204`. |
| `GET /credit-cards/{id}/invoices/{closing_month}` | Ganhou `charges`, `carried_balance` (`{amount_cents, from_closing_month}` ou `null`) e, em `invoice`, `charges_cents`, `carried_balance_cents`, `carried_forward_cents` e `payment_correction: {state, corrected_cents, max_cents}`. |
| `GET /transactions` | Ganhou itens `kind: "charge"` (`{id, amount_cents, date, description, category, credit_card, invoice, reference_month, is_projection}`): despesa da Categoria "Juros e encargos", presente nos filtros `type=expense`, `category_id` e `credit_card_id`. O Saldo rolado não aparece no extrato. |

**Pagamento corrigido.** O pagamento automático do vencimento é derivado na leitura (ADR-0002); ao corrigi-lo, a pessoa grava uma sobreposição explícita (`invoice_payment_corrections`, uma por Fatura, migration `0012`). A sobreposição vira o valor que sai do Saldo atual no Dia de vencimento, e a Fatura continua Paga (o estado vem das datas). Só existe pagamento a corrigir depois do vencimento. Erros `409`: `invoice_payment_not_due` (antes do vencimento), `invoice_nothing_to_pay` (pagamentos antecipados cobriram tudo) e `invoice_payment_locked`; valor acima do máximo dá `422` com `fields.amount_cents = amount_exceeds_remaining`, e negativo, `amount_range`.

**Saldo rolado.** A diferença não paga (o que o pagamento antecipado deixou menos o valor corrigido) não é gravada: é o item "Saldo rolado" da Fatura seguinte do mesmo cartão (`carried_balance_cents`), somado ao total e, portanto, ao valor restante e ao Pagamento automático dela. Não é Despesa (não entra em relatórios nem em Orçamentos) e não muda o Saldo atual por si; só o pagamento da Fatura seguinte o muda. A cadeia vale: a Fatura seguinte pode, por sua vez, passar o que não pagou adiante. Um cartão com Saldo rolado tem Fatura com valor restante e não pode ser desativado.

**Trava.** O pagamento continua editável (`payment_correction.state = "editable"`) até a Fatura que recebeu o Saldo rolado (a seguinte) ficar Paga, o que se deriva da data de vencimento dela; depois fica `locked` (`409 invoice_payment_locked`), como o resto da Fatura. A regra vale também para uma Fatura nunca corrigida.

**Encargos.** São uma Despesa da Categoria "Juros e encargos" (a marcada por `is_charges` do usuário, não escolhida por ele), ligada à Fatura pelo cartão e pelo mês de fechamento (não pela data do Encargo, que só ordena o extrato). Entram no total da Fatura (são pagos junto, pelo pagamento dela) e contam no Mês de referência do vencimento da Fatura. Só podem ser lançados, editados ou excluídos enquanto a Fatura não está Paga (`409 invoice_paid`). Com a Categoria desativada, o lançamento dá `422` com `fields.category_id = category_inactive`. O sistema nunca calcula juros. Enquanto houver Encargos, `CategoryRepository::has_charges` impede desativar "Juros e encargos" (`409 category_has_charges`). Ficam na tabela `invoice_charges` (ela referencia `(category_id, type)` como as compras no cartão).

**Módulo de cálculo.** `CardFacts` ganhou `charges: Vec<ChargeFact>` e `corrections: Vec<CorrectionFact>`. Cada Fatura é "liquidada" (`Settlement`) do mais antigo para o mais novo, sem olhar para "hoje": total = compras + parcelas + Encargos + Saldo rolado recebido; o que sobra do pagamento do vencimento passa à Fatura seguinte. `Invoice` ganhou `charges_cents`, `carried_in_cents`, `carried_out_cents` e `payment_correction` (`CorrectionState`: `NotDue`, `NothingToPay`, `Locked`, `Editable`). `PaymentOrigin` ganhou `Corrected` (e `is_derived()`); `CardFacts::payments()` datada no vencimento já traz o valor corrigido, então `Ledger::current_balance`, o histórico e o Saldo projetado ficam coerentes sem mudança: o Saldo projetado desconta, a cada vencimento, o valor restante (Encargos e Saldo rolado incluídos). O extrato ganhou `StatementItem::Charge`. Os testes ficam em `tests/invoice_corrections.rs` e `tests/invoice_charges.rs`, pela API.

## Antecipação de parcelas com Desconto (UC19)

O usuário traz Parcelas futuras de **um mesmo cartão** para a Fatura aberta (a que hoje recebe compras), com um Desconto opcional. É **irreversível**: não existe rota nem use case que desfaça, e a API só executa com confirmação explícita. Rotas protegidas; valores em centavos.

| Rota | Efeito |
| --- | --- |
| `GET /credit-cards/{id}/anticipable-installments` | O que dá para antecipar agora: `{open_invoice: {closing_month, closing_date, due_date, reference_month, state}, installments: [...]}`. Cada item tem a forma das linhas "3/10" da Fatura (`InvoiceInstallmentResponse`), da Fatura mais próxima para a mais distante. `404 credit_card_not_found`. |
| `POST /credit-cards/{id}/installment-anticipations` `{installment_ids, discount_cents?, confirm}` | Antecipa. `200 {anticipation: {credit_card_id, original_total_cents, discount_cents, total_cents, invoice, installments}}`: `invoice` é a Fatura aberta depois da operação e `installments` as Parcelas já antecipadas. |

**Erros.** `confirm` ausente ou `false`: `422 validation_failed` com `fields.confirm = confirmation_required` (nada muda). `installment_ids` vazio: `fields.installment_ids = installments_required`; id que não existe, é de outro cartão ou de outro usuário: `fields.installment_ids = installment_not_found`. `discount_cents` informado precisa ser maior que zero (`fields.discount_cents = discount_range`; omitido ou `null` é sem desconto) e deixar ao menos 1 centavo em cada Parcela, o que inclui nunca igualar nem passar da soma delas (`fields.discount_cents = discount_exceeds_total`). Parcela fora de Fatura posterior à aberta: `409 installments_not_anticipable`, e nada é movido (tudo ou nada).

**Regras.**

- Só Parcela de Fatura **posterior à aberta** pode ser antecipada. Isso exclui as Faturas aberta, fechadas e pagas e as Parcelas já antecipadas (já estão na Fatura que era a aberta, que não é mais posterior). Ids repetidos contam uma vez.
- Cada Parcela antecipada passa a estar na Fatura aberta (`invoice_month` dela), então conta no **Mês de referência** do vencimento dela. A Fatura aberta, o extrato e o Saldo projetado já refletem isso, porque tudo isso é calculado na leitura (ADR-0002): a operação só altera as Parcelas.
- **Desconto.** Dividido entre as Parcelas em proporção ao valor de cada uma, em centavos exatos (maior resto): cada parte é arredondada para baixo e os centavos que sobram vão, um a um, às Parcelas com a maior fração de centavo; no empate, à da Fatura mais antiga, depois à compra mais antiga e ao menor número. Cada Parcela diminui do seu quinhão (`amount_cents`), e assim a Despesa na Categoria da compra original, que é a soma das suas Parcelas. `total_cents` da compra continua o que o usuário informou; `discount_cents` na compra soma o Desconto recebido. A Parcela guarda `discount_cents`, `original_amount_cents` (valor + desconto), `original_invoice_month` e `anticipated_on`.
- **Parcela antecipada é histórico, como a paga.** Mudar os dias do cartão não a move (as demais Parcelas da compra são realocadas, mantendo o valor). Compra com Parcela antecipada só aceita mudar a Categoria (`409 installments_anticipated` para outros campos do `PATCH`), e o `DELETE` cancela só as Parcelas restantes (nem pagas nem antecipadas) e devolve `200`; sem restante, `409 installments_anticipated`.
- Uma operação concorrente sobre a mesma Parcela não passa duas vezes: o `UPDATE` confere que ela ainda está na Fatura e com o valor que o plano viu.

**Banco e módulo de cálculo.** A migration `0013` acrescenta a `installments` as colunas `anticipated_on`, `original_invoice_month` e `discount_cents` (só o que a operação muda). `ledger::anticipation` é puro: `Ledger::plan_anticipation(card, escolhidas, desconto, hoje)` devolve um `AnticipationPlan` (Fatura aberta e `AnticipatedShare` por Parcela) ou um `AnticipationError`, e `split_discount` faz a divisão; `Ledger::open_invoice_cycle` dá a Fatura aberta. `AnticipateInstallmentsUseCase` e `ListAnticipableInstallmentsUseCase` orquestram; `InstallmentPurchaseRepository::anticipate` grava tudo ou nada. Os testes ficam em `tests/installment_anticipations.rs`, pela API.

## Relatórios (UC11)

Dois relatórios por **Mês de referência**; o terceiro, o histórico do Saldo atual, já é `GET /balance/history` (veja acima) e a tela de relatórios o reutiliza. Rotas protegidas; valores em centavos; períodos em **meses inteiros** `YYYY-MM`, de no máximo 36 meses.

| Rota | Efeito |
| --- | --- |
| `GET /reports/expenses-by-category?from=&to=` | Despesas por Categoria no período. Padrão: o mês atual (`to` = mês atual, `from` = `to`). `200 {from, to, total_cents, projected_cents, categories: [{category_id, name, active, total_cents, projected_cents}]}`, da Categoria que mais gastou para a que menos gastou (empate: por nome); Categorias sem despesa no período não aparecem. |
| `GET /reports/income-vs-expenses?from=&to=` | Receitas e Despesas de cada mês do período, em série. Padrão: os últimos 6 meses, este incluído. `200 {from, to, income_cents, expense_cents, months: [{month, income_cents, expense_cents, balance_cents, projected_income_cents, projected_expense_cents}]}`, do mês mais antigo ao mais novo, com uma linha para cada mês, mesmo vazio. `balance_cents` é Receitas menos Despesas e pode ser negativo. |

**Erros.** Mês fora do formato `YYYY-MM`: `422 validation_failed` com `fields.from` ou `fields.to = invalid_month`. Mês inicial depois do final: `fields.to = period_order`. Mais de 36 meses: `fields.from = period_too_long`.

**O que conta.** Cada Receita ou Despesa conta no seu Mês de referência (GLOSSARY):

- Receita ou Despesa fora do cartão: o mês da própria data.
- Ocorrência de Transação fixa: o mês em que cai, **Projeções do período incluídas** (a Ocorrência futura de uma Receita fixa conta nas Receitas).
- Compra no cartão: o mês do vencimento da Fatura em que entra. Parcela e Encargo: o mês do vencimento da Fatura em que estão. Uma Parcela antecipada está na Fatura que era a aberta, com o valor já reduzido pelo Desconto; os Encargos entram na Categoria "Juros e encargos".
- **Ficam de fora**: Pagamentos de fatura (antecipados, automáticos e corrigidos) e o Saldo rolado. Não são Despesas: o gasto já contou quando foi feito.

`projected_cents` (e `projected_income_cents`/`projected_expense_cents`) é a parte do total que é Projeção: Transação ou Ocorrência com data depois de hoje, compra no cartão e Encargo com data depois de hoje e Parcela de uma compra feita depois de hoje. Parcelas de compras já feitas, ainda que em Faturas futuras, não são Projeções.

**Módulo de cálculo.** As regras de "o que conta e em que mês" são as dos Orçamentos e ficam num lugar só: `Ledger::expense_entries` (`ledger::budget`), alimentado por `SpendingRepository` (`Ledger::with_budget_facts`), devolve cada Despesa com Categoria, Mês de referência e data; `Ledger::category_spending` (Orçamentos) e os relatórios somam essas entradas. `ledger::reports` acrescenta a Receita (as Transações de entrada de `LedgerFacts` e as Ocorrências de Receitas fixas), a marca de Projeção e a validação do período: `Ledger::expenses_by_category` e `Ledger::income_vs_expenses`. `GetExpensesByCategoryUseCase` e `GetIncomeVsExpensesUseCase` orquestram. Os testes ficam em `tests/reports.rs`, pela API.
## Orçamentos e Alerta de orçamento (UC12 e UC13)

Um Orçamento é o limite mensal de uma Categoria de Despesa e vale todo mês até o usuário alterá-lo. Rotas protegidas; cada usuário só enxerga os próprios Orçamentos (uma Categoria de outro usuário responde como inexistente). Valores em centavos inteiros.

| Rota | Efeito |
| --- | --- |
| `PUT /budgets/{category_id}` `{limit_cents}` | Define ou altera o limite da Categoria. `200 {budget}` com `budget = {category: {id, name, type: "expense", active}, limit_cents, effective_from}` (`effective_from` é o mês `AAAA-MM` em que o limite começou). `limit_cents` de 1 a 100.000.000.000 (`422`, `fields.limit_cents = amount_range`); a Categoria precisa ser do usuário, de Despesa e ativa (`fields.category_id = category_not_found \| category_type_mismatch \| category_inactive`). |
| `GET /budgets` | `{budgets}`: os Orçamentos em vigor no mês de hoje, por nome de Categoria, inclusive os de Categorias desativadas (`category.active = false`). |
| `DELETE /budgets/{category_id}` | Remove o Orçamento a partir do mês de hoje. `204`; `404 budget_not_found` se a Categoria não tem Orçamento em vigor. Funciona também para Categoria desativada. |
| `GET /budgets/status?month=AAAA-MM` | O gasto de cada Categoria que tem Orçamento no Mês de referência (padrão: o mês de hoje) contra o limite. `{month, exceeded_count, budgets: [{category, limit_cents, spent_cents, remaining_cents, percent_used, exceeded}]}`. `month` inválido dá `400 invalid_request`. |

**Meses passados.** O limite é gravado como versões que começam em um mês (`budget_limits`, como as versões de valor das Transações fixas): o limite de um mês é a versão mais recente que começou até ele. Definir ou alterar vale **a partir do mês de hoje** e não mexe nos meses anteriores, que continuam com o limite que tinham; mudar de novo no mesmo mês substitui a versão do mês. Remover grava o fim do Orçamento a partir do mês de hoje (uma versão sem limite). Um mês anterior à primeira vez que o Orçamento foi definido não tem Orçamento (a Categoria não aparece em `/budgets/status` daquele mês), e um mês futuro usa o limite em vigor, sem precisar defini-lo de novo.

**Gasto da Categoria.** Soma das Despesas cujo Mês de referência é o mês consultado, na Categoria: Despesas fora do cartão (pela própria data), Ocorrências de Gastos fixos (as Projeções do mês entram, e valem as edições e exclusões de uma só Ocorrência), Compras no cartão e Parcelas (pelo mês do vencimento da Fatura em que estão). O Saldo rolado e os Pagamentos de fatura **nunca** contam: o gasto já foi contado quando foi feito. Receitas também não contam. Os Encargos contam em "Juros e encargos" pelo mês de vencimento da Fatura em que foram lançados (a data do Encargo não importa), e as Parcelas antecipadas contam no mês de vencimento da Fatura aberta para a qual foram, com o valor já descontado: o `SpendingRepository` lê Parcelas e Encargos como `InvoiceExpense`, por cartão, Categoria e mês de fechamento da Fatura. Não é gravado nada: como as Faturas e o Saldo, o gasto é recalculado a cada leitura (ADR-0002).

**Alerta.** `exceeded` é verdadeiro quando `spent_cents > limit_cents` (exatamente no limite não é Alerta) e a Categoria está ativa. O Alerta só existe dentro do sistema: não há e-mail nem push. `percent_used` é o gasto em porcentagem inteira do limite, arredondada para baixo, e passa de 100 quando o limite é excedido; `remaining_cents` fica negativo nesse caso. O Orçamento de uma Categoria desativada continua guardado e aparece em `/budgets/status` com `category.active = false` e `exceeded = false`, mesmo com o gasto acima do limite; reativar a Categoria faz o Orçamento voltar a valer com o mesmo limite. Definir um Orçamento exige Categoria ativa.

**Módulo de cálculo.** `src/ledger/budget.rs` (puro): `BudgetFacts` (`SpendingFacts` com as Despesas por Categoria e os `Budget`s com o histórico de limites) entra no `Ledger` por `Ledger::with_budget_facts`, sem mexer em `LedgerFacts`. `Ledger::category_spending(mês)` e `Ledger::budget_status(mês)` respondem; o Mês de referência de uma compra ou Parcela vem do vencimento da Fatura que o `CardSchedule` do cartão calcula. Os repositórios são `BudgetRepository` (versões de limite) e `SpendingRepository` (Despesas somadas por Categoria e dia ou Fatura). Os testes ficam em `tests/budgets.rs`, pela API, movendo o "hoje" com `set_today`, e em testes unitários de `ledger/budget.rs`.

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

Outros scripts, com o mesmo princípio (Docker, banco descartável removido ao terminar):

- `backend/scripts/fmt.sh`: aplica o `cargo fmt` (o `check.sh` só verifica o formato).
- `backend/scripts/dev-api.sh [porta]`: sobe a API (`cargo run`) com um PostgreSQL descartável, em `127.0.0.1:<porta>` (padrão 8080), para usar o front-end sem Rust nativo. `APP_ENV=development` e `COOKIE_SECURE=false`; os dados somem ao encerrar.
- `backend/scripts/load-check.sh`: teste de carga do requisito de desempenho (veja abaixo).

## Teste de carga (desempenho)

```bash
backend/scripts/load-check.sh                              # usuário realista, limite de 300 ms
LOAD_CHECK_TRANSACTIONS=10000 backend/scripts/load-check.sh # usuário maior
LOAD_CHECK_P95_MS=200 backend/scripts/load-check.sh         # limite mais rígido
```

O teste `tests/load_check.rs` (marcado `#[ignore]`, por isso fora do `cargo test` e do CI) cria um usuário com 2.000 Transações em dois anos, 6 Transações fixas, 4 cartões com 432 compras e 72 compras parceladas (montadas mês a mês, movendo o "hoje", como uma pessoa faria) e 6 Orçamentos. Depois chama os endpoints principais (extrato, saldo, histórico e projeção, cartões e faturas, orçamentos, relatórios e login) e imprime p50, p95 e máximo por endpoint, primeiro com um cliente e depois com 8 clientes ao mesmo tempo, sem pausa entre as requisições. Falha se algum p95 passar do limite. O script usa build de release, a API e o PostgreSQL numa rede Docker própria (banco ao lado da API, como numa hospedagem na mesma região) e chama a API por HTTP local: não inclui TLS nem o proxy do Next.js. Os números medidos estão em [`docs/nao-funcionais.md`](../docs/nao-funcionais.md).

## CI

O workflow `.github/workflows/ci.yml` roda a cada PR e a cada push na `main`.
O merge na `main` deve exigir o check `backend`.
