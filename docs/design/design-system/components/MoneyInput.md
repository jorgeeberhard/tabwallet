# MoneyInput

A text input for amounts in reais, with a fixed "R$" prefix, tabular figures and the numeric keyboard on phones.

- Props: same as `TextField`. The value is the string the user typed ("1.180,40"); convert it to cents in the form layer before sending it to the API.
- Amounts are always positive here — whether it is a receita or a despesa is chosen with a `SegmentedControl` next to it, never with a minus sign.
- Accept both "1180,4" and "1.180,40"; format back to `1.180,40` on blur.

## shadcn/ui

`input-group` with an "R$" addon (recent shadcn versions), or `input` inside a `relative` wrapper with an absolutely positioned prefix and `pl-11`. `inputMode="decimal"`, `tabular-nums font-semibold`. Wired to `react-hook-form` through `form`/`field`; the zod schema converts the string to cents.
