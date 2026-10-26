# TransactionRow

One line of the statement: direction icon, description, category and date, amount, and row actions (UC04–UC08, UC05 fixed expenses, UC18 instalments).

- Props: `description`, `amount` (number; the sign picks income/expense unless `kind` is set), `category`, `date` (already formatted "DD/MM/AAAA"), `installment` `{current, total}`, `recurring`, `onEdit`, `onDelete`.
- Render rows inside `<ul className="tw-txn-list">` in a `Card` with `flush`. Newest first, grouped by month.
- Row actions are ghost icon buttons labelled "Editar …" / "Excluir …"; on phones they hide — open the transaction to edit. Deleting always goes through a danger `Dialog`.
- Long descriptions truncate with an ellipsis; the amount never wraps.

## shadcn/ui

Own component on top of `badge` (category, Fixo, Parcela n/N), `button` (`ghost`, `icon`) and, below `sm`, a `dropdown-menu` holding Editar / Excluir. Deleting opens an `alert-dialog`. A desktop statement may render the same data in `table`.
