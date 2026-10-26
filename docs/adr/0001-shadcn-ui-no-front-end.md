# ADR-0001 — shadcn/ui como base de componentes do front-end

- **Status:** aceita
- **Data:** 26/10/2026
- **Decisor:** Jorge Miguel Eberhard da Conceição

## Contexto

O termo de abertura (seção 4.2) define o front-end em Next.js (App Router), React e TypeScript, com TanStack Query e Recharts. Ele não define a biblioteca de componentes nem a forma de estilizar.

O Design System TabWallet (`docs/design/design-system`) já especifica tokens como variáveis CSS, temas claro e escuro, tipografia e 18 componentes. O desenvolvimento do front-end tem duas semanas no cronograma (02/11 – 15/11). Construir do zero componentes acessíveis, como modais com foco preso, selects com navegação por teclado e calendário, consumiria boa parte desse prazo.

## Decisão

Usar **shadcn/ui** sobre **Tailwind CSS v4** como base dos componentes de interface, com o tema do Design System TabWallet:

- As variáveis do shadcn recebem os valores dos tokens (`docs/design/design-system/shadcn/globals.css`).
- Os componentes gerados em `components/ui/` são ajustados uma vez às regras do Design System (`docs/design/design-system/shadcn.md`, seção 4).
- Os componentes de domínio (`Money`, `TransactionRow`, `BalanceCard`, `BudgetBar`, `InvoiceCard`, `AppHeader`, `Logo`) são próprios, compostos a partir dos do shadcn.
- O tema escuro passa a ser a classe `.dark`, controlada por `next-themes`.
- O Tailwind CSS entra na stack do front-end.

## Alternativas consideradas

- **Componentes próprios com CSS puro sobre `tokens.css`:** controle total, mas exige implementar acessibilidade (Radix já resolve) e mais tempo do que o cronograma permite.
- **Biblioteca fechada (MUI, Chakra, Mantine):** componentes prontos, mas o tema é próprio de cada biblioteca, mais difícil de alinhar aos tokens. O código fica numa dependência, não no repositório.

## Consequências

- **Positivas:**
  - Acessibilidade (foco, teclado, ARIA) vinda do Radix.
  - Os componentes ficam no repositório e podem ser editados.
  - O tema é feito de variáveis CSS, como o Design System.
  - O `chart` do shadcn já usa Recharts, previsto no termo de abertura.
  - Formulários com `react-hook-form` + `zod` tipados.
- **Negativas / cuidados:**
  - Sem o tema e os ajustes da seção 4 do guia, o site fica com a aparência padrão do shadcn.
  - Atualizar um componente pelo CLI pode sobrescrever ajustes locais: revise o diff antes de aceitar.
  - O `tokens.css` deixa de ser importado no app, e os valores passam a existir em dois arquivos (`tokens.json` e `globals.css`). Mudanças de token devem atualizar ambos.
  - Adiciona Tailwind CSS, `next-themes`, `lucide-react`, `react-hook-form` e `zod` às dependências do front-end.
