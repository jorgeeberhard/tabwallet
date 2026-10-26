# Button

The one way to trigger an action: save, create, confirm, delete.

- Props: `variant` `primary` (default), `secondary`, `ghost` or `danger`; `size` `sm` (32px), `md` (40px, default), `lg` (48px); `icon` / `iconRight` (Icon names); `loading`; `fullWidth`; plus every native `<button>` attribute. Children are the label.
- One `primary` per screen or dialog — the main thing the user came to do. `secondary` for the alternative (Cancelar), `ghost` for low-emphasis tools (filters, row actions), `danger` only to confirm a deletion inside a Dialog.
- Labels start with a verb, sentence case, no full stop: "Salvar transação", "Adiantar pagamento".
- Icon-only buttons (no children) must have an `aria-label`. Use `lg` + `fullWidth` for the main action on login and mobile forms.

## shadcn/ui

`button`. Variant map: `primary` → `default`, `secondary` → `outline`, `ghost` → `ghost`, `danger` → `destructive`. Sizes: `sm` → `sm` (`h-8`), `md` → `default` (`h-10`), `lg` → `lg` (`h-12`), icon-only → `icon`. Apply the adjustments in `../shadcn.md` §4 (hover `bg-primary-hover`, `text-destructive-foreground`, solid focus ring). `loading`: disable the button and render `spinner` (or `Loader2` with `animate-spin`) before the label.
