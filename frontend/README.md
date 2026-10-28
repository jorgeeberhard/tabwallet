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

## Categorias

`app/(app)/categorias` mostra a tela de categorias (`components/categories/`), com atalho na página inicial: alterna Despesas e Receitas, lista as ativas e as desativadas, e permite criar, renomear, desativar e reativar. Se o nome digitado é o de uma categoria desativada, o diálogo oferece "Reativar categoria". As chamadas ficam em `lib/categories-api.ts` e os textos de erro em `lib/category-messages.ts`.

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
