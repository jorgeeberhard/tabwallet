# SegmentedControl

A row of two to four mutually exclusive options that filter what is on screen right away (UC08).

- Props: `options` `[{value, label}]`, `value` + `onChange` (controlled) or `defaultValue`, and `label` (required: names the group for screen readers).
- Use for type filters (Todas / Receitas / Despesas) and period switches. Arrow keys move the selection.
- More than four options, or options that need a submit: use `Select`.

## shadcn/ui

`toggle-group` with `type="single"` and an `aria-label` (filters that change the list in place). When each option shows a different panel, use `tabs` instead. Track: `bg-muted p-1 rounded-lg`; selected item `data-[state=on]:bg-card data-[state=on]:shadow-sm`.
