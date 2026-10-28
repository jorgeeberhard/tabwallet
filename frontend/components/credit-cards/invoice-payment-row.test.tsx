import { render, screen } from "@testing-library/react"
import { describe, expect, it } from "vitest"

import { InvoicePaymentRow } from "@/components/credit-cards/invoice-payment-row"
import type { InvoicePayment } from "@/lib/credit-cards-api"

const early: InvoicePayment = {
  id: "p1",
  origin: "early",
  amountCents: 12000,
  amount: 120,
  date: "2026-01-15",
  isProjection: false,
}

function renderRow(payment: InvoicePayment, cardName?: string) {
  render(
    <ul>
      <InvoicePaymentRow payment={payment} cardName={cardName} referenceMonth="2026-01" />
    </ul>,
  )
}

describe("InvoicePaymentRow", () => {
  it("marks an early payment as an invoice payment that leaves the balance, not an expense", () => {
    renderRow(early, "Nubank")

    expect(screen.getByText("Pagamento antecipado da fatura de janeiro de 2026")).toBeInTheDocument()
    expect(screen.getByText("Pagamento de fatura")).toBeInTheDocument()
    expect(screen.getByText("Nubank")).toBeInTheDocument()
    expect(screen.getByText("Antecipado")).toBeInTheDocument()
    const amount = screen.getByText("\u2212 R$ 120,00")
    expect(amount).not.toHaveClass("text-expense")
    expect(screen.queryByText("Projeção")).not.toBeInTheDocument()
  })

  it("marks the automatic payment of the due date, as a projection while it is in the future", () => {
    renderRow({ ...early, id: null, origin: "automatic", isProjection: true })

    expect(screen.getByText("Pagamento automático da fatura de janeiro de 2026")).toBeInTheDocument()
    expect(screen.getByText("No vencimento")).toBeInTheDocument()
    expect(screen.getByText("Projeção")).toBeInTheDocument()
  })
})
