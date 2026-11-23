import { render, screen } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { describe, expect, it, vi } from "vitest"

import { OccurrenceRow } from "@/components/transactions/occurrence-row"
import type { Occurrence } from "@/lib/recurring-api"

const base: Occurrence = {
  recurringId: "r1",
  month: "2026-02",
  type: "expense",
  amountCents: 150000,
  amount: 1500,
  date: "2026-02-05",
  description: "Aluguel",
  category: { id: "c1", name: "Moradia", active: true },
  isProjection: false,
  isEdited: false,
}

function renderRow(occurrence: Occurrence, handlers = { onEditAmount: vi.fn(), onSkip: vi.fn() }) {
  render(
    <ul>
      <OccurrenceRow occurrence={occurrence} {...handlers} />
    </ul>,
  )
  return handlers
}

describe("OccurrenceRow", () => {
  it("shows a fixed expense with its date, category and a minus sign", () => {
    renderRow(base)

    expect(screen.getByText("Aluguel")).toBeInTheDocument()
    expect(screen.getByText("05/02/2026")).toBeInTheDocument()
    expect(screen.getByText("Moradia")).toBeInTheDocument()
    expect(screen.getByText("Gasto fixo")).toBeInTheDocument()
    expect(screen.getByText("− R$ 1.500,00")).toHaveClass("text-expense")
    expect(screen.queryByText("Projeção")).not.toBeInTheDocument()
  })

  it("marks a future Occurrence as a Projection and an edited one as such", () => {
    renderRow({ ...base, isProjection: true, isEdited: true })

    expect(screen.getByText("Projeção")).toBeInTheDocument()
    expect(screen.getByText("Valor editado")).toBeInTheDocument()
  })

  it("shows a fixed income with a plus sign", () => {
    renderRow({ ...base, type: "income", description: "Salário" })

    expect(screen.getByText("Receita fixa")).toBeInTheDocument()
    expect(screen.getByText("+ R$ 1.500,00")).toHaveClass("text-income")
  })

  it("offers to edit the amount or delete this month only", async () => {
    const { onEditAmount, onSkip } = renderRow(base)

    await userEvent.click(
      screen.getByRole("button", { name: "Editar valor de Aluguel só deste mês" }),
    )
    await userEvent.click(screen.getByRole("button", { name: "Excluir Aluguel só deste mês" }))

    expect(onEditAmount).toHaveBeenCalledWith(base)
    expect(onSkip).toHaveBeenCalledWith(base)
  })
})
