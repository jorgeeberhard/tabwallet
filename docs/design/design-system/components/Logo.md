# Logo

The TabWallet signature (wallet mark + wordmark) as an inline SVG that follows the theme tokens.

- Props: `variant` `full` (default) or `mark`; `height` in px (default 32; width follows); `tone` `default`, `white` (only on a `brand-primary` fill in the light theme) or `mono`; `label` for the accessible name (default "TabWallet").
- Use `full` in the app header (28px), login and e-mails; `mark` for favicon-sized spots, the collapsed header and avatars.
- Keep clear space of half the mark's height on every side. Minimum: 160px wide for `full`, 16px for `mark`.
- Never recolour, stretch, rotate, outline, add shadows or reorder the mark and wordmark. Static files for outside the app are in the Logos asset group.

## shadcn/ui

Own component (`components/logo.tsx`): inline SVG from `../logos/` with fills `fill-primary`, `fill-logo-tab`, `fill-logo-clasp` and the wordmark in `fill-primary` / `fill-foreground`, so it follows the `.dark` theme. Favicon and PWA icons use the static files.
