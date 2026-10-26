# Select

A native dropdown with the TabWallet field styling, for picking one item from a list (category, card, month).

- Props: `options` `[{value, label, disabled?}]`, `placeholderOption` (a disabled first line such as "Escolha uma categoria"), plus `label`, `hint`, `error` and native `<select>` attributes.
- Deactivated categories (UC09) are not offered for new transactions; keep them in the list only when editing a transaction that already uses one.
- Two to four options that change the view instantly: use `SegmentedControl`.

## shadcn/ui

`select` (Radix). For long or searchable category lists, a combobox from `popover` + `command`. Deactivated categories are not rendered as `SelectItem` for new transactions.
