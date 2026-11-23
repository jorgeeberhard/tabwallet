import { describe, expect, it } from "vitest"

import { todayInSaoPaulo } from "@/lib/today"

describe("todayInSaoPaulo", () => {
  it("follows the Sao Paulo calendar day, not UTC", () => {
    // 02:59 UTC is still 23:59 of the previous day in Sao Paulo (UTC-3).
    expect(todayInSaoPaulo(new Date("2026-10-27T02:59:59Z"))).toBe("2026-10-26")
    expect(todayInSaoPaulo(new Date("2026-10-27T03:00:00Z"))).toBe("2026-10-27")
  })
})
