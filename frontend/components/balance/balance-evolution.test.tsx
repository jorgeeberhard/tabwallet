import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { render, screen, within } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { BalanceEvolution } from "@/components/balance/balance-evolution"

// Recharts measures its container, which jsdom cannot do; the table carries
// the same values, so the tests read the table and leave the chart out.
vi.mock("@/components/balance/balance-chart", () => ({ BalanceChart: () => null }))

function json(status: number, body?: unknown) {
  return Promise.resolve(
    new Response(body === undefined ? null : JSON.stringify(body), {
      status,
      headers: { "Content-Type": "application/json" },
    }),
  )
}

function renderScreen() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return render(
    <QueryClientProvider client={client}>
      <BalanceEvolution />
    </QueryClientProvider>,
  )
}

/** A stand-in for the two series routes: today is 2026-01-15. */
function fakeSeriesApi(url: string) {
  if (url.startsWith("/api/balance/history")) {
    return json(200, {
      granularity: "month",
      from: "2025-12-01",
      to: "2026-01-15",
      points: [
        { date: "2025-12-31", balance_cents: 100000 },
        { date: "2026-01-15", balance_cents: 118040 },
      ],
    })
  }
  if (url.startsWith("/api/balance/projection")) {
    return json(200, {
      granularity: "month",
      today: "2026-01-15",
      to: "2026-03-31",
      current_balance_cents: 118040,
      points: [
        { date: "2026-01-31", balance_cents: 90000 },
        { date: "2026-02-28", balance_cents: -12050 },
        { date: "2026-03-31", balance_cents: 250000 },
      ],
    })
  }
  return json(404)
}

describe("BalanceEvolution", () => {
  const fetchMock = vi.fn()

  beforeEach(() => {
    vi.stubGlobal("fetch", fetchMock)
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    fetchMock.mockReset()
  })

  it("asks for the monthly series and shows the balances converted to reais", async () => {
    fetchMock.mockImplementation((url: string) => fakeSeriesApi(url))
    renderScreen()

    expect(await screen.findByText("Saldo atual")).toBeInTheDocument()
    const current = screen.getByText("Saldo atual").closest("[data-slot=card]") as HTMLElement
    expect(within(current).getByText("R$ 1.180,40")).toBeInTheDocument()
    const requested = fetchMock.mock.calls.map(([url]) => String(url))
    expect(requested.some((url) => /history\?granularity=month&from=\d{4}-\d{2}-01/.test(url))).toBe(
      true,
    )
    expect(requested.some((url) => /projection\?granularity=month&to=\d{4}-\d{2}-\d{2}/.test(url))).toBe(
      true,
    )
  })

  it("lists history and projected months in the table, telling them apart in words", async () => {
    fetchMock.mockImplementation((url: string) => fakeSeriesApi(url))
    renderScreen()
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
      "dezembro de 2025 | Histórico | R$ 1.000,00",
      "janeiro de 2026 (até hoje) | Histórico | R$ 1.180,40",
      "janeiro de 2026 (fim do mês) | Projetado | R$ 900,00",
      "fevereiro de 2026 | Projetado | − R$ 120,50",
      "março de 2026 | Projetado | R$ 2.500,00",
    ])
  })

  it("points out the lowest projected balance", async () => {
    fetchMock.mockImplementation((url: string) => fakeSeriesApi(url))
    renderScreen()

    const card = (await screen.findByText("Menor saldo projetado")).closest("[data-slot=card]")
    expect(card).not.toBeNull()
    expect(within(card as HTMLElement).getByText("− R$ 120,50")).toBeInTheDocument()
    expect(within(card as HTMLElement).getByText("no fim de fevereiro de 2026")).toBeInTheDocument()
  })

  it("offers to try again when the series cannot be loaded", async () => {
    fetchMock.mockImplementation(() => json(500, { error: { code: "internal_error", message: "" } }))
    renderScreen()

    expect(await screen.findByText(/Não foi possível carregar a evolução do saldo/)).toBeInTheDocument()
    expect(screen.getByRole("button", { name: "Tentar de novo" })).toBeInTheDocument()
  })
})
