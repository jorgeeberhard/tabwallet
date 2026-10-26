# AppHeader

The top bar of every signed-in page: logo (link to Início), main navigation and the user's actions.

- Props: `items` `[{label, href, icon, active}]` (four at most: Início, Transações, Cartões, Relatórios), `right` (the "Nova" button and settings/profile), `homeHref`.
- The active item uses `aria-current="page"` and the `brand-soft` fill. Below `breakpoint-md` labels hide visually and icons remain (labels stay for screen readers).
- Height `space-16` (64px), `bg-surface`, a `border` hairline at the bottom. Login and sign-up pages show only the logo.

## shadcn/ui

Own component in `components/app-header.tsx`, built from `button`, `sheet` (navigation drawer below `md`) and `dropdown-menu` (account menu). Active link: `bg-brand-soft text-primary`.
