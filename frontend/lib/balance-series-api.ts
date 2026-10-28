import { centsToReais } from "@/lib/amount"
import { api } from "@/lib/api"
import { BALANCE_KEY } from "@/lib/balance-api"

export type Granularity = "day" | "month"

/** The balance at the end of one day (or month). */
export interface BalancePoint {
  /** A calendar day, `YYYY-MM-DD`: the last day of the month for monthly series. */
  date: string
  balanceCents: number
  /** Balance in reais, for the Design System components and the chart. */
  balance: number
}

export interface BalanceHistory {
  granularity: Granularity
  from: string
  to: string
  points: BalancePoint[]
}

export interface ProjectedBalance {
  granularity: Granularity
  /** Today in America/Sao_Paulo, as the API sees it. */
  today: string
  to: string
  currentBalanceCents: number
  currentBalance: number
  points: BalancePoint[]
}

interface PointPayload {
  date: string
  balance_cents: number
}

interface HistoryPayload {
  granularity: Granularity
  from: string
  to: string
  points: PointPayload[]
}

interface ProjectionPayload {
  granularity: Granularity
  today: string
  to: string
  current_balance_cents: number
  points: PointPayload[]
}

function toPoints(points: PointPayload[]): BalancePoint[] {
  return points.map((point) => ({
    date: point.date,
    balanceCents: point.balance_cents,
    balance: centsToReais(point.balance_cents),
  }))
}

/** Query key prefix of the balance series; it sits under `BALANCE_KEY`, so anything that changes the balance refreshes them. */
export const BALANCE_SERIES_KEY = [...BALANCE_KEY, "series"] as const

export interface HistoryParams {
  granularity: Granularity
  /** First day, inclusive, `YYYY-MM-DD`. */
  from: string
}

export async function getBalanceHistory({
  granularity,
  from,
}: HistoryParams): Promise<BalanceHistory> {
  const payload = await api<HistoryPayload>(
    `/balance/history?granularity=${granularity}&from=${from}`,
  )
  return {
    granularity: payload.granularity,
    from: payload.from,
    to: payload.to,
    points: toPoints(payload.points),
  }
}

export interface ProjectionParams {
  granularity: Granularity
  /** Last day, inclusive, `YYYY-MM-DD`; at most 12 months ahead. */
  to: string
}

export async function getProjectedBalance({
  granularity,
  to,
}: ProjectionParams): Promise<ProjectedBalance> {
  const payload = await api<ProjectionPayload>(
    `/balance/projection?granularity=${granularity}&to=${to}`,
  )
  return {
    granularity: payload.granularity,
    today: payload.today,
    to: payload.to,
    currentBalanceCents: payload.current_balance_cents,
    currentBalance: centsToReais(payload.current_balance_cents),
    points: toPoints(payload.points),
  }
}
