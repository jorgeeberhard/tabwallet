import { render, screen } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { describe, expect, it, vi } from "vitest"

import { CardPurchaseRow } from "@/components/credit-cards/card-purchase-row"
import type { CardPurchase } from "@/lib/credit-cards-api"

const base: CardPurchase = {
  id: "p1",
  amountCents: 12000,
  amount: 120,
  date: "2026-01-11",
  description: "Restaurante",
  category: { id: "c1", name: "Alimentação", active: true },
  creditCard: { id: "k1", name: "Nubank", active: true },
  invoice: {
    closingMonth: "2026-02",
    closingDate: "2026-02-10",
    dueDate: "2026-02-17",
    referenceMonth: "2026-02",
    state: "open",
  },
  referenceMonth: "2026-02",
  isProjection: false,
}

function renderRow(purchase: CardPurchase, handlers = { onEdit: vi.fn(), onDelete: vi.fn() }) {
  render(
    <ul>
      <CardPurchaseRow purchase={purchase} {...handlers} />
    </ul>,
  )
  return handlers
}

describe("CardPurchaseRow", () => {
  it("shows the card and the invoice the purchase belongs to", () => {
    renderRow(base)

    expect(screen.getByText("Restaurante")).toBeInTheDocument()
    expect(screen.getByText("Nubank")).toBeInTheDocument()
    expect(screen.getByText("Fatura de fevereiro de 2026 · aberta")).toBeInTheDocument()
    expect(screen.getByText("− R$ 120,00")).toHaveClass("text-expense")
  })

  it("lets the person edit and delete a purchase of an open invoice", async () => {
    const handlers = renderRow(base)

    await userEvent.click(screen.getByRole("button", { name: "Editar Restaurante" }))
    await userEvent.click(screen.getByRole("button", { name: "Excluir Restaurante" }))

    expect(handlers.onEdit).toHaveBeenCalledWith(base)
    expect(handlers.onDelete).toHaveBeenCalledWith(base)
  })

  it("turns editing and deleting off for a purchase of a paid invoice", () => {
    renderRow({ ...base, invoice: { ...base.invoice, state: "paid" } })

    expect(screen.getByRole("button", { name: "Editar Restaurante" })).toBeDisabled()
    expect(screen.getByRole("button", { name: "Excluir Restaurante" })).toBeDisabled()
    expect(screen.getByText("Fatura de fevereiro de 2026 · paga")).toBeInTheDocument()
  })
})
