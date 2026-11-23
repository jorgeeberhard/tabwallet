import { describe, expect, it } from "vitest"

import { splitInstallments } from "@/lib/installments"

describe("splitInstallments", () => {
  it("puts the leftover cents on the first installment", () => {
    expect(splitInstallments(10_000, 3)).toEqual([3_334, 3_333, 3_333])
    expect(splitInstallments(10_002, 4)).toEqual([2_502, 2_500, 2_500, 2_500])
  })

  it("splits an even total evenly", () => {
    expect(splitInstallments(12_000, 4)).toEqual([3_000, 3_000, 3_000, 3_000])
  })

  it("always adds up to the total", () => {
    for (const [total, count] of [
      [99_999, 7],
      [1, 1],
      [100, 48],
      [123_456_789, 13],
    ]) {
      const parts = splitInstallments(total, count) as number[]
      expect(parts).toHaveLength(count)
      expect(parts.reduce((sum, part) => sum + part, 0)).toBe(total)
    }
  })

  it("refuses a total that cannot give each installment a cent", () => {
    expect(splitInstallments(2, 3)).toBeNull()
    expect(splitInstallments(3, 3)).toEqual([1, 1, 1])
    expect(splitInstallments(100, 0)).toBeNull()
  })
})
