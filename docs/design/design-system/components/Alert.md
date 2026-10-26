# Alert

An inline message about the state of the user's money or of an action: budget alerts (UC13), saved, failed, upcoming invoice.

- Props: `tone` `info`, `success`, `warning`, `danger`; `title` (the message itself, one sentence); children for one line of detail; `action` for a button.
- Budget: `warning` from 80% of a limit, `danger` once it is exceeded — always with the category and the amount in words.
- Calm, factual and specific: "Você passou R$ 120,00 do limite de Lazer.", never "ATENÇÃO!". No technical error codes.
- Warnings and errors are announced to screen readers (`role="alert"`); info and success are polite (`role="status"`).

## shadcn/ui

`alert`, with added variants `info` (`bg-brand-soft`), `success` (`bg-income-soft`), `warning` (`bg-warning-soft`) and `destructive` on `bg-expense-soft`. Short-lived confirmations ("Transação salva.") use `toast` from `sonner` instead of an inline Alert.
