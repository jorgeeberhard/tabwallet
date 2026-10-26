# Badge

A short label (one to three words) for a category, a status or a property of a transaction.

- Props: `tone` `neutral` (default), `brand`, `income`, `expense`, `warning`; `icon`; children are the text.
- `neutral` for categories and properties (Alimentação, Fixo, Parcela 3/10). Tones only for status: invoice Aberta = `brand`, Fechada = `warning`, Paga = `income`, Vencida = `expense`.
- Never use a badge as a button, and never invent a tone for decoration.

## shadcn/ui

`badge` with `rounded-full` and added variants `brand`, `income`, `expense`, `warning` (soft background + full-colour text). Neutral = `secondary` with `text-muted-foreground`.
