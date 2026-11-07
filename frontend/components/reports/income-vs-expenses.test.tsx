import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { render, screen, within } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { IncomeVsExpenses } from "@/components/reports/income-vs-expenses"
import { shortMonth } from "@/components/reports/income-expense-chart"

vi.mock("@/components/reports/income-expense-chart", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@/components/reports/income-expense-chart")>()),
  IncomeExpenseChart: () => null,
}))

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
      <IncomeVsExpenses />
    </QueryClientProvider>,
  )
}

const REPORT = {
  from: "2026-01",
  to: "2026-02",
  income_cents: 1000000,
  expense_cents: 247251,
  months: [
    {
      month: "2026-01",
      income_cents: 500000,
      expense_cents: 107000,
      balance_cents: 393000,
      projected_income_cents: 0,
      projected_expense_cents: 0,
    },
    {
      month: "2026-02",
      income_cents: 500000,
      expense_cents: 140251,
      balance_cents: 359749,
      projected_income_cents: 500000,
      projected_expense_cents: 100000,
    },
  ],
}

describe("IncomeVsExpenses", () => {
  const fetchMock = vi.fn()

  beforeEach(() => {
    vi.stubGlobal("fetch", fetchMock)
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    fetchMock.mockReset()
  })

  it("asks for the last six months by default", async () => {
    fetchMock.mockImplementation(() => json(200, REPORT))
    renderSection()

    await screen.findByText("Ver os valores em tabela")
    const url = String(fetchMock.mock.calls[0][0])
    expect(url).toMatch(/^\/api\/reports\/income-vs-expenses\?from=\d{4}-\d{2}&to=\d{4}-\d{2}$/)
    expect(screen.getByLabelText("Período")).toHaveTextContent("Últimos 6 meses")
  })

  it("lists income, expenses and the result of each month, marking the months with Projections", async () => {
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
      "janeiro de 2026 | R$ 5.000,00 | − R$ 1.070,00 | R$ 3.930,00",
      "fevereiro de 2026Com projeções | R$ 5.000,00 | − R$ 1.402,51 | R$ 3.597,49",
      "Total do período | R$ 10.000,00 | − R$ 2.472,51 | R$ 7.527,49",
    ])
  })

  it("says so when nothing was recorded in the period", async () => {
    fetchMock.mockImplementation(() =>
      json(200, {
        from: "2026-01",
        to: "2026-01",
        income_cents: 0,
        expense_cents: 0,
        months: [
          {
            month: "2026-01",
            income_cents: 0,
            expense_cents: 0,
            balance_cents: 0,
            projected_income_cents: 0,
            projected_expense_cents: 0,
          },
        ],
      }),
    )
    renderSection()

    expect(await screen.findByText(/Nada lançado em/)).toBeInTheDocument()
  })

  it("labels the months of the axis in short Portuguese", () => {
    expect(shortMonth("2026-01")).toBe("jan/26")
    expect(shortMonth("2025-12")).toBe("dez/25")
  })
})
