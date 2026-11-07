import { render, screen } from "@testing-library/react"
import { describe, expect, it } from "vitest"

import { BudgetBar, budgetBarState, budgetPercent } from "@/components/budget-bar"

describe("budgetBarState", () => {
  it("is ok below 80%, a warning from 80% and exceeded only above the limit", () => {
    expect(budgetBarState(79.99, 100)).toBe("ok")
    expect(budgetBarState(80, 100)).toBe("warning")
    expect(budgetBarState(100, 100)).toBe("warning")
    expect(budgetBarState(100.01, 100)).toBe("exceeded")
  })

  it("compares in cents, not in floating point", () => {
    // 0.1 + 0.2 is 0.30000000000000004 in floating point.
    expect(budgetBarState(0.1 + 0.2, 0.3)).toBe("warning")
  })
})

describe("budgetPercent", () => {
  it("rounds down and passes 100 when exceeded", () => {
    expect(budgetPercent(41, 100)).toBe(41)
    expect(budgetPercent(1, 3)).toBe(33)
    expect(budgetPercent(150, 100)).toBe(150)
  })
})

describe("BudgetBar", () => {
  it("writes how much of the limit is used while it is fine", () => {
    render(<BudgetBar category="Lazer" spent={164} limit={400} />)
    expect(screen.getByText("41% usado")).toBeInTheDocument()
    expect(screen.getByRole("progressbar", { name: "Gasto em Lazer" })).toHaveAttribute(
      "aria-valuenow",
      "41",
    )
  })

  it("warns from 80% of the limit", () => {
    render(<BudgetBar category="Lazer" spent={368} limit={400} />)
    expect(screen.getByText("92% do limite")).toBeInTheDocument()
  })

  it("says by how much the limit was exceeded and fills the bar", () => {
    render(<BudgetBar category="Lazer" spent={520} limit={400} />)
    expect(screen.getByText("Limite excedido em R$ 120,00")).toBeInTheDocument()
    expect(screen.getByRole("progressbar", { name: "Gasto em Lazer" })).toHaveAttribute(
      "aria-valuenow",
      "100",
    )
  })

  it("shows the amounts spent and of the limit", () => {
    render(<BudgetBar category="Lazer" spent={164} limit={400} />)
    expect(screen.getByText("R$ 164,00")).toBeInTheDocument()
    expect(screen.getByText("R$ 400,00")).toBeInTheDocument()
  })
})
