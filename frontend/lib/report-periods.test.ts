import { describe, expect, it } from "vitest"

import {
  describePeriod,
  monthOf,
  monthOptions,
  monthsBetween,
  periodProblem,
  presetRange,
  resolvePeriod,
  shiftMonth,
} from "@/lib/report-periods"

describe("report periods", () => {
  it("reads the month of a day and moves months across years", () => {
    expect(monthOf("2026-01-15")).toBe("2026-01")
    expect(shiftMonth("2026-01", -1)).toBe("2025-12")
    expect(shiftMonth("2026-11", 3)).toBe("2027-02")
    expect(shiftMonth("2026-03", -14)).toBe("2025-01")
  })

  it("counts both ends of a period", () => {
    expect(monthsBetween("2026-01", "2026-01")).toBe(1)
    expect(monthsBetween("2025-11", "2026-02")).toBe(4)
    expect(monthsBetween("2026-03", "2026-01")).toBe(-1)
  })

  it("defaults to the current month and has presets around it", () => {
    expect(presetRange("2026-01", "this-month")).toEqual({ from: "2026-01", to: "2026-01" })
    expect(presetRange("2026-01", "last-month")).toEqual({ from: "2025-12", to: "2025-12" })
    expect(presetRange("2026-01", "last-6")).toEqual({ from: "2025-08", to: "2026-01" })
    expect(presetRange("2026-01", "next-3")).toEqual({ from: "2026-01", to: "2026-03" })
    expect(presetRange("2026-01", "unknown")).toEqual({ from: "2026-01", to: "2026-01" })
  })

  it("offers months from three years back to a year ahead", () => {
    const options = monthOptions("2026-01")
    expect(options[0]).toEqual({ value: "2023-02", label: "fevereiro de 2023" })
    expect(options.at(-1)).toEqual({ value: "2027-01", label: "janeiro de 2027" })
  })

  it("flags an upside-down or too long period like the API does", () => {
    expect(periodProblem({ from: "2026-03", to: "2026-01" })).toBe("order")
    expect(periodProblem({ from: "2023-01", to: "2026-01" })).toBe("too_long")
    expect(periodProblem({ from: "2023-02", to: "2026-01" })).toBeNull()
  })

  it("resolves a choice into the preset's months or the two custom months", () => {
    expect(resolvePeriod("2026-01", { presetId: "last-3" })).toEqual({
      from: "2025-11",
      to: "2026-01",
    })
    expect(
      resolvePeriod("2026-01", { presetId: "custom", from: "2025-03", to: "2025-05" }),
    ).toEqual({ from: "2025-03", to: "2025-05" })
    expect(resolvePeriod("2026-01", { presetId: "custom" })).toEqual({
      from: "2026-01",
      to: "2026-01",
    })
  })

  it("describes a period in words", () => {
    expect(describePeriod({ from: "2026-01", to: "2026-01" })).toBe("janeiro de 2026")
    expect(describePeriod({ from: "2026-01", to: "2026-03" })).toBe("janeiro a março de 2026")
    expect(describePeriod({ from: "2025-11", to: "2026-02" })).toBe(
      "novembro de 2025 a fevereiro de 2026",
    )
  })
})
