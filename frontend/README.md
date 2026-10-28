# TabWallet front-end

Aplicação Next.js (App Router, TypeScript) com shadcn/ui sobre Tailwind CSS v4, usando o tema do Design System ([ADR-0001](../docs/adr/0001-shadcn-ui-no-front-end.md)). Dados remotos com TanStack Query; tema claro e escuro com `next-themes` (classe `.dark`).

## Organização

- `app/`: rotas, `layout.tsx` (fontes, `Providers`) e `globals.css` (tema do Design System, copiado de `docs/design/design-system/shadcn/globals.css`).
- `components/ui/`: componentes do shadcn, já ajustados ao Design System (`docs/design/design-system/shadcn.md`, seção 4).
- `components/`: componentes de domínio próprios (`Logo`, `Money`, `ThemeToggle`; os demais entram com as telas).
- `lib/`: utilitários (`format.ts`: `formatBRL`).

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
