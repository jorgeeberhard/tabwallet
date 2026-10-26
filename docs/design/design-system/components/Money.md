# Money

Every amount in reais on screen: formatted pt-BR, tabular figures, sign and colour from its meaning.

- Props: `value` (number, in reais), `kind` `auto` (default: positive = income, negative = expense), `income`, `expense` or `neutral`; `size` `sm`, `md`, `lg`; `showSign={false}` for balances and totals.
- Income reads `+ R$ 3.200,00` in `income`; expense `− R$ 1.180,40` in `expense` (true minus sign, U+2212). Colour never carries the meaning alone.
- Neutral amounts (limits, invoice totals) are `text-primary`. A negative balance with `showSign={false}` still shows "−" and turns `expense`.
- Format elsewhere with `TabWallet.formatBRL(value)`; never build "R$" strings by hand.

## shadcn/ui

Own component (`components/money.tsx`) — no shadcn equivalent. A `span` with `tabular-nums` and `text-income` / `text-expense` / `text-foreground`, formatted by a shared `formatBRL()` in `lib/format.ts` (`Intl.NumberFormat("pt-BR", { style: "currency", currency: "BRL" })`).
