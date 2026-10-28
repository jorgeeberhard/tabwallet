import { describe, expect, it } from "vitest"

import { formatAmountInput, parseAmountToCents } from "@/lib/amount"

describe("parseAmountToCents", () => {
  it.each([
    ["1180,4", 118040],
    ["1.180,40", 118040],
    ["1180.40", 118040],
    ["R$ 45,90", 4590],
    ["0,01", 1],
    ["12", 1200],
    ["1.180", 118000],
    ["19.9", 1990],
    ["0,29", 29],
  ])("reads %s as %i cents", (text, cents) => {
    expect(parseAmountToCents(text)).toBe(cents)
  })

  it.each(["", "abc", "0", "0,00", "-5", "1,234", "1,2,3", "12,", "1.18.0,5", "9999999999999"])(
    "refuses %j",
    (text) => {
      expect(parseAmountToCents(text)).toBeNull()
    },
  )
})

describe("formatAmountInput", () => {
  it("writes cents the way the field shows them", () => {
    expect(formatAmountInput(118040)).toBe("1.180,40")
    expect(formatAmountInput(5)).toBe("0,05")
  })
})
