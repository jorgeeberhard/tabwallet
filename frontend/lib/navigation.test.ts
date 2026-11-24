import { describe, expect, it } from "vitest"

import { activeNavHref, NAV_ITEMS } from "@/lib/navigation"

describe("activeNavHref", () => {
  it("lists the eight sections in the order of the header", () => {
    expect(NAV_ITEMS.map((item) => item.label)).toEqual([
      "Início",
      "Transações",
      "Cartões",
      "Gastos e receitas fixos",
      "Categorias",
      "Orçamentos",
      "Relatórios",
      "Evolução do saldo",
    ])
  })

  it.each([
    ["/", "/"],
    ["/transacoes", "/transacoes"],
    ["/cartoes", "/cartoes"],
    ["/fixos", "/fixos"],
    ["/categorias", "/categorias"],
    ["/orcamentos", "/orcamentos"],
    ["/relatorios", "/relatorios"],
    ["/saldo", "/saldo"],
  ])("marks %s as the current section", (pathname, href) => {
    expect(activeNavHref(pathname)).toBe(href)
  })

  it("treats a nested route as its parent section", () => {
    expect(activeNavHref("/cartoes/6f1c9d2e")).toBe("/cartoes")
  })

  it("ignores a trailing slash", () => {
    expect(activeNavHref("/transacoes/")).toBe("/transacoes")
  })

  it("does not match a different section that only shares a prefix", () => {
    expect(activeNavHref("/cartoes-antigos")).toBeNull()
  })

  it("marks nothing on pages outside the menu, such as the profile", () => {
    expect(activeNavHref("/profile")).toBeNull()
  })
})
