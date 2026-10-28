import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { render, screen, within } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { RecurringScreen } from "@/components/recurring/recurring-screen"

vi.mock("sonner", () => ({ toast: { success: vi.fn(), error: vi.fn() } }))

function json(status: number, body?: unknown) {
  return Promise.resolve(
    new Response(body === undefined ? null : JSON.stringify(body), {
      status,
      headers: { "Content-Type": "application/json" },
    }),
  )
}

function recurring(id: string, description: string, overrides = {}) {
  return {
    id,
    type: "expense",
    amount_cents: 150000,
    day_of_month: 5,
    description,
    category: { id: "c1", name: "Moradia", type: "expense", active: true },
    start_month: "2026-01",
    end_month: null,
    ended: false,
    next_date: "2026-02-05",
    ...overrides,
  }
}

function renderScreen() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return render(
    <QueryClientProvider client={client}>
      <RecurringScreen />
    </QueryClientProvider>,
  )
}

describe("RecurringScreen", () => {
  const fetchMock = vi.fn()
  const categories = [
    { id: "c1", name: "Moradia", type: "expense", active: true, is_charges: false },
    { id: "c2", name: "Salário", type: "income", active: true, is_charges: false },
  ]
  let rows: unknown[]

  beforeEach(() => {
    rows = []
    vi.stubGlobal("fetch", fetchMock)
    fetchMock.mockImplementation((url: string, init?: RequestInit) => {
      if (url === "/api/categories") return json(200, { categories })
      if (url === "/api/recurring-transactions" && init?.method === "GET")
        return json(200, { recurring_transactions: rows })
      if (url.endsWith("/end") && init?.method === "POST")
        return json(200, {
          recurring_transaction: recurring("r1", "Aluguel", { ended: true, end_month: "2026-01" }),
        })
      return json(404)
    })
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    fetchMock.mockReset()
  })

  it("invites the person to add the first fixed transaction when there is none", async () => {
    renderScreen()

    expect(await screen.findByText("Nenhum lançamento fixo ainda")).toBeInTheDocument()
  })

  it("lists the ones in progress with day and next date, and the ended ones apart", async () => {
    rows = [
      recurring("r1", "Aluguel"),
      recurring("r2", "Academia", { ended: true, end_month: "2026-01", next_date: null }),
    ]
    renderScreen()

    const active = await screen.findByRole("list", { name: "Lançamentos fixos em andamento" })
    expect(within(active).getByText("Aluguel")).toBeInTheDocument()
    expect(within(active).getByText("Todo dia 5")).toBeInTheDocument()
    expect(within(active).getByText("05/02/2026")).toBeInTheDocument()
    const ended = screen.getByRole("list", { name: "Lançamentos fixos encerrados" })
    expect(within(ended).getByText("Academia")).toBeInTheDocument()
    expect(within(ended).getByText("Encerrado em janeiro de 2026")).toBeInTheDocument()
    expect(within(ended).queryByRole("button", { name: "Encerrar Academia" })).not.toBeInTheDocument()
  })

  it("asks for confirmation before ending", async () => {
    rows = [recurring("r1", "Aluguel")]
    renderScreen()
    await screen.findByText("Aluguel")

    await userEvent.click(screen.getByRole("button", { name: "Encerrar Aluguel" }))
    expect(fetchMock).not.toHaveBeenCalledWith(
      "/api/recurring-transactions/r1/end",
      expect.anything(),
    )
    const dialog = await screen.findByRole("alertdialog")
    await userEvent.click(within(dialog).getByRole("button", { name: "Encerrar" }))

    expect(fetchMock).toHaveBeenCalledWith(
      "/api/recurring-transactions/r1/end",
      expect.objectContaining({ method: "POST" }),
    )
  })

  it("asks for a day between 1 and 31 and an amount before calling the API", async () => {
    renderScreen()
    await screen.findByText("Nenhum lançamento fixo ainda")

    await userEvent.click(screen.getAllByRole("button", { name: "Novo lançamento fixo" })[0])
    await userEvent.type(screen.getByLabelText("Descrição"), "Aluguel")
    await userEvent.type(screen.getByLabelText("Dia do mês"), "32")
    await userEvent.click(screen.getByRole("button", { name: "Cadastrar" }))

    expect(await screen.findByText("Escolha um dia do mês de 1 a 31.")).toBeInTheDocument()
    expect(screen.getByText("Digite um valor maior que zero.")).toBeInTheDocument()
    expect(
      fetchMock.mock.calls.some(([, init]) => (init as RequestInit | undefined)?.method === "POST"),
    ).toBe(false)
  })
})
