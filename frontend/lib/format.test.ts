import { describe, expect, it } from "vitest"

import { formatBRL, formatDate, formatMonth } from "@/lib/format"

const NBSP = " "

describe("formatBRL", () => {
  it("formats reais in pt-BR with a non-breaking space after the symbol", () => {
    expect(formatBRL(1234.56)).toBe(`R$${NBSP}1.234,56`)
  })

  it("always shows two decimal places", () => {
    expect(formatBRL(45)).toBe(`R$${NBSP}45,00`)
  })

  it("writes a calendar day the Brazilian way, without time zones", () => {
    expect(formatDate("2026-01-09")).toBe("09/01/2026")
    expect(formatMonth("2026-03-31")).toBe("março de 2026")
  })

  it("formats the absolute value: the sign belongs to the caller", () => {
    expect(formatBRL(-45.9)).toBe(`R$${NBSP}45,90`)
  })
})
