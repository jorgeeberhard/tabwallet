# TabWallet front-end

Aplicação Next.js (App Router, TypeScript) com shadcn/ui sobre Tailwind CSS v4, usando o tema do Design System ([ADR-0001](../docs/adr/0001-shadcn-ui-no-front-end.md)). Dados remotos com TanStack Query; tema claro e escuro com `next-themes` (classe `.dark`).

## Organização

- `app/`: rotas, `layout.tsx` (fontes, `Providers`) e `globals.css` (tema do Design System, copiado de `docs/design/design-system/shadcn/globals.css`).
- `components/ui/`: componentes do shadcn, já ajustados ao Design System (`docs/design/design-system/shadcn.md`, seção 4).
- `components/`: componentes de domínio próprios (`Logo`, `Money`, `ThemeToggle`; os demais entram com as telas).
- `lib/`: utilitários (`format.ts`: `formatBRL`).

## Autenticação

- O navegador só fala com este app: `next.config.ts` encaminha `/api/*` para a API (`API_URL`, padrão `http://127.0.0.1:8080`). Com uma origem só, o cookie de sessão `httpOnly` é de primeira parte (`SameSite=Lax`) e a API não precisa de CORS. Copie `.env.example` para `.env.local`.
- `proxy.ts` (o "middleware" do Next 16) roda em toda página: lê o cookie `tabwallet_session`, pergunta à API (`GET /auth/me`) se a sessão vale e redireciona visitantes para `/login`; quem já entrou é levado de `/login` e `/signup` para `/`. A regra de decisão está em `lib/access.ts`.
- `lib/api.ts` chama a API (`signUp`, `logIn`, `logOut`, `getCurrentUser`) e converte erros para `ApiError` com o `code` estável da API; `lib/auth-messages.ts` traduz cada código para o texto em português.
- Telas: `app/(auth)/login` e `app/(auth)/signup` (formulários com `react-hook-form` + `zod`, `lib/auth-schemas.ts`); `app/(app)` é a área logada, com o menu da conta e a ação "Sair".

## Perfil e recuperação de senha

- `app/(app)/profile`: tela de perfil (`components/profile/profile-forms.tsx`) com três formulários independentes (nome, e-mail, senha). Trocar e-mail ou senha pede a senha atual. Acessível pelo menu da conta e pela página inicial.
- `app/(auth)/forgot-password`: pede o link de redefinição. A resposta é sempre a mesma ("se existir uma conta..."), porque a API não revela quais e-mails existem. O login tem o link "Esqueci a senha".
- `app/(auth)/reset-password?token=...`: página aberta pelo link do e-mail; define a nova senha e leva ao login (não inicia sessão). Fica acessível mesmo para quem já está logado (`lib/access.ts`). Em desenvolvimento sem provedor de e-mail, o link aparece no log da API.

## Categorias

`app/(app)/categorias` mostra a tela de categorias (`components/categories/`), com atalho na página inicial: alterna Despesas e Receitas, lista as ativas e as desativadas, e permite criar, renomear, desativar e reativar. Se o nome digitado é o de uma categoria desativada, o diálogo oferece "Reativar categoria". As chamadas ficam em `lib/categories-api.ts` e os textos de erro em `lib/category-messages.ts`.

## Transações

`app/(app)/transacoes` mostra o extrato (`components/transactions/`), com atalho na página inicial: lista agrupada por mês, mais novas primeiro, com filtros de tipo, Categoria e período, "Carregar mais" (páginas de 50) e estado vazio. Cada linha (`TransactionRow`) marca as Projeções (data depois de hoje) com um selo "Projeção"; uma Categoria desativada continua aparecendo na Transação antiga. O diálogo de lançamento/edição tem alternância Despesa/Receita, `MoneyInput`, data, descrição e Categoria (só as ativas do tipo escolhido; ao editar, a atual continua na lista). Excluir pede confirmação. A API fala em centavos: `lib/amount.ts` converte o que a pessoa digita ("1.180,40") para centavos sem usar ponto flutuante, e `lib/transactions-api.ts` converte para reais na borda, para o `Money`. "Hoje" no formulário é o dia de America/Sao_Paulo (`lib/today.ts`), o mesmo da API.

## Gastos e receitas fixos

`app/(app)/fixos` mostra a tela "Gastos e receitas fixos" (`components/recurring/`), com atalho na página inicial: lista os lançamentos fixos em andamento (dia do mês, Categoria, próxima data e valor) e os encerrados, e permite cadastrar (tipo, valor, dia do mês de 1 a 31, mês de início, descrição, Categoria), editar e encerrar (com confirmação). Editar o valor vale a partir do mês atual, o que o diálogo avisa; o tipo e o dia não mudam. `lib/recurring-api.ts` fala com a API e `lib/recurring-messages.ts` traduz os erros.

No extrato (`/transacoes`), as Ocorrências aparecem como itens `kind: "occurrence"` (`OccurrenceRow`), com o selo "Gasto fixo" ou "Receita fixa" e o selo "Projeção" quando a data é futura. Cada uma tem as ações "Editar valor só deste mês" (`OccurrenceAmountDialog`) e "Excluir só este mês" (confirmação), que não afetam os outros meses. Todas as ações invalidam o extrato e o Saldo atual, que a API recalcula (nada é gravado antes do usuário alterar algo).

## Saldo atual e Saldo inicial

A página inicial (`app/(app)/page.tsx`) mostra o Saldo atual logo abaixo da saudação (`components/balance/balance-section.tsx`, com o `BalanceCard` do Design System em `components/balance-card.tsx`; a versão atual não traz Receitas e Despesas do mês, que ainda não são calculadas). Saldo negativo aparece com "−" e na cor de despesa. Enquanto o Saldo inicial não foi informado, o cartão oferece "Informar saldo inicial" (`opening-balance-dialog.tsx`: `MoneyInput` com `signed`, que aceita zero e "-200,00", e o aviso de que só vale uma vez); depois de informado o botão some e o valor aparece como "Saldo inicial informado". `lib/balance-api.ts` converte centavos em reais na borda. Lançar, editar ou excluir uma Transação invalida `BALANCE_KEY`, então o saldo se atualiza sozinho. O saldo vem pronto da API (que decide o que já é "hoje"); o front-end não recalcula nada.

## Evolução do saldo

`app/(app)/saldo` mostra a evolução do saldo (`components/balance/balance-evolution.tsx`), com atalho "Evolução do saldo" na página inicial: o histórico do Saldo atual até hoje e o Saldo projetado dos próximos meses (até 12), por mês ou por dia, com seletores de período para cada lado (`lib/balance-series.ts` tem as opções e a conta das datas; o back-end valida o horizonte). Três cartões resumem o Saldo atual, o Saldo projetado no fim do período e o menor Saldo projetado. O gráfico (`balance-chart.tsx`, Recharts sobre `components/ui/chart.tsx`) desenha o passado como linha contínua na cor da marca e a projeção como linha tracejada neutra, com a marca "Hoje" entre elas e uma legenda escrita; a diferença não depende só da cor. Os mesmos valores estão na tabela "Ver os valores em tabela" (`balance-table.tsx`), com cada linha marcada "Histórico" ou "Projetado" em texto. `lib/balance-series-api.ts` chama `GET /balance/history` e `GET /balance/projection` e converte centavos em reais na borda; as consultas ficam sob `BALANCE_KEY`, então lançar, editar ou excluir uma Transação as atualiza. Compras no cartão ainda não lançadas não entram na projeção, e as Faturas entrarão nos tickets de pagamento de fatura.

## Pré-requisitos

- Node.js 22 e npm.

## Rodar localmente

```bash
cd frontend
npm install
npm run dev        # http://localhost:3000
```

O back-end é independente: para subir a API, veja `../backend/README.md`.

## Verificações

```bash
npm run typecheck  # next typegen + tsc --noEmit
npm run lint       # eslint
npm test           # vitest (testes de componentes de domínio)
npm run build      # build de produção
```

O CI (job `frontend`) roda `typecheck`, `lint`, `test` e `build` a cada PR.

## Adicionar componentes do shadcn

```bash
npx shadcn@latest add <componente>
```

Revise o diff antes de aceitar: o CLI pode sobrescrever os ajustes feitos nos arquivos de `components/ui/`. Depois de adicionar, aplique os ajustes da seção 4 do guia (anel de foco sólido, alturas, raios, variantes de `badge` e `alert`). Use apenas as variáveis do tema (`bg-primary`, `text-income`…), nunca hex nem cores do Tailwind.

## Se o tema mudar

`docs/design/design-system/tokens.json` é a fonte de verdade. Atualize também `app/globals.css` e `docs/design/design-system/shadcn/globals.css`.
