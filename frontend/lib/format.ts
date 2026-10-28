const brl = new Intl.NumberFormat("pt-BR", {
  style: "currency",
  currency: "BRL",
})

/** True minus sign (U+2212), as the Design System requires for expenses. */
export const MINUS = "−"

/**
 * Formats an amount in reais as "R$ 1.234,56" (non-breaking space after the
 * symbol). Formats the absolute value: the caller adds the "+" or "−" sign.
 */
export function formatBRL(reais: number): string {
  return brl.format(Math.abs(reais)).replace(/\s/g, " ")
}
