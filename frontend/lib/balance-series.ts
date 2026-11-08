import type { BalancePoint, Granularity } from "@/lib/balance-series-api"

/**
 * Pure helpers for the balance evolution view: the periods the person can
 * pick, and the rows the chart and the table draw. Nothing here recomputes a
 * balance; the API decides every value.
 */

export interface PeriodOption {
  id: string
  label: string
  /** Days (daily series) or months (monthly series). */
  amount: number
}

export const HISTORY_PERIODS: Record<Granularity, PeriodOption[]> = {
  day: [
    { id: "30", label: "Últimos 30 dias", amount: 30 },
    { id: "60", label: "Últimos 60 dias", amount: 60 },
    { id: "90", label: "Últimos 90 dias", amount: 90 },
  ],
  month: [
    { id: "6", label: "Últimos 6 meses", amount: 6 },
    { id: "12", label: "Últimos 12 meses", amount: 12 },
    { id: "24", label: "Últimos 24 meses", amount: 24 },
  ],
}

export const PROJECTION_PERIODS: Record<Granularity, PeriodOption[]> = {
  day: [
    { id: "30", label: "Próximos 30 dias", amount: 30 },
    { id: "90", label: "Próximos 90 dias", amount: 90 },
    { id: "180", label: "Próximos 180 dias", amount: 180 },
    { id: "365", label: "Próximos 365 dias", amount: 365 },
  ],
  month: [
    { id: "3", label: "Próximos 3 meses", amount: 3 },
    { id: "6", label: "Próximos 6 meses", amount: 6 },
    { id: "12", label: "Próximos 12 meses", amount: 12 },
  ],
}

function parse(isoDay: string): Date {
  const [year, month, day] = isoDay.split("-").map(Number)
  return new Date(Date.UTC(year, month - 1, day))
}

function format(date: Date): string {
  return date.toISOString().slice(0, 10)
}

/** `isoDay` moved by whole days, as `YYYY-MM-DD`. */
export function addDays(isoDay: string, days: number): string {
  const date = parse(isoDay)
  date.setUTCDate(date.getUTCDate() + days)
  return format(date)
}

/** First day of the month that is `months` after (or before, if negative) the month of `isoDay`. */
export function firstOfMonthShifted(isoDay: string, months: number): string {
  const date = parse(isoDay)
  return format(new Date(Date.UTC(date.getUTCFullYear(), date.getUTCMonth() + months, 1)))
}

/** Last day of the month that is `months` after the month of `isoDay`. */
export function lastOfMonthShifted(isoDay: string, months: number): string {
  return addDays(firstOfMonthShifted(isoDay, months + 1), -1)
}

/** First day of the history: `amount` days (or months) counting today (or this month). */
export function historyStart(today: string, granularity: Granularity, amount: number): string {
  return granularity === "day"
    ? addDays(today, -(amount - 1))
    : firstOfMonthShifted(today, -(amount - 1))
}

/** Last day of the projection: `amount` days ahead, or the end of the month `amount` months ahead. */
export function projectionEnd(today: string, granularity: Granularity, amount: number): string {
  return granularity === "day" ? addDays(today, amount) : lastOfMonthShifted(today, amount)
}

/** One row of the chart and of the table. */
export interface SeriesRow {
  date: string
  /** In reais; set on past rows. */
  history?: number
  /** In reais; set on projected rows (and on the day the two meet). */
  projected?: number
  kind: "history" | "projected"
}

/**
 * Joins the history and the projection into rows by date. The two lines
 * meet at the Current Balance: the projection also gets a row on `today`
 * (when the history has no row there, as in a monthly series) so the dashed
 * line leaves from where the solid one ends. Rows come out in date order.
 */
export function buildRows(
  history: BalancePoint[],
  projection: BalancePoint[],
  today: string,
  currentBalance: number,
): SeriesRow[] {
  const rows = new Map<string, SeriesRow>()
  for (const point of history) {
    rows.set(point.date, { date: point.date, history: point.balance, kind: "history" })
  }
  const joinsAtToday = history.some((point) => point.date === today)
  if (!joinsAtToday) {
    rows.set(today, { date: today, projected: currentBalance, kind: "history" })
  } else {
    const row = rows.get(today)
    if (row) row.projected = currentBalance
  }
  for (const point of projection) {
    const row = rows.get(point.date)
    if (row) {
      row.projected = point.balance
    } else {
      rows.set(point.date, { date: point.date, projected: point.balance, kind: "projected" })
    }
  }
  return [...rows.values()].sort((a, b) => a.date.localeCompare(b.date))
}

/** The point with the lowest balance, to point out a tight month; `null` without points. */
export function lowestPoint<T extends { balance: number }>(points: T[]): T | null {
  return points.reduce<T | null>(
    (lowest, point) => (lowest === null || point.balance < lowest.balance ? point : lowest),
    null,
  )
}
