import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { render, screen, within } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { TransactionsScreen } from "@/components/transactions/transactions-screen"

vi.mock("sonner", () => ({ toast: { success: vi.fn(), error: vi.fn() } }))

function json(status: number, body?: unknown) {
  return Promise.resolve(
    new Response(body === undefined ? null : JSON.stringify(body), {
      status,
      headers: { "Content-Type": "application/json" },
    }),
  )
}

function item(id: string, description: string, date: string, overrides = {}) {
  return {
    kind: "transaction",
    id,
    type: "expense",
    amount_cents: 1000,
    date,
    description,
    category: { id: "c1", name: "Alimentação", type: "expense", active: true },
    is_projection: false,
    ...overrides,
  }
}

function statement(items: unknown[]) {
  return { items, total: items.length, limit: 50, offset: 0 }
}

function renderScreen() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return render(
    <QueryClientProvider client={client}>
      <TransactionsScreen />
    </QueryClientProvider>,
  )
}

describe("TransactionsScreen", () => {
  const fetchMock = vi.fn()
  const categories = [
    { id: "c1", name: "Alimentação", type: "expense", active: true, is_charges: false },
    { id: "c2", name: "Salário", type: "income", active: true, is_charges: false },
  ]
  let rows: unknown[]

  beforeEach(() => {
    rows = []
    vi.stubGlobal("fetch", fetchMock)
    fetchMock.mockImplementation((url: string, init?: RequestInit) => {
      if (url === "/api/categories") return json(200, { categories })
      if (url.startsWith("/api/transactions") && init?.method === "GET") return json(200, statement(rows))
      if (init?.method === "DELETE") return json(204)
      return json(404)
    })
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    fetchMock.mockReset()
  })

  it("lists transactions grouped by month, with Projections marked", async () => {
    rows = [
      item("1", "Cinema", "2026-02-03", { is_projection: true }),
      item("2", "Mercado", "2026-01-10"),
    ]
    renderScreen()

    const february = await screen.findByRole("list", { name: "Transações de fevereiro de 2026" })
    expect(within(february).getByText("Cinema")).toBeInTheDocument()
    expect(within(february).getByText("Projeção")).toBeInTheDocument()
    const january = screen.getByRole("list", { name: "Transações de janeiro de 2026" })
    expect(within(january).getByText("Mercado")).toBeInTheDocument()
    expect(within(january).queryByText("Projeção")).not.toBeInTheDocument()
  })

  it("invites the person to add the first transaction when there is none", async () => {
    renderScreen()

    expect(await screen.findByText("Nenhuma transação ainda")).toBeInTheDocument()
    expect(screen.getAllByRole("button", { name: "Nova transação" })).not.toHaveLength(0)
  })

  it("sends the type filter to the API and offers to clear filters when nothing matches", async () => {
    renderScreen()
    await screen.findByText("Nenhuma transação ainda")

    await userEvent.click(screen.getByRole("radio", { name: "Receitas" }))

    expect(await screen.findByText("Nenhuma transação com estes filtros")).toBeInTheDocument()
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining("type=income"),
      expect.anything(),
    )
    await userEvent.click(screen.getAllByRole("button", { name: "Limpar filtros" })[0])
    expect(await screen.findByText("Nenhuma transação ainda")).toBeInTheDocument()
  })

  it("asks for confirmation before deleting", async () => {
    rows = [item("1", "Mercado", "2026-01-10")]
    renderScreen()
    await screen.findByText("Mercado")

    await userEvent.click(screen.getByRole("button", { name: "Excluir Mercado" }))
    expect(fetchMock).not.toHaveBeenCalledWith(
      "/api/transactions/1",
      expect.objectContaining({ method: "DELETE" }),
    )
    const dialog = await screen.findByRole("alertdialog")
    await userEvent.click(within(dialog).getByRole("button", { name: "Excluir" }))

    expect(fetchMock).toHaveBeenCalledWith(
      "/api/transactions/1",
      expect.objectContaining({ method: "DELETE" }),
    )
  })

  it("asks for an amount greater than zero before calling the API", async () => {
    renderScreen()
    await screen.findByText("Nenhuma transação ainda")

    await userEvent.click(screen.getAllByRole("button", { name: "Nova transação" })[0])
    await userEvent.type(screen.getByLabelText("Descrição"), "Teste")
    await userEvent.click(screen.getByRole("button", { name: "Lançar" }))

    expect(await screen.findByText("Digite um valor maior que zero.")).toBeInTheDocument()
    expect(
      fetchMock.mock.calls.some(([, init]) => (init as RequestInit | undefined)?.method === "POST"),
    ).toBe(false)
  })
})
