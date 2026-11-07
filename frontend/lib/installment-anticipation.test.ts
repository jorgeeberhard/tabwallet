import { describe, expect, it } from "vitest"

import { discountTooLarge, summarizeAnticipation } from "@/lib/installment-anticipation"

const chosen = [{ amountCents: 2_500 }, { amountCents: 3_000 }]

describe("summarizeAnticipation", () => {
  it("adds the chosen installments and takes the discount off", () => {
    expect(summarizeAnticipation(chosen, 500)).toEqual({
      count: 2,
      originalCents: 5_500,
      discountCents: 500,
      totalCents: 5_000,
    })
  })

  it("has no discount by default", () => {
    expect(summarizeAnticipation(chosen, 0).totalCents).toBe(5_500)
    expect(summarizeAnticipation([], 0).totalCents).toBe(0)
  })
})

describe("discountTooLarge", () => {
  it("is true when the discount reaches the sum of the installments", () => {
    expect(discountTooLarge(5_500, chosen)).toBe(true)
    expect(discountTooLarge(6_000, chosen)).toBe(true)
    expect(discountTooLarge(5_499, chosen)).toBe(false)
  })
})
