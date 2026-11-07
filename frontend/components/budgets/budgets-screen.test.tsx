import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { render, screen, within } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { BudgetsScreen } from "@/components/budgets/budgets-screen"

vi.mock("sonner", () => ({ toast: { success: vi.fn(), error: vi.fn() } }))
vi.mock("@/lib/use-today", () => ({ useToday: () => "2026-11-07" }))

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
      <BudgetsScreen />
    </QueryClientProvider>,
  )
}

const categories = [
  { id: "c1", name: "Alimentação", type: "expense", active: true, is_charges: false },
  { id: "c2", name: "Lazer", type: "expense", active: true, is_charges: false },
  { id: "c3", name: "Transporte", type: "expense", active: true, is_charges: false },
  { id: "c4", name: "Educação", type: "expense", active: false, is_charges: false },
  { id: "c5", name: "Salário", type: "income", active: true, is_charges: false },
]

function line(
  id: string,
  name: string,
  limit: number,
  spent: number,
  overrides: { active?: boolean; exceeded?: boolean } = {},
) {
  const active = overrides.active ?? true
  return {
    category: { id, name, type: "expense", active },
    limit_cents: limit,
    spent_cents: spent,
    remaining_cents: limit - spent,
    percent_used: Math.floor((spent * 100) / limit),
    exceeded: overrides.exceeded ?? (active && spent > limit),
  }
}

describe("BudgetsScreen", () => {
  const fetchMock = vi.fn()
  let lines: ReturnType<typeof line>[]

  beforeEach(() => {
    lines = [
      line("c1", "Alimentação", 100000, 41000),
      line("c2", "Lazer", 40000, 52000),
      line("c4", "Educação", 20000, 30000, { active: false }),
    ]
    vi.stubGlobal("fetch", fetchMock)
    fetchMock.mockImplementation((url: string, init?: RequestInit) => {
      if (url === "/api/categories") return json(200, { categories })
      if (url.startsWith("/api/budgets/status")) {
        return json(200, {
          month: url.split("month=")[1],
          exceeded_count: lines.filter((l) => l.exceeded).length,
          budgets: lines,
        })
      }
      if (url === "/api/budgets/c3" && init?.method === "PUT") {
        return json(200, {
          budget: {
            category: { id: "c3", name: "Transporte", type: "expense", active: true },
            limit_cents: JSON.parse(init.body as string).limit_cents,
            effective_from: "2026-11",
          },
        })
      }
      if (url.startsWith("/api/budgets/") && init?.method === "DELETE") return json(204)
      return json(404)
    })
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    fetchMock.mockReset()
  })

  it("shows each budget against its spending, the exceeded one first, for the current month", async () => {
    renderScreen()

    const list = await screen.findByRole("list", { name: "Orçamentos do mês" })
    const rows = within(list).getAllByRole("listitem")
    expect(rows).toHaveLength(2)
    expect(within(rows[0]).getByText("Lazer")).toBeInTheDocument()
    expect(within(rows[0]).getByText("Limite excedido em R$ 120,00")).toBeInTheDocument()
    expect(within(rows[1]).getByText("Alimentação")).toBeInTheDocument()
    expect(within(rows[1]).getByText("41% usado")).toBeInTheDocument()
    expect(screen.getByText("novembro de 2026")).toBeInTheDocument()
    expect(fetchMock).toHaveBeenCalledWith(
      "/api/budgets/status?month=2026-11",
      expect.anything(),
    )
  })

  it("offers a limit only to active expense categories that have none", async () => {
    renderScreen()

    const without = await screen.findByRole("list", { name: "Categorias sem orçamento" })
    expect(within(without).getByText("Transporte")).toBeInTheDocument()
    expect(within(without).queryByText("Salário")).not.toBeInTheDocument()
    expect(within(without).queryByText("Educação")).not.toBeInTheDocument()
    expect(within(without).queryByText("Lazer")).not.toBeInTheDocument()
  })

  it("keeps the budget of a deactivated category, marked and without an alert", async () => {
    renderScreen()

    const paused = await screen.findByRole("list", {
      name: "Orçamentos de categorias desativadas",
    })
    expect(within(paused).getByText("Educação")).toBeInTheDocument()
    expect(within(paused).getByText("Desativada")).toBeInTheDocument()
    expect(screen.getByText(/não gera alerta/)).toBeInTheDocument()
  })

  it("sets a limit in cents for a category and tells it applies from the current month", async () => {
    renderScreen()
    await screen.findByRole("list", { name: "Categorias sem orçamento" })

    await userEvent.click(screen.getByRole("button", { name: "Definir orçamento de Transporte" }))
    const dialog = await screen.findByRole("dialog")
    expect(within(dialog).getByText(/a partir de novembro de 2026/)).toBeInTheDocument()
    await userEvent.type(within(dialog).getByLabelText("Limite por mês"), "350,50")
    await userEvent.click(within(dialog).getByRole("button", { name: "Salvar orçamento" }))

    expect(fetchMock).toHaveBeenCalledWith(
      "/api/budgets/c3",
      expect.objectContaining({ method: "PUT", body: JSON.stringify({ limit_cents: 35050 }) }),
    )
  })

  it("asks for a positive limit before calling the API", async () => {
    renderScreen()
    await screen.findByRole("list", { name: "Categorias sem orçamento" })

    await userEvent.click(screen.getByRole("button", { name: "Definir orçamento de Transporte" }))
    const dialog = await screen.findByRole("dialog")
    await userEvent.click(within(dialog).getByRole("button", { name: "Salvar orçamento" }))

    expect(await within(dialog).findByText("Digite um limite maior que zero.")).toBeInTheDocument()
    expect(fetchMock).not.toHaveBeenCalledWith(
      "/api/budgets/c3",
      expect.objectContaining({ method: "PUT" }),
    )
  })

  it("starts the dialog with the current limit when changing a budget", async () => {
    renderScreen()
    await screen.findByRole("list", { name: "Orçamentos do mês" })

    await userEvent.click(screen.getByRole("button", { name: "Alterar orçamento de Lazer" }))
    const dialog = await screen.findByRole("dialog")
    expect(within(dialog).getByLabelText("Limite por mês")).toHaveValue("400,00")
  })

  it("removes a budget only after confirming", async () => {
    renderScreen()
    await screen.findByRole("list", { name: "Orçamentos do mês" })

    await userEvent.click(screen.getByRole("button", { name: "Remover orçamento de Lazer" }))
    const dialog = await screen.findByRole("alertdialog")
    expect(dialog).toHaveTextContent("Os meses anteriores continuam com o limite que tinham")
    expect(fetchMock).not.toHaveBeenCalledWith(
      "/api/budgets/c2",
      expect.objectContaining({ method: "DELETE" }),
    )
    await userEvent.click(within(dialog).getByRole("button", { name: "Remover" }))

    expect(fetchMock).toHaveBeenCalledWith(
      "/api/budgets/c2",
      expect.objectContaining({ method: "DELETE" }),
    )
  })

  it("looks at another month and says that changes apply from the current one", async () => {
    renderScreen()
    await screen.findByRole("list", { name: "Orçamentos do mês" })

    await userEvent.click(screen.getByRole("button", { name: "Mês anterior" }))

    expect(await screen.findByText("outubro de 2026")).toBeInTheDocument()
    expect(fetchMock).toHaveBeenCalledWith(
      "/api/budgets/status?month=2026-10",
      expect.anything(),
    )
    expect(screen.getByText(/Um mês passado mostra o limite que ele tinha/)).toBeInTheDocument()
    await userEvent.click(screen.getByRole("button", { name: "Mês atual" }))
    expect(await screen.findByText("novembro de 2026")).toBeInTheDocument()
  })

  it("does not look further than twelve months ahead", async () => {
    renderScreen()
    await screen.findByRole("list", { name: "Orçamentos do mês" })

    const next = screen.getByRole("button", { name: "Próximo mês" })
    for (let i = 0; i < 12; i++) await userEvent.click(next)

    expect(await screen.findByText("novembro de 2027")).toBeInTheDocument()
    expect(next).toBeDisabled()
  })
})
