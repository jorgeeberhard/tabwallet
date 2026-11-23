/**
 * Months as `YYYY-MM` text, the way the API takes them. Shared by the budgets
 * screen and the report periods.
 */

/** The month (`YYYY-MM`) a calendar day (`YYYY-MM-DD`) belongs to. */
export function monthOf(isoDay: string): string {
  return isoDay.slice(0, 7)
}

/** `yearMonth` moved by whole months (negative goes back), as `YYYY-MM`. */
export function shiftMonth(yearMonth: string, months: number): string {
  const [year, month] = yearMonth.split("-").map(Number)
  const index = year * 12 + (month - 1) + months
  const shiftedYear = Math.floor(index / 12)
  const shiftedMonth = ((index % 12) + 12) % 12 + 1
  return `${String(shiftedYear).padStart(4, "0")}-${String(shiftedMonth).padStart(2, "0")}`
}
