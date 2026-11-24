import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { render, screen, within } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { AppHeader } from "@/components/app-header"

let pathname = "/"
vi.mock("next/navigation", () => ({ usePathname: () => pathname }))

function answerBudgetStatus(exceededCount: number) {
  const lines = Array.from({ length: exceededCount }, (_, i) => ({
    category: { id: `c${i}`, name: `Categoria ${i}`, type: "expense", active: true },
    limit_cents: 1000,
    spent_cents: 2000,
    remaining_cents: -1000,
    percent_used: 200,
    exceeded: true,
  }))
  vi.stubGlobal(
    "fetch",
    vi.fn(() =>
      Promise.resolve(
        new Response(JSON.stringify({ month: "2026-01", budgets: lines, exceeded_count: exceededCount }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        }),
      ),
    ),
  )
}

function renderHeader() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return render(
    <QueryClientProvider client={client}>
      <AppHeader right={<button type="button">Conta</button>} />
    </QueryClientProvider>,
  )
}

const LABELS = [
  "Início",
  "Transações",
  "Cartões",
  "Gastos e receitas fixos",
  "Categorias",
  "Orçamentos",
  "Relatórios",
  "Evolução do saldo",
]

describe("AppHeader", () => {
  beforeEach(() => {
    pathname = "/"
    answerBudgetStatus(0)
  })

  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it("links the logo to the home page", () => {
    renderHeader()

    expect(screen.getByRole("link", { name: "TabWallet" })).toHaveAttribute("href", "/")
  })

  it("lists the eight sections, then the account area it was given", () => {
    renderHeader()

    const nav = screen.getByRole("navigation", { name: "Principal" })
    expect(within(nav).getAllByRole("link").map((link) => link.textContent)).toEqual(LABELS)
    expect(screen.getByRole("button", { name: "Conta" })).toBeInTheDocument()
  })

  it("marks the current page for screen readers, and only that one", () => {
    pathname = "/transacoes"
    renderHeader()

    const nav = screen.getByRole("navigation", { name: "Principal" })
    const current = within(nav).getAllByRole("link", { current: "page" })
    expect(current.map((link) => link.textContent)).toEqual(["Transações"])
  })

  it("keeps the parent section marked on a nested route", () => {
    pathname = "/cartoes/6f1c9d2e"
    renderHeader()

    const nav = screen.getByRole("navigation", { name: "Principal" })
    expect(within(nav).getByRole("link", { current: "page" })).toHaveTextContent("Cartões")
  })

  it("marks nothing on a page outside the menu", () => {
    pathname = "/profile"
    renderHeader()

    expect(screen.queryByRole("link", { current: "page" })).not.toBeInTheDocument()
  })

  it("shows how many budgets are over their limit on Orçamentos", async () => {
    answerBudgetStatus(2)
    renderHeader()

    const nav = screen.getByRole("navigation", { name: "Principal" })
    expect(await within(nav).findByText("2 acima do limite")).toBeInTheDocument()
  })

  describe("menu for small screens", () => {
    it("is closed at first and opens with the same links, the current one marked", async () => {
      pathname = "/relatorios"
      renderHeader()
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument()

      await userEvent.click(screen.getByRole("button", { name: "Abrir menu" }))

      const dialog = await screen.findByRole("dialog", { name: "Menu" })
      expect(within(dialog).getAllByRole("link").map((link) => link.textContent)).toEqual(LABELS)
      expect(within(dialog).getByRole("link", { current: "page" })).toHaveTextContent("Relatórios")
    })

    it("closes with Escape and gives the focus back to the menu button", async () => {
      renderHeader()
      const button = screen.getByRole("button", { name: "Abrir menu" })
      await userEvent.click(button)
      await screen.findByRole("dialog", { name: "Menu" })

      await userEvent.keyboard("{Escape}")

      expect(screen.queryByRole("dialog")).not.toBeInTheDocument()
      expect(button).toHaveFocus()
    })

    it("closes when a link is chosen", async () => {
      renderHeader()
      await userEvent.click(screen.getByRole("button", { name: "Abrir menu" }))
      const dialog = await screen.findByRole("dialog", { name: "Menu" })

      await userEvent.click(within(dialog).getByRole("link", { name: "Categorias" }))

      expect(screen.queryByRole("dialog")).not.toBeInTheDocument()
    })

    it("closes with its own button", async () => {
      renderHeader()
      await userEvent.click(screen.getByRole("button", { name: "Abrir menu" }))
      const dialog = await screen.findByRole("dialog", { name: "Menu" })

      await userEvent.click(within(dialog).getByRole("button", { name: "Fechar" }))

      expect(screen.queryByRole("dialog")).not.toBeInTheDocument()
    })
  })
})
