# Requisitos não funcionais: verificação

Registro de como cada requisito não funcional do termo de abertura (Anexo I, seção 3) foi verificado no MVP, com os números medidos e o que ainda não foi possível verificar. Só constam resultados de medições e verificações feitas de fato; o que não foi medido está dito como tal.

Ambiente das medições: Windows 11 com Docker Desktop (16 CPUs, 7,6 GiB), PostgreSQL 17, API compilada em release, navegador do painel do aplicativo (Chromium) para as verificações de interface.

| Requisito | Situação |
| --- | --- |
| Desempenho | Atendido nas medições (maior p95 de um endpoint: 84 ms; p95 de todas as leituras: 51 ms com 8 clientes) |
| Segurança | Verificada: Argon2id, JWT em cookie `httpOnly`, CSRF, XSS, SQL injection e HTTPS (HSTS) |
| Usabilidade (responsiva) | Verificada em 375 px e em desktop, em Chromium |
| Portabilidade (navegadores) | Parcial: só Chromium (base do Chrome e do Edge); Firefox e Safari não testados |
| Disponibilidade | Não verificável ainda: depende da implantação (ticket #6) |
| Manutenibilidade | Atendida: TypeScript e Rust, testes das regras críticas no CI |
| Escalabilidade | Arquitetura em camadas mantida; sem teste além do usuário único |
| Acessibilidade básica | Verificada (axe em testes e no navegador, teclado e foco) |

## 1. Desempenho: 95% das requisições em até 300 ms

**Como medir.** `backend/scripts/load-check.sh` (teste `backend/tests/load_check.rs`, fora do `cargo test` e do CI). Cria um usuário realista pela própria API: 2.000 Transações em dois anos (Receitas e Despesas, com datas futuras como Projeções), 6 Transações fixas, 4 cartões com 432 compras e 72 compras parceladas (lançadas mês a mês, movendo o "hoje", como uma pessoa faria) e 6 Orçamentos. Depois chama os endpoints principais, primeiro com 1 cliente (100 requisições por endpoint) e depois com 8 clientes simultâneos sem pausa (240 por endpoint). A API e o PostgreSQL ficam numa rede Docker própria; a chamada é HTTP local, sem TLS e sem o proxy do Next.js. Falha se algum p95 passar de 300 ms.

**Resultado final** (`load-check.sh`, 2.000 Transações), em milissegundos:

| Endpoint | 1 cliente p50 | 1 cliente p95 | 8 clientes p50 | 8 clientes p95 |
| --- | ---: | ---: | ---: | ---: |
| `GET /auth/me` | 1,1 | 1,7 | 1,4 | 4,9 |
| `GET /categories` | 1,3 | 2,1 | 1,5 | 3,6 |
| `GET /transactions` (página de 50) | 21,9 | 31,3 | 32,4 | 43,0 |
| `GET /transactions` (um mês) | 15,2 | 27,2 | 29,0 | 40,2 |
| `GET /transactions` (página funda, offset 1500) | 38,1 | 55,8 | 59,2 | 73,9 |
| `GET /transactions` (por cartão) | 18,5 | 26,7 | 39,3 | 53,0 |
| `GET /balance` | 5,3 | 7,5 | 12,8 | 17,9 |
| `GET /balance/history` (24 meses) | 4,7 | 7,1 | 13,7 | 18,5 |
| `GET /balance/history` (365 dias) | 5,9 | 8,6 | 13,4 | 17,7 |
| `GET /balance/projection` (12 meses) | 5,6 | 7,8 | 14,1 | 18,2 |
| `GET /balance/projection` (diária) | 5,8 | 8,7 | 13,7 | 18,5 |
| `GET /credit-cards` | 0,8 | 2,0 | 6,5 | 12,0 |
| `GET /credit-cards/{id}/invoices` | 2,6 | 4,4 | 4,4 | 8,2 |
| `GET /credit-cards/{id}/invoices/{mês}` | 7,6 | 11,2 | 17,5 | 27,3 |
| `GET /credit-cards/{id}/anticipable-installments` | 5,0 | 9,7 | 10,5 | 16,3 |
| `GET /recurring-transactions` | 1,1 | 2,3 | 5,2 | 10,1 |
| `GET /budgets/status` | 11,6 | 19,5 | 21,5 | 30,4 |
| `GET /reports/expenses-by-category` | 12,7 | 22,2 | 25,5 | 34,6 |
| `GET /reports/income-vs-expenses` | 13,6 | 21,6 | 21,7 | 31,1 |
| `POST /auth/login` (Argon2) | 25,2 | 41,2 | 45,4 | 84,3 |

Todas as leituras juntas: 1 cliente, p50 6,3 ms, p95 31,0 ms, p99 44,5 ms (1.900 requisições); 8 clientes, p50 14,8 ms, p95 50,8 ms, p99 67,2 ms (4.560 requisições). Nenhum endpoint passou de 85 ms no p95.

**Usuário maior.** Com `LOAD_CHECK_TRANSACTIONS=10000` (5 vezes mais Transações), medido antes da carga em paralelo descrita abaixo: leituras com 1 cliente p95 de 44,0 ms; com 8 clientes, p95 de 83,8 ms e p99 de 99,7 ms; o maior p95 de um endpoint foi 105,8 ms (extrato, página funda).

**O que a medição mostrou e o que foi corrigido.**

- Na primeira medição, o PostgreSQL era alcançado pela porta publicada do Docker Desktop (cada consulta custava 2 a 3 ms a mais). Nessa situação, com 1 cliente todas as leituras ficaram abaixo de 185 ms no p95 (todas juntas: p95 135,3 ms), mas com 8 clientes simultâneos o p95 de todas as leituras foi 300,4 ms e o da página funda do extrato chegou a 367,3 ms: o custo estava no número de consultas feitas em sequência (de 10 a 30 por requisição, cada uma esperando a anterior), não no volume de dados.
- Correção: `card_facts` (7 leituras do cartão) e os use cases de saldo, histórico e projeção passaram a executar as consultas independentes ao mesmo tempo (`tokio::try_join!`). Na mesma situação de banco distante, o saldo foi de p50 55,6 ms para 15,0 ms e todas as leituras com 1 cliente foram de p95 135,3 ms para 64,9 ms; com 8 clientes, de 300,4 ms para 238,8 ms.
- O script de carga passou a colocar a API e o banco na mesma rede Docker (o caso de uma hospedagem na mesma região), que é a tabela acima.
- Não foi preciso criar índice novo (a migration `0015` não existe): os índices por usuário e data das migrations anteriores bastam nessas cargas.

**Limites da medição.** É um usuário só (os dados de todos os usuários ficam na mesma tabela, filtrados por `user_id`); a rede até o navegador, o TLS e o proxy do Next.js não entram; o banco e a API estavam na mesma máquina. Em produção, a latência entre a API e o PostgreSQL gerenciado pesa mais do que o volume de dados: manter os dois na mesma região é importante para o requisito.

## 2. Segurança

| Item | Como foi verificado |
| --- | --- |
| Senhas com Argon2 | `backend/src/auth/password.rs` usa Argon2id com sal aleatório; teste unitário confere o prefixo `$argon2id$`, que duas senhas iguais geram hashes diferentes e que um hash malformado nunca confere. O login com e-mail desconhecido verifica contra um hash isca, para levar o mesmo tempo. |
| JWT em cookie `httpOnly` | Cookie `tabwallet_session`: `HttpOnly`, `SameSite=Lax`, `Path=/`, 7 dias, `Secure` por padrão (só `COOKIE_SECURE=false` o desliga). Teste `signup_creates_the_account...` confere `HttpOnly` e `SameSite=Lax`. O front-end nunca vê o token. |
| CSRF | Três camadas: `SameSite=Lax`; sem CORS e proxy de mesma origem (`/api/*`); e a verificação de `Origin`/`Referer` da API (`backend/src/csrf.rs`), nova neste ticket: requisição que altera dados e declara outra origem recebe `403 forbidden_origin`. Testes em `backend/tests/csrf.rs` (origem alheia, `null`, domínio parecido, porta e esquema diferentes, `Referer`, login e logout, GET livre) e unitários. Verificado também pelo navegador: o login pelo Next.js (que repassa o `Origin`) funciona e um `curl` com `Origin: https://evil.example` em `/api/auth/logout` recebeu 403. |
| XSS | Nenhum HTML de usuário é renderizado: a única ocorrência de `dangerouslySetInnerHTML` é o estilo de cores do componente de gráfico do shadcn, alimentado só por configuração estática do código. O e-mail de recuperação é texto puro. Cabeçalhos: `Content-Security-Policy` (`default-src 'self'`, `object-src 'none'`, `base-uri`, `form-action` e `connect-src` restritos, `frame-ancestors 'none'`), `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`, `Referrer-Policy` e `Permissions-Policy` (testes em `frontend/lib/security-headers.test.ts`; conferidos nas respostas do `next dev` e do `next start`). Com a CSP ativa, a versão de produção carregou login e relatórios com gráficos sem violação. |
| SQL injection | Busca no código: nenhum SQL é montado com texto de usuário; todas as consultas usam `.bind`. O `sqlx` 0.9 só aceita SQL dinâmico dentro de `AssertSqlSafe`, que aparece apenas nos testes (nome do banco gerado pelo próprio teste). |
| HTTPS | Em produção o front-end envia `Strict-Transport-Security: max-age=63072000; includeSubDomains` (conferido no `next start`) e o cookie é `Secure`. Em desenvolvimento nenhum dos dois vale, por serem HTTP local. O HTTPS em si é da hospedagem (Vercel); a implantação ainda não foi feita. |
| Limite de tentativas | Novo: 5 senhas erradas em 15 minutos por e-mail e 3 pedidos de recuperação de senha por hora por e-mail, com `429` e `Retry-After`. Testes em `backend/tests/rate_limit.rs` e unitários. As contagens ficam na memória de cada instância. |

**Limitações conhecidas (decisões a rever).**

- O JWT vale 7 dias e não pode ser revogado: sair apaga o cookie no navegador, mas um token copiado segue válido até vencer; trocar a senha não derruba sessões já abertas. Trocar `JWT_SECRET` derruba todas.
- A CSP permite `'unsafe-inline'` em scripts e estilos: o Next.js injeta scripts inline e o uso de nonce exigiria renderização totalmente dinâmica, incompatível com a pré-renderização parcial adotada.
- `APP_URL` precisa ser exatamente o endereço público do front-end; se estiver errado, as gravações pela interface recebem 403.
- O limite de tentativas é por e-mail e por instância; não há limite por IP.
- `npm ci` falhou neste ambiente num momento por causa do `postinstall` do `unrs-resolver`; não foi reproduzido depois (`npm rebuild unrs-resolver` concluiu sem erro e `npm ci --ignore-scripts` funciona). O CI está verde, então tratei como problema só deste ambiente.

## 3. Usabilidade: interface responsiva

Auditoria no navegador do painel (Chromium) com a API real e um usuário com dados (60 despesas, receitas, fixos, 2 cartões, fatura com compras e parcelas, 3 orçamentos), em 375 px de largura (viewport de celular) e em desktop, nos temas claro e escuro: início, transações, fixos, categorias, cartões, faturas do cartão, saldo, orçamentos, relatórios, perfil, login, cadastro, recuperação e redefinição de senha.

- Rolagem horizontal da página: 0 px em todas as telas em 375 px (as tabelas do saldo e dos relatórios rolam dentro do próprio contêiner).
- Alvos de toque: botões e campos com altura de pelo menos 32 px. Ficaram abaixo disso apenas links de texto dentro de frases (por exemplo "Esqueci a senha", "Criar conta" nas telas de login); o link "Ver orçamentos" do alerta e o "Editar perfil" da página inicial foram ampliados neste ticket (área de toque com `py-2`).
- Teclado: o diálogo de nova transação abre com Enter, move o foco para dentro, tem nome (`aria-labelledby`) e fecha com Esc.

**Não verificado:** celular físico e toque real (o painel emula o tamanho e o agente de usuário, e os cliques chegam como cliques de mouse).

## 4. Portabilidade: navegadores

Verificado só em Chromium (o mesmo motor do Chrome e do Edge). **Firefox e Safari não foram testados** neste ambiente; o código usa recursos padrão (CSS do Tailwind v4, componentes Radix) e não depende de plugins, mas isso não é verificação. Pendência para o responsável antes da entrega final.

## 5. Acessibilidade básica (foco, teclado, rótulos)

- Teste automatizado `frontend/components/a11y.test.tsx` (axe-core, roda no CI): 13 formulários e diálogos (login, cadastro, esqueci a senha, redefinir, perfil, transação, saldo inicial, categoria, fixo, cartão, compra no cartão, compra parcelada, orçamento) sem violações; um teste confirma que o próprio axe detecta campo sem rótulo e botão sem nome. O jsdom não calcula layout nem cores.
- Axe no navegador (inclui contraste de cores) nas telas listadas acima, nos temas claro e escuro: sem violações no fim. A varredura achou 1 problema sério, corrigido: os botões de orçamento ("Alterar limite", "Definir limite") tinham nome acessível que não continha o texto visível (9 elementos); agora contêm.
- Foco visível: anel de foco de 4 px (`ring-ring` com recuo) ao navegar com Tab, medido no estilo calculado.
- Aviso de hidratação (corrigido): a saudação da página inicial e os formulários do perfil renderizavam o esqueleto no servidor e os dados no cliente quando o menu da conta já tinha buscado o usuário; um teste reproduz o erro de hidratação e passou a verde com `useHydrated`.

## 6. Disponibilidade

Não verificável: não há ambiente hospedado ainda (ticket #6 depende de contas do responsável). O `GET /health` existe e consulta o banco, para o monitoramento.

## 7. Manutenibilidade e CI

- Tipagem forte: TypeScript (`npm run typecheck`) e Rust.
- O workflow `.github/workflows/ci.yml` roda a cada PR e a cada push na `main`: job `backend` (`cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, que inclui todos os testes de integração das regras de negócio contra um PostgreSQL real; o teste de carga é `#[ignore]`) e job `frontend` (`typecheck`, `lint`, `npm test`, `build`). A exigência de "verde" para o merge é configuração de proteção da `main` no GitHub, que não foi alterada aqui.
- Contagem no fim deste ticket (`backend/scripts/check.sh` e `npm test`, tudo verde): 312 testes no back-end (unitários e de integração; mais 1 de carga ignorado) e 243 testes em 41 arquivos no front-end.

## 8. Escalabilidade

A arquitetura em camadas (handlers, use cases, repositórios) foi mantida; o estado do limite de tentativas é por instância (veja 2). A carga com 5 vezes mais Transações (10.000) mostrou crescimento pequeno nos tempos (seção 1). Não foi medido mais de um usuário com dados ao mesmo tempo nem várias instâncias.
