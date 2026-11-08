import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { render, screen } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { CorrectPaymentDialog } from "@/components/credit-cards/correct-payment-dialog"
import type { Invoice } from "@/lib/credit-cards-api"

const toast = vi.hoisted(() => ({ success: vi.fn(), error: vi.fn() }))
vi.mock("sonner", () => ({ toast }))

const invoice: Invoice = {
  creditCardId: "k1",
  closingMonth: "2026-01",
  closingDate: "2026-01-20",
  dueDate: "2026-01-27",
  referenceMonth: "2026-01",
  state: "paid",
  totalCents: 30000,
  total: 300,
  paidCents: 30000,
  paid: 300,
  remainingCents: 0,
  remaining: 0,
  chargesCents: 0,
  carriedBalanceCents: 0,
  carriedForwardCents: 0,
  paymentCorrection: { state: "editable", correctedCents: null, maxCents: 30000 },
}

const receipt = {
  payment: {
    id: null,
    origin: "corrected",
    amount_cents: 20000,
    date: "2026-01-27",
    is_projection: false,
  },
  invoice: {
    credit_card_id: "k1",
    closing_month: "2026-01",
    closing_date: "2026-01-20",
    due_date: "2026-01-27",
    reference_month: "2026-01",
    state: "paid",
    total_cents: 30000,
    paid_cents: 20000,
    remaining_cents: 0,
    charges_cents: 0,
    carried_balance_cents: 0,
    carried_forward_cents: 10000,
    payment_correction: { state: "editable", corrected_cents: 20000, max_cents: 30000 },
  },
}

function json(status: number, body?: unknown) {
  return Promise.resolve(
    new Response(body === undefined ? null : JSON.stringify(body), {
      status,
      headers: { "Content-Type": "application/json" },
    }),
  )
}

describe("CorrectPaymentDialog", () => {
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
        <CorrectPaymentDialog cardId="k1" cardName="Nubank" invoice={invoice} onClose={onClose} />
      </QueryClientProvider>,
    )
  }

  it("starts with what was debited, shows what will roll over and sends the amount really paid", async () => {
    fetchMock.mockImplementation(() => json(200, receipt))
    renderDialog()

    const field = screen.getByLabelText("Valor realmente pago")
    expect(field).toHaveValue("300,00")
    await userEvent.clear(field)
    await userEvent.type(field, "200,00")
    expect(screen.getByText(/R\$ 100,00 que ficaram sem pagar vão como saldo rolado/)).toBeInTheDocument()
    await userEvent.click(screen.getByRole("button", { name: "Salvar" }))

    await vi.waitFor(() => expect(onClose).toHaveBeenCalled())
    expect(fetchMock).toHaveBeenCalledWith(
      "/api/credit-cards/k1/invoices/2026-01/payment",
      expect.objectContaining({ method: "PUT", body: JSON.stringify({ amount_cents: 20000 }) }),
    )
    expect(toast.success).toHaveBeenCalled()
  })

  it("accepts zero, because the person may have paid nothing", async () => {
    fetchMock.mockImplementation(() => json(200, receipt))
    renderDialog()

    const field = screen.getByLabelText("Valor realmente pago")
    await userEvent.clear(field)
    await userEvent.type(field, "0")
    await userEvent.click(screen.getByRole("button", { name: "Salvar" }))

    await vi.waitFor(() => expect(onClose).toHaveBeenCalled())
    expect(fetchMock).toHaveBeenCalledWith(
      "/api/credit-cards/k1/invoices/2026-01/payment",
      expect.objectContaining({ body: JSON.stringify({ amount_cents: 0 }) }),
    )
  })

  it("refuses more than the invoice owed at the due date, without calling the API", async () => {
    renderDialog()

    const field = screen.getByLabelText("Valor realmente pago")
    await userEvent.clear(field)
    await userEvent.type(field, "300,01")
    await userEvent.click(screen.getByRole("button", { name: "Salvar" }))

    expect(screen.getByText(/não pode passar do que a fatura devia/)).toBeInTheDocument()
    expect(fetchMock).not.toHaveBeenCalled()
    expect(onClose).not.toHaveBeenCalled()
  })

  it("explains in Portuguese when the payment turned out to be locked", async () => {
    fetchMock.mockImplementation(() =>
      json(409, { error: { code: "invoice_payment_locked", message: "locked" } }),
    )
    renderDialog()

    await userEvent.click(screen.getByRole("button", { name: "Salvar" }))

    expect(await screen.findByText(/já está paga/)).toBeInTheDocument()
    expect(onClose).not.toHaveBeenCalled()
  })
})
