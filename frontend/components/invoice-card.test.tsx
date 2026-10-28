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
