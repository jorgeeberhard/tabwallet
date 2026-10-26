# EmptyState

What a list, chart or page shows when there is nothing yet: an icon, one line on what is missing, one on why it matters, and the next action.

- Props: `icon` (default `inbox`), `title`, `description`, `action` (usually the primary `Button`).
- Write it as a fact plus an invitation, never as an error: "Nenhuma transação neste período" + "Nova transação".
- A filter that returned nothing gets a "Limpar filtros" ghost button instead of a create action.

## shadcn/ui

`empty` (recent shadcn versions) or a small own component with the same parts: icon in a `bg-brand-soft text-primary` disc, title, description, action.
