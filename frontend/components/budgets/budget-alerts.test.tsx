import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { render, screen } from "@testing-library/react"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { BudgetAlerts, BudgetsShortcut } from "@/components/budgets/budget-alerts"

function json(status: number, body?: unknown) {
  return Promise.resolve(
    new Response(body === undefined ? null : JSON.stringify(body), {
      status,
      headers: { "Content-Type": "application/json" },
    }),
  )
}

function line(id: string, name: string, limit: number, spent: number, active = true) {
  return {
    category: { id, name, type: "expense", active },
    limit_cents: limit,
    spent_cents: spent,
    remaining_cents: limit - spent,
    percent_used: Math.floor((spent * 100) / limit),
    exceeded: active && spent > limit,
  }
}

function renderWithClient(ui: React.ReactNode) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return render(<QueryClientProvider client={client}>{ui}</QueryClientProvider>)
}

describe("budget alerts on the home page", () => {
  const fetchMock = vi.fn()

  function answerWith(budgets: ReturnType<typeof line>[]) {
    fetchMock.mockImplementation((url: string) =>
      url === "/api/budgets/status"
        ? json(200, {
            month: "2026-11",
            exceeded_count: budgets.filter((b) => b.exceeded).length,
            budgets,
          })
        : json(404),
    )
  }

  beforeEach(() => {
    vi.stubGlobal("fetch", fetchMock)
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    fetchMock.mockReset()
  })

  it("says how far over the limit each category is, in words, the furthest first", async () => {
    answerWith([
      line("c1", "Alimentação", 100000, 101000),
      line("c2", "Lazer", 40000, 52000),
      line("c3", "Transporte", 30000, 10000),
    ])
    renderWithClient(<BudgetAlerts />)

    const alerts = await screen.findAllByRole("alert")
    expect(alerts).toHaveLength(2)
    expect(alerts[0]).toHaveTextContent("Você passou R$ 120,00 do limite de Lazer.")
    expect(alerts[0]).toHaveTextContent("Gasto de R$ 520,00 em R$ 400,00 de limite em novembro de 2026.")
    expect(alerts[1]).toHaveTextContent("Você passou R$ 10,00 do limite de Alimentação.")
    expect(screen.getByRole("link", { name: "Ver orçamentos" })).toHaveAttribute(
      "href",
      "/orcamentos",
    )
  })

  it("shows nothing when no category is over its limit", async () => {
    answerWith([line("c1", "Alimentação", 100000, 100000), line("c2", "Lazer", 40000, 1000)])
    renderWithClient(<BudgetAlerts />)

    await vi.waitFor(() => expect(fetchMock).toHaveBeenCalled())
    expect(screen.queryByRole("alert")).not.toBeInTheDocument()
  })

  it("raises no alert for a deactivated category", async () => {
    answerWith([line("c4", "Educação", 20000, 30000, false)])
    renderWithClient(<BudgetAlerts />)

    await vi.waitFor(() => expect(fetchMock).toHaveBeenCalled())
    expect(screen.queryByRole("alert")).not.toBeInTheDocument()
  })

  it("spells out three alerts and counts the rest", async () => {
    answerWith(
      ["A", "B", "C", "D", "E"].map((name, index) => line(`c${index}`, name, 1000, 2000)),
    )
    renderWithClient(<BudgetAlerts />)

    expect(await screen.findAllByRole("alert")).toHaveLength(3)
    expect(screen.getByText(/E mais 2 categorias acima do limite/)).toBeInTheDocument()
  })

  it("puts the number of categories over their limit on the shortcut", async () => {
    answerWith([line("c1", "Alimentação", 100000, 101000), line("c2", "Lazer", 40000, 52000)])
    renderWithClient(<BudgetsShortcut className="shortcut" />)

    expect(await screen.findByText("2 acima do limite")).toBeInTheDocument()
    expect(screen.getByRole("link", { name: /Orçamentos/ })).toHaveAttribute("href", "/orcamentos")
  })

  it("has no badge on the shortcut without alerts", async () => {
    answerWith([line("c1", "Alimentação", 100000, 1000)])
    renderWithClient(<BudgetsShortcut className="shortcut" />)

    await vi.waitFor(() => expect(fetchMock).toHaveBeenCalled())
    expect(screen.queryByText(/acima do limite/)).not.toBeInTheDocument()
  })
})
