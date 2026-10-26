# BudgetBar

Progress of the month's spending in one category against its limit (UC12) and its alert state (UC13).

- Props: `category`, `spent`, `limit` (numbers in reais), `warnAt` (default 0.8).
- States: below 80% = `income` bar, "41% usado"; from 80% = `warning`, "92% do limite" with an alert icon; over 100% = `expense`, full bar, "Limite excedido em R$ 120,00". The state is always written, never colour alone.
- Stack bars in a `Card` with `space-5`–`space-6` between them, most critical first.

## shadcn/ui

Own component on top of `progress`, with `indicatorClassName` set to `bg-income`, `bg-warning` or `bg-expense` by state. The status text stays visible next to the bar.
