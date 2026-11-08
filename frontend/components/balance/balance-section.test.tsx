import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { render, screen, within } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { BalanceSection } from "@/components/balance/balance-section"

vi.mock("sonner", () => ({ toast: { success: vi.fn(), error: vi.fn() } }))

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
  return {
    client,
    ...render(
      <QueryClientProvider client={client}>
        <BalanceSection />
      </QueryClientProvider>,
    ),
  }
}

/**
 * A stand-in for the API's two balance routes. Like the real one, it takes
 * the Opening Balance once and then answers 409.
 */
function fakeBalanceApi(initial: { current: number; opening: number | null }) {
  const state = { ...initial }
  return (url: string, init?: RequestInit) => {
    const payload = () => ({
      current_balance_cents: state.current,
      opening_balance_set: state.opening !== null,
      opening_balance_cents: state.opening,
    })
    if (url === "/api/balance" && init?.method === "GET") return json(200, payload())
    if (url === "/api/balance/opening" && init?.method === "POST") {
      if (state.opening !== null) {
        return json(409, { error: { code: "opening_balance_already_set", message: "" } })
      }
      const sent = JSON.parse(String(init.body)).amount_cents as number
      state.opening = sent
      state.current += sent
      return json(201, payload())
    }
    return json(404)
  }
}

describe("BalanceSection", () => {
  const fetchMock = vi.fn()

  beforeEach(() => {
    vi.stubGlobal("fetch", fetchMock)
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    fetchMock.mockReset()
  })

  it("shows the current balance in reais, converted from cents", async () => {
    fetchMock.mockImplementation(fakeBalanceApi({ current: 118040, opening: null }))
    renderSection()

    expect(await screen.findByText("R$ 1.180,40")).toBeInTheDocument()
    expect(screen.getByText("Saldo atual")).toBeInTheDocument()
  })

  it("shows a negative balance with the minus sign in the expense colour", async () => {
    fetchMock.mockImplementation(fakeBalanceApi({ current: -15000, opening: null }))
    renderSection()

    expect(await screen.findByText("− R$ 150,00")).toHaveClass("text-expense")
  })

  it("offers the Opening Balance while none was informed", async () => {
    fetchMock.mockImplementation(fakeBalanceApi({ current: 0, opening: null }))
    renderSection()

    expect(await screen.findByRole("button", { name: "Informar saldo inicial" })).toBeVisible()
  })

  it("hides the offer and shows the Opening Balance once informed", async () => {
    fetchMock.mockImplementation(fakeBalanceApi({ current: 50000, opening: 20000 }))
    renderSection()

    expect(await screen.findByText(/Saldo inicial informado/)).toBeInTheDocument()
    expect(screen.getByText("R$ 200,00")).toBeInTheDocument()
    expect(screen.queryByRole("button", { name: "Informar saldo inicial" })).not.toBeInTheDocument()
  })

  it("sends the typed amount in cents, then shows the new balance and the informed value", async () => {
    fetchMock.mockImplementation(fakeBalanceApi({ current: 30000, opening: null }))
    renderSection()
    await userEvent.click(await screen.findByRole("button", { name: "Informar saldo inicial" }))

    const dialog = await screen.findByRole("dialog")
    await userEvent.type(within(dialog).getByLabelText("Saldo inicial"), "1.500,00")
    await userEvent.click(within(dialog).getByRole("button", { name: "Salvar saldo inicial" }))

    expect(fetchMock).toHaveBeenCalledWith(
      "/api/balance/opening",
      expect.objectContaining({ method: "POST", body: JSON.stringify({ amount_cents: 150000 }) }),
    )
    expect(await screen.findByText("R$ 1.800,00")).toBeInTheDocument()
    expect(screen.getByText(/Saldo inicial informado/)).toBeInTheDocument()
    expect(screen.queryByRole("button", { name: "Informar saldo inicial" })).not.toBeInTheDocument()
  })

  it("accepts a negative Opening Balance", async () => {
    fetchMock.mockImplementation(fakeBalanceApi({ current: 0, opening: null }))
    renderSection()
    await userEvent.click(await screen.findByRole("button", { name: "Informar saldo inicial" }))

    const dialog = await screen.findByRole("dialog")
    await userEvent.type(within(dialog).getByLabelText("Saldo inicial"), "-200,00")
    await userEvent.click(within(dialog).getByRole("button", { name: "Salvar saldo inicial" }))

    expect(fetchMock).toHaveBeenCalledWith(
      "/api/balance/opening",
      expect.objectContaining({ body: JSON.stringify({ amount_cents: -20000 }) }),
    )
    // The balance and the informed Opening Balance are both -R$ 200,00.
    expect(await screen.findByText(/Saldo inicial informado/)).toBeInTheDocument()
    expect(screen.getAllByText("− R$ 200,00")).toHaveLength(2)
  })

  it("refuses text that is not an amount without calling the API", async () => {
    fetchMock.mockImplementation(fakeBalanceApi({ current: 0, opening: null }))
    renderSection()
    await userEvent.click(await screen.findByRole("button", { name: "Informar saldo inicial" }))

    const dialog = await screen.findByRole("dialog")
    await userEvent.type(within(dialog).getByLabelText("Saldo inicial"), "abc")
    await userEvent.click(within(dialog).getByRole("button", { name: "Salvar saldo inicial" }))

    expect(await within(dialog).findByText(/Digite um valor/)).toBeInTheDocument()
    expect(fetchMock).not.toHaveBeenCalledWith("/api/balance/opening", expect.anything())
  })

  it("explains when the Opening Balance was already informed elsewhere", async () => {
    const api = fakeBalanceApi({ current: 0, opening: null })
    fetchMock.mockImplementation((url: string, init?: RequestInit) =>
      url === "/api/balance/opening"
        ? json(409, { error: { code: "opening_balance_already_set", message: "" } })
        : api(url, init),
    )
    renderSection()
    await userEvent.click(await screen.findByRole("button", { name: "Informar saldo inicial" }))

    const dialog = await screen.findByRole("dialog")
    await userEvent.type(within(dialog).getByLabelText("Saldo inicial"), "10")
    await userEvent.click(within(dialog).getByRole("button", { name: "Salvar saldo inicial" }))

    expect(await within(dialog).findByText(/já foi informado/)).toBeInTheDocument()
  })

  it("offers to try again when the balance cannot be loaded", async () => {
    fetchMock.mockImplementation(() => json(500, { error: { code: "internal_error" } }))
    renderSection()

    expect(await screen.findByText(/Não foi possível carregar o saldo/)).toBeInTheDocument()
    expect(screen.getByRole("button", { name: "Tentar de novo" })).toBeVisible()
  })
})
