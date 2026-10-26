# BalanceCard

The current balance with the month's income and expenses — the first thing on the dashboard (UC10).

- Props: `balance`, `income`, `expense` (numbers in reais), `period` ("Setembro de 2026"), `label` (default "Saldo atual").
- One per page, top left. The balance is neutral ink; it turns `expense` and gets "−" only when negative.
- Pair it with the balance-history chart; never repeat the same numbers in another card.

## shadcn/ui

Own component on top of `card` (`CardHeader` + `CardContent`), using `Money` for the figures.
