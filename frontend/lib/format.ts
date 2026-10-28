const brl = new Intl.NumberFormat("pt-BR", {
  style: "currency",
  currency: "BRL",
})

/** True minus sign (U+2212), as the Design System requires for expenses. */
export const MINUS = "−"

/** "2026-01-10" as "10/01/2026". Dates are calendar days, so no time zone is involved. */
export function formatDate(isoDay: string): string {
  const [year, month, day] = isoDay.split("-")
  return `${day}/${month}/${year}`
}

const MONTH_NAMES = [
  "janeiro",
  "fevereiro",
  "março",
  "abril",
  "maio",
  "junho",
  "julho",
  "agosto",
  "setembro",
  "outubro",
  "novembro",
  "dezembro",
]

/** "2026-01-10" as "janeiro de 2026". */
export function formatMonth(isoDay: string): string {
  const [year, month] = isoDay.split("-")
  return `${MONTH_NAMES[Number(month) - 1]} de ${year}`
}

/** "2026-01" as "janeiro de 2026". */
export function formatMonthOfYear(yearMonth: string): string {
  return formatMonth(`${yearMonth}-01`)
}

/**
 * Formats an amount in reais as "R$ 1.234,56" (non-breaking space after the
 * symbol). Formats the absolute value: the caller adds the "+" or "−" sign.
 */
export function formatBRL(reais: number): string {
  return brl.format(Math.abs(reais)).replace(/\s/g, " ")
}
