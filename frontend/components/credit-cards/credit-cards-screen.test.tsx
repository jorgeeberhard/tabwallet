import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { render, screen, within } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { CreditCardsScreen } from "@/components/credit-cards/credit-cards-screen"

const toast = vi.hoisted(() => ({ success: vi.fn(), error: vi.fn() }))
vi.mock("sonner", () => ({ toast }))

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
      <CreditCardsScreen />
    </QueryClientProvider>,
  )
}

describe("CreditCardsScreen", () => {
  const fetchMock = vi.fn()
  let cards: unknown[]

  beforeEach(() => {
    cards = []
    vi.stubGlobal("fetch", fetchMock)
    fetchMock.mockImplementation((url: string, init?: RequestInit) => {
      if (url === "/api/credit-cards" && init?.method === "GET") {
        return json(200, { credit_cards: cards })
      }
      if (url === "/api/credit-cards" && init?.method === "POST") {
        const body = JSON.parse(init.body as string)
        return json(201, {
          credit_card: { id: "k2", active: true, ...body },
        })
      }
      if (url.endsWith("/deactivate")) {
        return json(409, {
          error: { code: "credit_card_has_remaining_invoice", message: "x" },
        })
      }
      return json(404)
    })
    toast.success.mockReset()
    toast.error.mockReset()
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    fetchMock.mockReset()
  })

  it("lists active and deactivated cards with their closing and due days", async () => {
    cards = [
      { id: "k1", name: "Nubank", closing_day: 10, due_day: 17, active: true },
      { id: "k2", name: "Itaú", closing_day: 25, due_day: 5, active: false },
    ]
    renderScreen()

    const active = await screen.findByRole("list", { name: "Cartões ativos" })
    expect(within(active).getByText("Nubank")).toBeInTheDocument()
    expect(within(active).getByText("Fecha dia 10 · vence dia 17")).toBeInTheDocument()
    const inactive = screen.getByRole("list", { name: "Cartões desativados" })
    expect(within(inactive).getByText("Itaú")).toBeInTheDocument()
    expect(within(inactive).getByRole("button", { name: "Reativar Itaú" })).toBeInTheDocument()
  })

  it("invites the person to register the first card", async () => {
    renderScreen()

    expect(await screen.findByText("Nenhum cartão ativo")).toBeInTheDocument()
  })

  it("registers a card with its closing and due days", async () => {
    renderScreen()
    await screen.findByText("Nenhum cartão ativo")

    await userEvent.click(screen.getAllByRole("button", { name: "Novo cartão" })[0])
    await userEvent.type(screen.getByLabelText("Nome"), "Nubank")
    await userEvent.type(screen.getByLabelText("Dia de fechamento"), "10")
    await userEvent.type(screen.getByLabelText("Dia de vencimento"), "17")
    await userEvent.click(screen.getByRole("button", { name: "Cadastrar cartão" }))

    const post = fetchMock.mock.calls.find(([, init]) => init?.method === "POST")
    expect(JSON.parse(post?.[1].body)).toEqual({ name: "Nubank", closing_day: 10, due_day: 17 })
    expect(toast.success).toHaveBeenCalledWith("Cartão cadastrado.")
  })

  it("rejects a day outside 1 to 31 before calling the API", async () => {
    renderScreen()
    await screen.findByText("Nenhum cartão ativo")

    await userEvent.click(screen.getAllByRole("button", { name: "Novo cartão" })[0])
    await userEvent.type(screen.getByLabelText("Nome"), "Nubank")
    await userEvent.type(screen.getByLabelText("Dia de fechamento"), "32")
    await userEvent.type(screen.getByLabelText("Dia de vencimento"), "17")
    await userEvent.click(screen.getByRole("button", { name: "Cadastrar cartão" }))

    expect(await screen.findByText("Escolha um dia de 1 a 31.")).toBeInTheDocument()
    expect(fetchMock.mock.calls.some(([, init]) => init?.method === "POST")).toBe(false)
  })

  it("explains why a card with an invoice to pay cannot be deactivated", async () => {
    cards = [{ id: "k1", name: "Nubank", closing_day: 10, due_day: 17, active: true }]
    renderScreen()

    await userEvent.click(await screen.findByRole("button", { name: "Desativar Nubank" }))

    await vi.waitFor(() =>
      expect(toast.error).toHaveBeenCalledWith(
        "Este cartão ainda tem fatura com valor a pagar. Ele só pode ser desativado quando todas estiverem pagas.",
      ),
    )
  })
})
