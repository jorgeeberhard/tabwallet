import { render, screen } from "@testing-library/react"
import { describe, expect, it } from "vitest"

import { BudgetBar, budgetBarState, budgetPercent } from "@/components/budget-bar"

describe("budgetBarState", () => {
  it("is ok below 80%, a warning from 80% and exceeded only above the limit", () => {
    expect(budgetBarState(7999, 10000)).toBe("ok")
    expect(budgetBarState(8000, 10000)).toBe("warning")
    expect(budgetBarState(10000, 10000)).toBe("warning")
    expect(budgetBarState(10001, 10000)).toBe("exceeded")
  })
})

describe("budgetPercent", () => {
  it("rounds down and passes 100 when exceeded", () => {
    expect(budgetPercent(4100, 10000)).toBe(41)
    expect(budgetPercent(100, 300)).toBe(33)
    expect(budgetPercent(15000, 10000)).toBe(150)
  })
})

describe("BudgetBar", () => {
  it("writes how much of the limit is used while it is fine", () => {
    render(<BudgetBar category="Lazer" spentCents={16400} limitCents={40000} />)
    expect(screen.getByText("41% usado")).toBeInTheDocument()
    expect(screen.getByRole("progressbar", { name: "Gasto em Lazer" })).toHaveAttribute(
      "aria-valuenow",
      "41",
    )
  })

  it("warns from 80% of the limit", () => {
    render(<BudgetBar category="Lazer" spentCents={36800} limitCents={40000} />)
    expect(screen.getByText("92% do limite")).toBeInTheDocument()
  })

  it("says by how much the limit was exceeded and fills the bar", () => {
    render(<BudgetBar category="Lazer" spentCents={52000} limitCents={40000} />)
    expect(screen.getByText("Limite excedido em R$ 120,00")).toBeInTheDocument()
    expect(screen.getByRole("progressbar", { name: "Gasto em Lazer" })).toHaveAttribute(
      "aria-valuenow",
      "100",
    )
  })

  it("shows the amounts spent and of the limit", () => {
    render(<BudgetBar category="Lazer" spentCents={16400} limitCents={40000} />)
    expect(screen.getByText("R$ 164,00")).toBeInTheDocument()
    expect(screen.getByText("R$ 400,00")).toBeInTheDocument()
  })
})
