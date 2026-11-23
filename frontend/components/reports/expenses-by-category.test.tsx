import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { render, screen, within } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { ExpensesByCategory } from "@/components/reports/expenses-by-category"

// Recharts measures its container, which jsdom cannot do; the table carries
// the same values, so the tests read the table and leave the chart out.
vi.mock("@/components/reports/category-chart", () => ({ CategoryChart: () => null }))

function json(status: number, body?: unknown) {
  return Promise.resolve(
    new Response(body === undefined ? null : JSON.stringify(body), {
      status,
      headers: { "Content-Type": "application/json" },
    }),
  )
}

function renderSection() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return render(
    <QueryClientProvider client={client}>
      <ExpensesByCategory />
    </QueryClientProvider>,
  )
}

const REPORT = {
  from: "2026-02",
  to: "2026-02",
  total_cents: 144251,
  projected_cents: 104000,
  categories: [
    { category_id: "c1", name: "Moradia", active: true, total_cents: 100000, projected_cents: 100000 },
    { category_id: "c2", name: "Alimentação", active: true, total_cents: 24000, projected_cents: 4000 },
    { category_id: "c3", name: "Lazer", active: true, total_cents: 19001, projected_cents: 0 },
    { category_id: "c4", name: "Juros e encargos", active: false, total_cents: 1250, projected_cents: 0 },
  ],
}

describe("ExpensesByCategory", () => {
  const fetchMock = vi.fn()

  beforeEach(() => {
    vi.stubGlobal("fetch", fetchMock)
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    fetchMock.mockReset()
  })

  it("asks for the current month by default", async () => {
    fetchMock.mockImplementation(() => json(200, REPORT))
    renderSection()

    await screen.findByText(/Total de despesas em/)
    const url = String(fetchMock.mock.calls[0][0])
    expect(url).toMatch(/^\/api\/reports\/expenses-by-category\?from=(\d{4}-\d{2})&to=\1$/)
    expect(screen.getByLabelText("Período")).toHaveTextContent("Este mês")
  })

  it("shows the total and, in a table, each Category with its share and Projection", async () => {
    fetchMock.mockImplementation(() => json(200, REPORT))
    renderSection()
    await userEvent.click(await screen.findByText("Ver os valores em tabela"))

    const table = await screen.findByRole("table")
    const rows = within(table).getAllByRole("row").slice(1)
    const text = rows.map((row) =>
      within(row)
        .getAllByRole("cell")
        .map((cell) => cell.textContent?.replace(/\s+/g, " ").trim())
        .join(" | "),
    )

    expect(text).toEqual([
      "Moradia | R$ 1.000,00 | 69,3% | R$ 1.000,00",
      "Alimentação | R$ 240,00 | 16,6% | R$ 40,00",
      "Lazer | R$ 190,01 | 13,2% | —",
      "Juros e encargosDesativada | R$ 12,50 | 0,9% | —",
      "Total | R$ 1.442,51 | 100% | R$ 1.040,00",
    ])
    expect(screen.getByText(/de projeção \(datas depois de hoje\)/)).toHaveTextContent(
      "Inclui R$ 1.040,00 de projeção",
    )
  })

  it("says so when the period has no expenses", async () => {
    fetchMock.mockImplementation(() =>
      json(200, { from: "2026-02", to: "2026-02", total_cents: 0, projected_cents: 0, categories: [] }),
    )
    renderSection()

    expect(await screen.findByText(/Nenhuma despesa em/)).toBeInTheDocument()
    expect(screen.queryByRole("table")).not.toBeInTheDocument()
  })

  it("offers to try again when the report cannot be loaded", async () => {
    fetchMock.mockImplementation(() => json(500, { error: { code: "internal_error", message: "" } }))
    renderSection()

    expect(
      await screen.findByText(/Não foi possível carregar as despesas por categoria/),
    ).toBeInTheDocument()
    expect(screen.getByRole("button", { name: "Tentar de novo" })).toBeInTheDocument()
  })
})
