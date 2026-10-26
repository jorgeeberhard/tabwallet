# Icon

An outline icon from the Lucide set (ISC licence), drawn in `currentColor` so it takes the colour of the text around it.

- Props: `name` (one of `Icon.names`), `size` 16, 20 (default) or 24, `label` when the icon carries meaning on its own (otherwise it is hidden from screen readers).
- In the Next.js app use `lucide-react` with `strokeWidth={1.75}`; the names match.
- Meaning map: receita `arrow-up-right`, despesa `arrow-down-left`, gasto fixo `repeat`, cartão/fatura `credit-card`, categoria `tag`, orçamento `chart-pie`, alerta `triangle-alert`, editar `pencil`, excluir `trash-2`.
- An icon sits next to a word. Icon-only buttons need `aria-label`. No emoji.

## shadcn/ui

`lucide-react` directly (shadcn's default icon library): `<Wallet className="size-5" strokeWidth={1.75} />`. Decorative icons get `aria-hidden`; meaningful ones an `aria-label`.
