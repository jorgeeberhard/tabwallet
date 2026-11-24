/** The sections of the signed-in app, in the order the header lists them. */
export const NAV_ITEMS = [
  { href: "/", label: "Início" },
  { href: "/transacoes", label: "Transações" },
  { href: "/cartoes", label: "Cartões" },
  { href: "/fixos", label: "Gastos e receitas fixos" },
  { href: "/categorias", label: "Categorias" },
  { href: "/orcamentos", label: "Orçamentos" },
  { href: "/relatorios", label: "Relatórios" },
  { href: "/saldo", label: "Evolução do saldo" },
] as const

export type NavHref = (typeof NAV_ITEMS)[number]["href"]

/**
 * The header item that owns `pathname`: a nested route (a card's invoices under
 * `/cartoes`) belongs to its parent section. `null` for pages outside the menu,
 * such as the profile.
 */
export function activeNavHref(pathname: string): NavHref | null {
  const path = pathname.length > 1 ? pathname.replace(/\/+$/, "") : pathname
  if (path === "/") return "/"
  const match = NAV_ITEMS.find(
    (item) => item.href !== "/" && (path === item.href || path.startsWith(`${item.href}/`)),
  )
  return match?.href ?? null
}
