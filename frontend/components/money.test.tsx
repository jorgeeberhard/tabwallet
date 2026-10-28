import { render, screen } from "@testing-library/react"
import { describe, expect, it } from "vitest"

import { Money } from "@/components/money"

// Testing Library collapses non-breaking spaces to plain spaces when matching,
// so the expectations below use plain spaces. The non-breaking space itself is
// covered by the formatBRL test.
describe("Money", () => {
  it("shows income with a plus sign in the income colour", () => {
    render(<Money value={3200} kind="income" />)

    expect(screen.getByText("+ R$ 3.200,00")).toHaveClass("text-income")
  })

  it("shows expense with a true minus sign (U+2212) in the expense colour", () => {
    render(<Money value={45.9} kind="expense" />)

    expect(screen.getByText("− R$ 45,90")).toHaveClass("text-expense")
  })

  it("infers the kind from the sign of the value by default", () => {
    render(<Money value={-1180.4} />)

    expect(screen.getByText("− R$ 1.180,40")).toHaveClass("text-expense")
  })

  it("keeps neutral amounts unsigned and in the text colour", () => {
    render(<Money value={500} kind="neutral" />)

    expect(screen.getByText("R$ 500,00")).toHaveClass("text-foreground")
  })

  it("keeps the minus sign and turns red for a negative balance without sign display", () => {
    render(<Money value={-120} showSign={false} />)

    expect(screen.getByText("− R$ 120,00")).toHaveClass("text-expense")
  })

  it("leaves a positive balance neutral when sign display is off", () => {
    render(<Money value={120} showSign={false} />)

    expect(screen.getByText("R$ 120,00")).toHaveClass("text-foreground")
  })

  it("aligns digits with tabular figures", () => {
    render(<Money value={1} kind="neutral" />)

    expect(screen.getByText("R$ 1,00")).toHaveClass("tabular-nums")
  })
})
