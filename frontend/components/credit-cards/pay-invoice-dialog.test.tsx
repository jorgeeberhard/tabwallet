import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { render, screen } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { PayInvoiceDialog } from "@/components/credit-cards/pay-invoice-dialog"
import type { Invoice } from "@/lib/credit-cards-api"

const toast = vi.hoisted(() => ({ success: vi.fn(), error: vi.fn() }))
vi.mock("sonner", () => ({ toast }))

const invoice: Invoice = {
  creditCardId: "k1",
  closingMonth: "2026-01",
  closingDate: "2026-01-20",
  dueDate: "2026-01-27",
  referenceMonth: "2026-01",
  state: "open",
  totalCents: 30000,
  total: 300,
  paidCents: 10000,
  paid: 100,
  remainingCents: 20000,
  remaining: 200,
  chargesCents: 0,
  carriedBalanceCents: 0,
  carriedForwardCents: 0,
  paymentCorrection: { state: "not_due", correctedCents: null, maxCents: 0 },
}

function json(status: number, body?: unknown) {
  return Promise.resolve(
    new Response(body === undefined ? null : JSON.stringify(body), {
      status,
      headers: { "Content-Type": "application/json" },
    }),
  )
}

describe("PayInvoiceDialog", () => {
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
        <PayInvoiceDialog cardId="k1" cardName="Nubank" invoice={invoice} onClose={onClose} />
      </QueryClientProvider>,
    )
  }

  it("starts with everything that is left to pay and sends the amount in cents", async () => {
    fetchMock.mockImplementation(() =>
      json(201, {
        payment: {
          id: "p1",
          origin: "early",
          amount_cents: 20000,
          date: "2026-01-15",
          is_projection: false,
        },
        invoice: { payment_correction: { state: "not_due", corrected_cents: null, max_cents: 0 } },
      }),
    )
    renderDialog()

    expect(screen.getByLabelText("Valor a pagar")).toHaveValue("200,00")
    await userEvent.click(screen.getByRole("button", { name: "Pagar" }))

    await vi.waitFor(() => expect(onClose).toHaveBeenCalled())
    expect(fetchMock).toHaveBeenCalledWith(
      "/api/credit-cards/k1/invoices/2026-01/payments",
      expect.objectContaining({ method: "POST", body: JSON.stringify({ amount_cents: 20000 }) }),
    )
    expect(toast.success).toHaveBeenCalled()
  })

  it("accepts a partial amount", async () => {
    fetchMock.mockImplementation(() =>
      json(201, {
        payment: { id: "p1", origin: "early", amount_cents: 5050, date: "2026-01-15", is_projection: false },
        invoice: { payment_correction: { state: "not_due", corrected_cents: null, max_cents: 0 } },
      }),
    )
    renderDialog()

    const field = screen.getByLabelText("Valor a pagar")
    await userEvent.clear(field)
    await userEvent.type(field, "50,50")
    await userEvent.click(screen.getByRole("button", { name: "Pagar" }))

    await vi.waitFor(() => expect(onClose).toHaveBeenCalled())
    expect(fetchMock).toHaveBeenCalledWith(
      "/api/credit-cards/k1/invoices/2026-01/payments",
      expect.objectContaining({ body: JSON.stringify({ amount_cents: 5050 }) }),
    )
  })

  it("refuses an amount above what is left or not above zero, without calling the API", async () => {
    renderDialog()
    const field = screen.getByLabelText("Valor a pagar")

    await userEvent.clear(field)
    await userEvent.type(field, "200,01")
    await userEvent.click(screen.getByRole("button", { name: "Pagar" }))
    expect(
      await screen.findByText("O valor não pode passar do restante da fatura (R$ 200,00)."),
    ).toBeInTheDocument()

    await userEvent.clear(field)
    await userEvent.click(screen.getByRole("button", { name: "Pagar" }))
    expect(await screen.findByText("Digite um valor maior que zero.")).toBeInTheDocument()
    expect(fetchMock).not.toHaveBeenCalled()
  })

  it("explains an API refusal and keeps the dialog open", async () => {
    fetchMock.mockImplementation(() =>
      json(409, { error: { code: "invoice_paid", message: "x" } }),
    )
    renderDialog()

    await userEvent.click(screen.getByRole("button", { name: "Pagar" }))

    expect(await screen.findByText(/Esta fatura já está paga/)).toBeInTheDocument()
    expect(onClose).not.toHaveBeenCalled()
  })
})
