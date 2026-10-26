# TextField

A labelled single-line input with an optional hint or error message.

- Props: `label`, `hint`, `error` (replaces the hint and marks the input invalid), `optional` (adds "(opcional)"), plus every native `<input>` attribute (`type`, `value`, `onChange`, `placeholder`, `autoComplete`…).
- Always give a visible `label`; placeholders show an example, never the label.
- Errors say what to do: "Digite um e-mail válido, como nome@dominio.com.", not "Campo inválido".
- Mark the optional fields, not the required ones. Stack fields with `space-4` between them.

## shadcn/ui

`form` (or `field` in recent versions) + `label` + `input`, validated with `react-hook-form` + `zod`. Hint → `FormDescription`, error → `FormMessage` (`text-destructive`). Input adjusted per `../shadcn.md` §4 (`h-10`, `bg-card`, `border-input`).
