import { describe, expect, it } from "vitest"

import {
  addDays,
  buildRows,
  historyStart,
  lastOfMonthShifted,
  lowestPoint,
  projectionEnd,
} from "@/lib/balance-series"
import type { BalancePoint } from "@/lib/balance-series-api"

function point(date: string, balance: number): BalancePoint {
  return { date, balance, balanceCents: Math.round(balance * 100) }
}

describe("period helpers", () => {
  it("counts today in a daily history and the current month in a monthly one", () => {
    expect(historyStart("2026-01-15", "day", 30)).toBe("2025-12-17")
    expect(historyStart("2026-01-15", "month", 12)).toBe("2025-02-01")
    expect(historyStart("2026-03-31", "month", 1)).toBe("2026-03-01")
  })

  it("ends a daily projection N days ahead and a monthly one at the end of the Nth month", () => {
    expect(projectionEnd("2026-01-15", "day", 90)).toBe("2026-04-15")
    expect(projectionEnd("2026-01-15", "month", 3)).toBe("2026-04-30")
    expect(projectionEnd("2026-01-31", "month", 1)).toBe("2026-02-28")
    expect(projectionEnd("2026-01-15", "month", 12)).toBe("2027-01-31")
  })

  it("never asks beyond the 12-month horizon", () => {
    // The horizon is the end of the month 12 months after today's month.
    expect(projectionEnd("2026-12-31", "day", 365) <= lastOfMonthShifted("2026-12-31", 12)).toBe(
      true,
    )
    expect(addDays("2026-12-31", 365)).toBe("2027-12-31")
  })
})

describe("buildRows", () => {
  it("joins the two lines at today and marks the rows after it as projected", () => {
    const rows = buildRows(
      [point("2026-01-14", 10), point("2026-01-15", 12)],
      [point("2026-01-15", 12), point("2026-01-16", 9)],
      "2026-01-15",
      12,
    )

    expect(rows).toEqual([
      { date: "2026-01-14", history: 10, kind: "history" },
      { date: "2026-01-15", history: 12, projected: 12, kind: "history" },
      { date: "2026-01-16", projected: 9, kind: "projected" },
    ])
  })

  it("keeps the history up to today and the month end apart in a monthly series", () => {
    const rows = buildRows(
      [point("2025-12-31", 5), point("2026-01-15", 12)],
      [point("2026-01-31", 20), point("2026-02-28", 30)],
      "2026-01-15",
      12,
    )

    expect(rows.map((row) => [row.date, row.kind])).toEqual([
      ["2025-12-31", "history"],
      ["2026-01-15", "history"],
      ["2026-01-31", "projected"],
      ["2026-02-28", "projected"],
    ])
    // The dashed line leaves from the Current Balance.
    expect(rows[1].projected).toBe(12)
  })
})

describe("lowestPoint", () => {
  it("finds the tightest point, and nothing when there are none", () => {
    expect(lowestPoint([point("a", 3), point("b", -2), point("c", 1)])?.date).toBe("b")
    expect(lowestPoint([])).toBeNull()
  })
})
