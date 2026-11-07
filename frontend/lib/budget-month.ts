/** The month (`YYYY-MM`) of a calendar day (`YYYY-MM-DD`). */
export function monthOf(isoDay: string): string {
  return isoDay.slice(0, 7)
}

/** `YYYY-MM` moved by `delta` months (negative goes back). */
export function shiftMonth(month: string, delta: number): string {
  const [year, number] = month.split("-").map(Number)
  const index = year * 12 + (number - 1) + delta
  const shiftedYear = Math.floor(index / 12)
  const shiftedMonth = ((index % 12) + 12) % 12 + 1
  return `${String(shiftedYear).padStart(4, "0")}-${String(shiftedMonth).padStart(2, "0")}`
}

/** How far ahead the budgets screen lets the person look, in months. */
export const MONTHS_AHEAD = 12
