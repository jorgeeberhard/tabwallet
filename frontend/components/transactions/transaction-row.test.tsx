import { render, screen } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { describe, expect, it, vi } from "vitest"

import { TransactionRow } from "@/components/transactions/transaction-row"
import type { Transaction } from "@/lib/transactions-api"

const base: Transaction = {
  id: "1",
  type: "expense",
  amountCents: 4590,
  amount: 45.9,
  date: "2026-01-10",
  description: "Supermercado",
  category: { id: "c1", name: "Alimentação", active: true },
  isProjection: false,
}

function renderRow(transaction: Transaction, handlers = { onEdit: vi.fn(), onDelete: vi.fn() }) {
  render(
    <ul>
      <TransactionRow transaction={transaction} {...handlers} />
    </ul>,
  )
  return handlers
}

describe("TransactionRow", () => {
  it("shows an expense with the description, date, category and a minus sign", () => {
    renderRow(base)

    expect(screen.getByText("Supermercado")).toBeInTheDocument()
    expect(screen.getByText("10/01/2026")).toBeInTheDocument()
    expect(screen.getByText("Alimentação")).toBeInTheDocument()
    expect(screen.getByText("− R$ 45,90")).toHaveClass("text-expense")
    expect(screen.queryByText("Projeção")).not.toBeInTheDocument()
  })

  it("shows an income with a plus sign", () => {
    renderRow({ ...base, type: "income", description: "Salário de janeiro" })

    expect(screen.getByText("+ R$ 45,90")).toHaveClass("text-income")
  })

  it("marks a Projection so it is not mistaken for money that already moved", () => {
    renderRow({ ...base, isProjection: true })

    expect(screen.getByText("Projeção")).toBeInTheDocument()
  })

  it("keeps a deactivated category visible on the old transaction, marked as such", () => {
    renderRow({ ...base, category: { id: "c1", name: "Lazer", active: false } })

    expect(screen.getByText("Lazer (desativada)")).toBeInTheDocument()
  })

  it("offers edit and delete for this transaction", async () => {
    const { onEdit, onDelete } = renderRow(base)

    await userEvent.click(screen.getByRole("button", { name: "Editar Supermercado" }))
    await userEvent.click(screen.getByRole("button", { name: "Excluir Supermercado" }))

    expect(onEdit).toHaveBeenCalledWith(base)
    expect(onDelete).toHaveBeenCalledWith(base)
  })
})
