import { describe, expect, it } from "vitest"

import { monthOf, shiftMonth } from "@/lib/year-month"

describe("year-month", () => {
  it("takes the month of a day", () => {
    expect(monthOf("2026-11-07")).toBe("2026-11")
  })

  it("moves across year boundaries in both directions", () => {
    expect(shiftMonth("2026-11", 1)).toBe("2026-12")
    expect(shiftMonth("2026-12", 1)).toBe("2027-01")
    expect(shiftMonth("2027-01", -1)).toBe("2026-12")
    expect(shiftMonth("2026-01", -13)).toBe("2024-12")
    expect(shiftMonth("2026-05", 0)).toBe("2026-05")
    expect(shiftMonth("2026-11", 3)).toBe("2027-02")
    expect(shiftMonth("2026-03", -14)).toBe("2025-01")
  })
})
