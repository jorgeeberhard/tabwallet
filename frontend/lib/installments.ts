/** The fewest and the most Installments the API accepts. */
export const MIN_INSTALLMENTS = 2
export const MAX_INSTALLMENTS = 48

/**
 * Splits a total in cents into `count` exact amounts, the cents left over by
 * the division going to the first one (R$ 100 in 3x = 33,34 + 33,33 + 33,33).
 * It is only the preview in the form: the API does the real split. `null`
 * when the total cannot give every Installment at least one cent.
 */
export function splitInstallments(totalCents: number, count: number): number[] | null {
  if (!Number.isInteger(count) || count < 1 || totalCents < count) return null
  const share = Math.floor(totalCents / count)
  const leftover = totalCents - share * count
  return Array.from({ length: count }, (_, index) => (index === 0 ? share + leftover : share))
}
