import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { render, screen } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { ChargeDialog } from "@/components/credit-cards/charge-dialog"
import type { Invoice } from "@/lib/credit-cards-api"

const toast = vi.hoisted(() => ({ success: vi.fn(), error: vi.fn() }))
vi.mock("sonner", () => ({ toast }))

const invoice: Invoice = {
  creditCardId: "k1",
  closingMonth: "2026-01",
  closingDate: "2026-01-20",
  dueDate: "2026-01-27",
  referenceMonth: "2026-01",
  state: "closed",
  totalCents: 30000,
  total: 300,
  paidCents: 0,
  paid: 0,
  remainingCents: 30000,
  remaining: 300,
  chargesCents: 0,
  carriedBalanceCents: 0,
  carriedForwardCents: 0,
  paymentCorrection: { state: "not_due", correctedCents: null, maxCents: 30000 },
}

function json(status: number, body?: unknown) {
  return Promise.resolve(
    new Response(body === undefined ? null : JSON.stringify(body), {
      status,
      headers: { "Content-Type": "application/json" },
    }),
  )
}

const created = {
  charge: {
    id: "c1",
    amount_cents: 1250,
    date: "2026-01-22",
    description: "Juros e encargos",
    category: { id: "cat", name: "Juros e encargos", type: "expense", active: true },
    credit_card: { id: "k1", name: "Nubank", active: true },
    invoice: {
      closing_month: "2026-01",
      closing_date: "2026-01-20",
      due_date: "2026-01-27",
      reference_month: "2026-01",
      state: "closed",
    },
    reference_month: "2026-01",
    is_projection: false,
  },
}

describe("ChargeDialog", () => {
  const fetchMock = vi.fn()
  const onClose = vi.fn()

  beforeEach(() => {
    vi.stubGlobal("fetch", fetchMock)
    onClose.mockReset()
    toast.success.mockReset()
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    fetchMock.mockReset()
  })

  function renderDialog() {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    render(
      <QueryClientProvider client={client}>
        <ChargeDialog
          mode={{ kind: "create", cardId: "k1", cardName: "Nubank", invoice }}
          onClose={onClose}
        />
      </QueryClientProvider>,
    )
  }

  it("records the amount in cents on the invoice and says the system does not calculate interest", async () => {
    fetchMock.mockImplementation(() => json(201, created))
    renderDialog()

    expect(screen.getByText(/O sistema não calcula juros/)).toBeInTheDocument()
    await userEvent.type(screen.getByLabelText("Valor cobrado"), "12,50")
    await userEvent.click(screen.getByRole("button", { name: "Lançar encargos" }))

    await vi.waitFor(() => expect(onClose).toHaveBeenCalled())
    expect(fetchMock).toHaveBeenCalledWith(
      "/api/credit-cards/k1/invoices/2026-01/charges",
      expect.objectContaining({ method: "POST" }),
    )
    const body = JSON.parse(fetchMock.mock.calls[0][1].body as string)
    expect(body.amount_cents).toBe(1250)
    expect(toast.success).toHaveBeenCalled()
  })

  it("needs an amount greater than zero before calling the API", async () => {
    renderDialog()

    await userEvent.click(screen.getByRole("button", { name: "Lançar encargos" }))

    expect(screen.getByText("Digite um valor maior que zero.")).toBeInTheDocument()
    expect(fetchMock).not.toHaveBeenCalled()
  })

  it("tells the person to reactivate the category when it is deactivated", async () => {
    fetchMock.mockImplementation(() =>
      json(422, {
        error: {
          code: "validation_failed",
          message: "invalid",
          fields: { category_id: "category_inactive" },
        },
      }),
    )
    renderDialog()

    await userEvent.type(screen.getByLabelText("Valor cobrado"), "5")
    await userEvent.click(screen.getByRole("button", { name: "Lançar encargos" }))

    expect(await screen.findByText(/está desativada. Reative-a em Categorias/)).toBeInTheDocument()
    expect(onClose).not.toHaveBeenCalled()
  })
})
