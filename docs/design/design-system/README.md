# TabWallet — Design System

Fonte de verdade visual do front-end (Next.js / React / TypeScript). Versão navegável com prévias ao vivo: artifact **TabWallet** (Design System) no Claude.

## Como usar no código

O front-end usa **shadcn/ui** (Radix + Tailwind CSS v4) com o tema deste Design System — ver [ADR-0001](../../adr/0001-shadcn-ui-no-front-end.md).

1. Siga o guia [`shadcn.md`](shadcn.md): instalação, tema em [`shadcn/globals.css`](shadcn/globals.css), fontes com `next/font/local`, tema escuro com `next-themes` (classe `.dark`) e os ajustes obrigatórios nos componentes gerados.
2. Nunca use hex nem cores do Tailwind nas telas: use as variáveis do tema (`bg-primary`, `text-muted-foreground`, `bg-income-soft`…).
3. Componentes: os primitivos vêm do shadcn (`components/ui/`); os de domínio (`Money`, `TransactionRow`, `BalanceCard`, `BudgetBar`, `InvoiceCard`, `AppHeader`, `Logo`) são próprios, com os nomes e props de `components/index.d.ts`. Cada `components/<Nome>.md` traz a diretriz de uso e, no fim, a base shadcn correspondente. `components/reference.js` e `reference.css` são a implementação usada nas prévias do artifact — servem de guia visual, não de dependência.
4. Ícones: `lucide-react` com `strokeWidth={1.75}`.
5. `tokens.json` é a fonte de verdade. `shadcn/globals.css` e `tokens.css` são derivados dele: ao mudar um token, atualize os dois. `tokens.css` não é importado no app; fica para documentação e usos fora do Next.js.

## Arquivos

| Caminho | Conteúdo |
| --- | --- |
| `tokens.json` | Tokens (cores claro/escuro, tipografia, espaço, raios, sombras, tamanhos, breakpoints) |
| `tokens.css` | Variáveis CSS, `@font-face` e classes de texto (referência; o app usa `shadcn/globals.css`) |
| `shadcn.md` | Guia de uso do shadcn/ui com o tema TabWallet: instalação, mapeamento, ajustes e componentes |
| `shadcn/globals.css` | Tema pronto para `front-end/app/globals.css` (claro e escuro) |
| `components/` | `index.d.ts` (props), uma diretriz `.md` por componente (com a base shadcn), implementação de referência |
| `logos/` | 5 SVGs oficiais + regras de uso |
| `fonts/` | Manrope, Inter e JetBrains Mono (woff2, latin) + licenças SIL OFL 1.1 |

## Componentes

- [`Alert`](components/Alert.md)
- [`AppHeader`](components/AppHeader.md)
- [`Badge`](components/Badge.md)
- [`BalanceCard`](components/BalanceCard.md)
- [`BudgetBar`](components/BudgetBar.md)
- [`Button`](components/Button.md)
- [`Card`](components/Card.md)
- [`Dialog`](components/Dialog.md)
- [`EmptyState`](components/EmptyState.md)
- [`Icon`](components/Icon.md)
- [`InvoiceCard`](components/InvoiceCard.md)
- [`Logo`](components/Logo.md)
- [`Money`](components/Money.md)
- [`MoneyInput`](components/MoneyInput.md)
- [`SegmentedControl`](components/SegmentedControl.md)
- [`Select`](components/Select.md)
- [`TextField`](components/TextField.md)
- [`TransactionRow`](components/TransactionRow.md)

## Guia da marca na interface

TabWallet é um site gratuito de controle financeiro pessoal: a pessoa registra receitas, despesas, gastos fixos e faturas de cartão, e acompanha saldo e orçamentos. A interface deve ser **clara, calma e precisa** — números fáceis de ler, nada de alarme. Palavras-chave: Clareza · Controle · Autonomia. Tagline: “Seu dinheiro, em uma aba.”

## Conteúdo e voz

- Escreva em português do Brasil e trate a pessoa por “você”. Código, tokens e nomes de componentes ficam em inglês.
- **Direto:** frases curtas; botões começam com verbo, em caixa de frase e sem ponto final — “Salvar transação”, “Adiantar pagamento”, “Nova transação”.
- **Calmo:** informe sem alarme e sem julgar gastos. Prefira “Você passou R$ 120,00 do limite de Lazer.” a “ATENÇÃO!!! Você gastou demais!”.
- **Próximo:** sem jargão financeiro — “gasto fixo”, “parcela”, “fatura”, “limite”.
- Sucesso é curto (“Transação salva.”); erro diz o que fazer (“Não foi possível salvar. Verifique a conexão e tente de novo.”) e nunca mostra códigos técnicos; estado vazio explica e oferece a próxima ação (“Nenhuma transação neste período.” + “Nova transação”).
- Sem emoji e sem pontos de exclamação. Caixa alta só em legendas curtas.
- Formatos: `R$ 1.234,56` (use `TabWallet.formatBRL`); receitas `+ R$ 3.200,00`; despesas `− R$ 45,90` com o sinal de subtração (U+2212); datas `30/09/2026`; meses por extenso em minúsculas no meio da frase (“setembro de 2026”); parcelas `3/10`.

## Cor

- Fundo da página `bg-canvas`; cards, modais, campos e cabeçalho em `bg-surface`; trilhos, hovers e áreas recuadas em `bg-subtle`.
- Texto em `text-primary`; apoio, metadados e placeholders em `text-secondary`. Os dois passam 4,5:1 sobre `bg-canvas`, `bg-surface` e `bg-subtle` nos dois temas.
- `brand-primary` (Índigo) é a cor de ação: botão principal, links, item ativo do menu, foco. Proporção: ~60% superfícies, ~30% texto e neutros, ~10% Índigo. Texto sobre preenchimento Índigo é `on-brand` — nunca branco fixo (no tema escuro ele é escuro).
- `brand-secondary` (Lavanda) é decorativa: logo e ilustrações, nunca texto. `brand-soft` é o fundo de destaques com texto `brand-primary`.
- Semânticas: `income` para receitas, saldo positivo e orçamento dentro do limite; `expense` para despesas, saldo negativo, erros e exclusões; `warning` para orçamento a partir de 80% do limite. Fundos suaves `income-soft`, `expense-soft`, `warning-soft` sempre com o texto na cor cheia.
- **A cor nunca fala sozinha.** `income` e `expense` têm luminosidade parecida e se distinguem só pelo matiz: toda receita leva “+” e `arrow-up-right`, toda despesa “−” e `arrow-down-left`, todo alerta tem texto.
- `border` para divisórias e contorno de cards; `border-strong` (3:1) para contorno de campos e do botão secundário.
- Dois temas, claro (padrão) e escuro, trocados pela classe `.dark` no `<html>` via `next-themes` (`defaultTheme="system"`, que respeita `prefers-color-scheme`). Nunca escreva hex no código das telas — use as variáveis.

## Tipografia

- **Manrope** (`--font-display`) só para títulos: `display`, `heading-1` (um por página), `heading-2` (seções), `heading-3` (cards e diálogos).
- **Inter** (`--font-body`) para todo o resto: `body` (16px, padrão), `body-sm`, `label`, `caption` (12px, o mínimo).
- **Valores** sempre em Inter 600 com `font-variant-numeric: tabular-nums` (`money-lg`, `money-md` ou o componente `Money`), para que colunas de valores se alinhem.
- **JetBrains Mono** (`--font-mono`) apenas para código e documentação técnica.
- Pesos: 400 texto, 500 rótulos, 600 ênfase e valores, 700–800 títulos. As três famílias são SIL OFL 1.1 — uso comercial livre; licenças em `fonts/`.

## Espaço, forma e profundidade

- Grade de 4px: `space-1` (4) a `space-16` (64). Padding de card e diálogo `space-6`; entre campos `space-4`; entre seções `space-8`.
- Raios: `radius-sm` campos, selects e itens de menu; `radius-md` botões e alertas; `radius-lg` cards e diálogos; `radius-full` badges, chips, avatares e barras.
- Profundidade discreta: cards com `border` + `shadow-sm`; menus `shadow-md`; diálogos `shadow-lg` sobre `overlay`.
- Controles com `control-md` (40px) de altura; `control-sm` em linhas densas; `control-lg` na ação principal do login e de formulários no celular. Área de toque mínima de 40px.
- Foco: anel sólido de 2px em `focus-ring`, afastado 2px, em todo elemento interativo (`:focus-visible`). Nunca remova o contorno sem substituí-lo.
- Layout responsivo, mobile primeiro. Margem lateral `space-4` no celular e `space-6` a partir de `breakpoint-md`; grade do painel com 1 coluna, 2 a partir de `breakpoint-sm` e 3 a partir de `breakpoint-lg`; conteúdo limitado a 1200px.
- Movimento: só transições de 150ms ease-out em hover e foco; nada de animação decorativa; respeite `prefers-reduced-motion`.

## Iconografia

- Ícones **Lucide** (ISC, código aberto) em contorno, traço 1,75px, tamanhos 16, 20 ou 24px, cor `currentColor`. No app, `lucide-react`; aqui, o componente `Icon` traz o subconjunto usado.
- Um ícone acompanha uma palavra; ícone sozinho só em botões com `aria-label`. Sem emoji.
- Significados fixos: receita `arrow-up-right`, despesa `arrow-down-left`, gasto fixo `repeat`, cartão e fatura `credit-card`, categoria `tag`, orçamento `chart-pie`, alerta `triangle-alert`.

## Logo

- Use o componente `Logo` nas telas e os arquivos do grupo Logos fora delas; nunca redesenhe a marca. O símbolo é uma carteira cuja parte de cima forma uma aba; o logotipo é Manrope ExtraBold com “Tab” em `brand-primary` e “Wallet” em `text-primary`.
- Área livre: metade da altura do símbolo em todos os lados. Mínimo: 160px de largura para a assinatura; abaixo disso, só o símbolo, nunca menor que 16px.
- Não recolorir, distorcer, rotacionar, contornar, aplicar sombra ou reorganizar os elementos.

## Componentes e casos de uso

- Contas e acesso (UC01–UC03, UC14): `TextField`, `Button` `lg` + `fullWidth`, `Alert`.
- Transações (UC04–UC08): `TransactionRow` em `Card` `flush`, `MoneyInput`, `Select` de categoria, `SegmentedControl` Receita/Despesa e filtros, `Dialog` `danger` para excluir, `EmptyState`.
- Categorias (UC09): `Badge` neutro; categorias desativadas não aparecem no `Select` de novos lançamentos.
- Saldo e relatórios (UC10–UC11): `BalanceCard`; gráficos com Recharts usando `brand-primary`, `income`, `expense` e neutros, sempre com legenda escrita.
- Orçamentos (UC12–UC13): `BudgetBar` e `Alert` `warning`/`danger`.
- Cartões (UC15–UC18): `InvoiceCard`, `TransactionRow` com `installment`, `Dialog` + `MoneyInput` para adiantar pagamento.
- Navegação: `AppHeader` com Início, Transações, Cartões e Relatórios.
- No front-end, monte os componentes sobre o shadcn/ui conforme [`shadcn.md`](shadcn.md), com os mesmos nomes e props de `components/index.d.ts` para os componentes de domínio.
