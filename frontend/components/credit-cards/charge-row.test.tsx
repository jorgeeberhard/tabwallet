import { render, screen } from "@testing-library/react"
import { describe, expect, it, vi } from "vitest"

import { ChargeRow } from "@/components/credit-cards/charge-row"
import type { Charge } from "@/lib/credit-cards-api"

const charge: Charge = {
  id: "c1",
  amountCents: 1250,
  amount: 12.5,
  date: "2026-01-22",
  description: "Juros de janeiro",
  category: { id: "cat", name: "Juros e encargos", active: true },
  creditCard: { id: "k1", name: "Nubank", active: true },
  invoice: {
    closingMonth: "2026-01",
    closingDate: "2026-01-20",
    dueDate: "2026-01-27",
    referenceMonth: "2026-01",
    state: "closed",
  },
  referenceMonth: "2026-01",
  isProjection: false,
}

function renderRow(value: Charge, showCard?: boolean) {
  render(
    <ul>
      <ChargeRow charge={value} showCard={showCard} onEdit={vi.fn()} onDelete={vi.fn()} />
    </ul>,
  )
}

describe("ChargeRow", () => {
  it("shows a charge as an expense of Juros e encargos linked to its invoice", () => {
    renderRow(charge)

    expect(screen.getByText("Encargos", { selector: "[data-slot=badge]" })).toBeInTheDocument()
    expect(screen.getByText("Juros e encargos")).toBeInTheDocument()
    expect(screen.getByText("Nubank")).toBeInTheDocument()
    expect(screen.getByText("Fatura de janeiro de 2026 · fechada")).toBeInTheDocument()
    expect(screen.getByText("− R$ 12,50")).toHaveClass("text-expense")
    expect(screen.getByRole("button", { name: "Editar Juros de janeiro" })).toBeEnabled()
  })

  it("locks the actions once the invoice is paid", () => {
    renderRow({ ...charge, invoice: { ...charge.invoice, state: "paid" } })

    expect(screen.getByRole("button", { name: "Editar Juros de janeiro" })).toBeDisabled()
    expect(screen.getByRole("button", { name: "Excluir Juros de janeiro" })).toBeDisabled()
  })
})
