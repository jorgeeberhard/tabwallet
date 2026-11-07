import { formatMonthOfYear } from "@/lib/format"

/**
 * Pure helpers for the period a report covers: whole months (`YYYY-MM`), as
 * the API takes them. "This month" comes from the same "today" the API uses.
 */

/** The longest period the API accepts, in months. Keep in sync with the back-end. */
export const MAX_REPORT_MONTHS = 36

/** The month a calendar day (`YYYY-MM-DD`) belongs to, as `YYYY-MM`. */
export function monthOf(isoDay: string): string {
  return isoDay.slice(0, 7)
}

/** `yearMonth` moved by whole months (negative goes back), as `YYYY-MM`. */
export function shiftMonth(yearMonth: string, months: number): string {
  const [year, month] = yearMonth.split("-").map(Number)
  const index = year * 12 + (month - 1) + months
  const shiftedYear = Math.floor(index / 12)
  const shiftedMonth = (index % 12) + 1
  return `${String(shiftedYear).padStart(4, "0")}-${String(shiftedMonth).padStart(2, "0")}`
}

/** How many months the period has, both ends counted; zero or less if it is upside down. */
export function monthsBetween(from: string, to: string): number {
  const [fromYear, fromMonth] = from.split("-").map(Number)
  const [toYear, toMonth] = to.split("-").map(Number)
  return (toYear - fromYear) * 12 + (toMonth - fromMonth) + 1
}

export interface PeriodPreset {
  id: string
  label: string
  /** Months from the current one to the first month of the period. */
  startOffset: number
  /** Months from the current one to the last month of the period. */
  endOffset: number
}

export const PERIOD_PRESETS: PeriodPreset[] = [
  { id: "this-month", label: "Este mês", startOffset: 0, endOffset: 0 },
  { id: "last-month", label: "Mês passado", startOffset: -1, endOffset: -1 },
  { id: "last-3", label: "Últimos 3 meses", startOffset: -2, endOffset: 0 },
  { id: "last-6", label: "Últimos 6 meses", startOffset: -5, endOffset: 0 },
  { id: "last-12", label: "Últimos 12 meses", startOffset: -11, endOffset: 0 },
  { id: "next-3", label: "Este mês e os próximos 2", startOffset: 0, endOffset: 2 },
  { id: "next-6", label: "Este mês e os próximos 5", startOffset: 0, endOffset: 5 },
]

/** The id the picker uses for a period the person typed month by month. */
export const CUSTOM_PERIOD_ID = "custom"

export interface PeriodRange {
  from: string
  to: string
}

/** The months a preset covers when `currentMonth` is the current month. */
export function presetRange(currentMonth: string, presetId: string): PeriodRange {
  const preset = PERIOD_PRESETS.find((option) => option.id === presetId) ?? PERIOD_PRESETS[0]
  return {
    from: shiftMonth(currentMonth, preset.startOffset),
    to: shiftMonth(currentMonth, preset.endOffset),
  }
}

export interface MonthOption {
  value: string
  label: string
}

/** Months to pick from for a custom period: three years back and a year ahead. */
export function monthOptions(currentMonth: string): MonthOption[] {
  const options: MonthOption[] = []
  for (let offset = -(MAX_REPORT_MONTHS - 1); offset <= 12; offset += 1) {
    const value = shiftMonth(currentMonth, offset)
    options.push({ value, label: formatMonthOfYear(value) })
  }
  return options
}

export type PeriodProblem = "order" | "too_long"

/** What is wrong with a period, if anything; the API refuses the same cases. */
export function periodProblem({ from, to }: PeriodRange): PeriodProblem | null {
  const months = monthsBetween(from, to)
  if (months < 1) return "order"
  if (months > MAX_REPORT_MONTHS) return "too_long"
  return null
}

/** What the person picked: a preset, or (id `custom`) two months. */
export interface PeriodChoice {
  presetId: string
  /** Only for the custom period. */
  from?: string
  to?: string
}

/** The months a choice covers when `currentMonth` is the current month. */
export function resolvePeriod(currentMonth: string, choice: PeriodChoice): PeriodRange {
  if (choice.presetId === CUSTOM_PERIOD_ID) {
    return { from: choice.from ?? currentMonth, to: choice.to ?? currentMonth }
  }
  return presetRange(currentMonth, choice.presetId)
}

/** "janeiro de 2026" for one month, "janeiro a março de 2026" or "novembro de 2025 a fevereiro de 2026" for more. */
export function describePeriod({ from, to }: PeriodRange): string {
  if (from === to) return formatMonthOfYear(from)
  const [fromYear] = from.split("-")
  const [toYear] = to.split("-")
  if (fromYear === toYear) {
    const fromName = formatMonthOfYear(from).split(" de ")[0]
    return `${fromName} a ${formatMonthOfYear(to)}`
  }
  return `${formatMonthOfYear(from)} a ${formatMonthOfYear(to)}`
}
