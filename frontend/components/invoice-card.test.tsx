import { render, screen } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { describe, expect, it, vi } from "vitest"

import { InvoiceCard } from "@/components/invoice-card"
import type { Invoice } from "@/lib/credit-cards-api"

const base: Invoice = {
  creditCardId: "k1",
  closingMonth: "2026-01",
  closingDate: "2026-01-10",
  dueDate: "2026-01-17",
  referenceMonth: "2026-01",
  state: "open",
  totalCents: 123456,
  total: 1234.56,
  paidCents: 0,
  paid: 0,
  remainingCents: 123456,
  remaining: 1234.56,
  chargesCents: 0,
  carriedBalanceCents: 0,
  carriedForwardCents: 0,
  paymentCorrection: { state: "not_due", correctedCents: null, maxCents: 0 },
}

describe("InvoiceCard", () => {
  it("shows the card, the reference month, the total, the state and the dates", () => {
    render(<InvoiceCard cardName="Nubank" invoice={base} />)

    expect(screen.getByText("Fatura de Nubank")).toBeInTheDocument()
    expect(screen.getByText("janeiro de 2026")).toBeInTheDocument()
    expect(screen.getByText("R$ 1.234,56")).toBeInTheDocument()
    expect(screen.getByText("Aberta")).toBeInTheDocument()
    expect(screen.getByText("10/01/2026")).toBeInTheDocument()
    expect(screen.getByText("17/01/2026")).toBeInTheDocument()
    expect(screen.getByText("Fecha em")).toBeInTheDocument()
  })

  it("labels a closed and a paid invoice in the past tense", () => {
    const { rerender } = render(
      <InvoiceCard cardName="Nubank" invoice={{ ...base, state: "closed" }} />,
    )
    expect(screen.getByText("Fechada")).toBeInTheDocument()
    expect(screen.getByText("Fechou em")).toBeInTheDocument()
    expect(screen.getByText("Vence em")).toBeInTheDocument()

    rerender(<InvoiceCard cardName="Nubank" invoice={{ ...base, state: "paid" }} />)
    expect(screen.getByText("Paga")).toBeInTheDocument()
    expect(screen.getByText("Venceu em")).toBeInTheDocument()
  })

  it("shows what is left to pay only when it differs from the total", () => {
    const { rerender } = render(<InvoiceCard cardName="Nubank" invoice={base} />)
    expect(screen.queryByText(/Restante/)).not.toBeInTheDocument()

    rerender(
      <InvoiceCard
        cardName="Nubank"
        invoice={{ ...base, paidCents: 73456, paid: 734.56, remainingCents: 50000, remaining: 500 }}
      />,
    )
    expect(screen.getByText(/Restante/)).toBeInTheDocument()
    expect(screen.getByText("R$ 500,00")).toBeInTheDocument()
    expect(screen.getByText(/Pago:/)).toBeInTheDocument()
    expect(screen.getByText("R$ 734,56")).toBeInTheDocument()
  })

  it("offers to pay early while the invoice is open or closed and has something to pay", async () => {
    const onPayEarly = vi.fn()
    const { rerender } = render(
      <InvoiceCard cardName="Nubank" invoice={base} onPayEarly={onPayEarly} />,
    )
    await userEvent.click(screen.getByRole("button", { name: "Pagar antecipado" }))
    expect(onPayEarly).toHaveBeenCalledTimes(1)

    rerender(
      <InvoiceCard
        cardName="Nubank"
        invoice={{ ...base, state: "closed" }}
        onPayEarly={onPayEarly}
      />,
    )
    expect(screen.getByRole("button", { name: "Pagar antecipado" })).toBeInTheDocument()

    rerender(
      <InvoiceCard cardName="Nubank" invoice={{ ...base, state: "paid" }} onPayEarly={onPayEarly} />,
    )
    expect(screen.queryByRole("button", { name: "Pagar antecipado" })).not.toBeInTheDocument()

    rerender(
      <InvoiceCard
        cardName="Nubank"
        invoice={{ ...base, paidCents: 123456, paid: 1234.56, remainingCents: 0, remaining: 0 }}
        onPayEarly={onPayEarly}
      />,
    )
    expect(screen.queryByRole("button", { name: "Pagar antecipado" })).not.toBeInTheDocument()
    expect(screen.getByText(/Paga antecipadamente/)).toBeInTheDocument()
  })

  const paid: Invoice = {
    ...base,
    state: "paid",
    paidCents: 123456,
    paid: 1234.56,
    remainingCents: 0,
    remaining: 0,
  }

  it("offers to correct the payment of a paid invoice while it can still be corrected", async () => {
    const onCorrectPayment = vi.fn()
    const editable: Invoice = {
      ...paid,
      paymentCorrection: { state: "editable", correctedCents: null, maxCents: 123456 },
    }
    const { rerender } = render(
      <InvoiceCard cardName="Nubank" invoice={editable} onCorrectPayment={onCorrectPayment} />,
    )
    await userEvent.click(screen.getByRole("button", { name: "Corrigir pagamento" }))
    expect(onCorrectPayment).toHaveBeenCalledTimes(1)

    rerender(
      <InvoiceCard
        cardName="Nubank"
        invoice={{ ...paid, paymentCorrection: { ...editable.paymentCorrection, state: "not_due" } }}
        onCorrectPayment={onCorrectPayment}
      />,
    )
    expect(screen.queryByRole("button", { name: "Corrigir pagamento" })).not.toBeInTheDocument()
  })

  it("explains in plain words why a locked payment cannot be corrected", () => {
    render(
      <InvoiceCard
        cardName="Nubank"
        invoice={{
          ...paid,
          paymentCorrection: { state: "locked", correctedCents: null, maxCents: 123456 },
        }}
        onCorrectPayment={vi.fn()}
      />,
    )

    expect(screen.queryByRole("button", { name: "Corrigir pagamento" })).not.toBeInTheDocument()
    expect(
      screen.getByText(/O pagamento desta fatura não pode mais ser corrigido/),
    ).toBeInTheDocument()
  })

  it("shows the Carried Balance received from the previous invoice as part of the total", () => {
    render(
      <InvoiceCard
        cardName="Nubank"
        invoice={{ ...base, carriedBalanceCents: 10000 }}
        carriedBalance={{ amountCents: 10000, amount: 100, fromClosingMonth: "2025-12" }}
      />,
    )

    expect(screen.getByText(/Saldo rolado da fatura de dezembro de 2025/)).toBeInTheDocument()
    expect(screen.getByText("R$ 100,00")).toBeInTheDocument()
    expect(screen.getByText(/Não conta como nova despesa/)).toBeInTheDocument()
  })

  it("shows the part of a paid invoice that moved on to the next one", () => {
    render(
      <InvoiceCard
        cardName="Nubank"
        invoice={{ ...paid, paidCents: 100000, paid: 1000, carriedForwardCents: 23456 }}
      />,
    )

    expect(screen.getByText(/Passou para a próxima fatura como saldo rolado/)).toBeInTheDocument()
    expect(screen.getByText("R$ 234,56")).toBeInTheDocument()
  })

  it("shows the Charges of the invoice and offers to record more until it is paid", async () => {
    const onAddCharges = vi.fn()
    const { rerender } = render(
      <InvoiceCard
        cardName="Nubank"
        invoice={{ ...base, chargesCents: 5000 }}
        onAddCharges={onAddCharges}
      />,
    )
    expect(screen.getByText(/Encargos:/)).toBeInTheDocument()
    expect(screen.getByText("R$ 50,00")).toBeInTheDocument()
    await userEvent.click(screen.getByRole("button", { name: "Lançar encargos" }))
    expect(onAddCharges).toHaveBeenCalledTimes(1)

    rerender(<InvoiceCard cardName="Nubank" invoice={paid} onAddCharges={onAddCharges} />)
    expect(screen.queryByRole("button", { name: "Lançar encargos" })).not.toBeInTheDocument()
  })

  it("shows what was paid on a paid invoice", () => {
    render(
      <InvoiceCard
        cardName="Nubank"
        invoice={{ ...base, state: "paid", paidCents: 123456, paid: 1234.56, remainingCents: 0, remaining: 0 }}
      />,
    )

    expect(screen.getByText(/Paga:/)).toBeInTheDocument()
    expect(screen.queryByText(/Restante/)).not.toBeInTheDocument()
  })
})
