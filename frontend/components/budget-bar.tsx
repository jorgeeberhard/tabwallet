import { cn } from "cn"
import { TriangleAlertIcon } from "lucide-react"

import { Money } from "@/components/money"
import { Progress } from "@/components/ui/progress"
import { formatBRL } from "@/lib/format"

export type BudgetBarState = "ok" | "warning" | "exceeded"

/** From which share of the limit the bar warns (Design System default). */
export const DEFAULT_WARN_AT = 0.8

const cents = (reais: number) => Math.round(reais * 100)

/**
 * `exceeded` only when the spending is above the limit (exactly at the limit
 * is not); `warning` from `warnAt` of it; `ok` below. Amounts in reais.
 */
export function budgetBarState(
  spent: number,
  limit: number,
  warnAt: number = DEFAULT_WARN_AT,
): BudgetBarState {
  if (cents(spent) > cents(limit)) return "exceeded"
  if (cents(spent) >= cents(limit) * warnAt) return "warning"
  return "ok"
}

/** The share of the limit already spent, as a whole percent rounded down. */
export function budgetPercent(spent: number, limit: number): number {
  return Math.floor((cents(spent) * 100) / cents(limit))
}

const INDICATOR = {
  ok: "bg-income",
  warning: "bg-warning",
  exceeded: "bg-expense",
} as const

const TEXT = {
  ok: "text-income",
  warning: "text-warning",
  exceeded: "text-expense",
} as const

export interface BudgetBarProps {
  category: string
  /** Spent in the month, in reais. */
  spent: number
  /** The monthly limit, in reais. */
  limit: number
  warnAt?: number
  /**
   * The category is deactivated: its budget is kept but raises no alert, so
   * the bar stays neutral. The status is still written.
   */
  paused?: boolean
  className?: string
}

/**
 * The month's spending in one category against its limit (Design System
 * `BudgetBar`, UC12 and UC13). The state is always written next to the bar,
 * never only coloured: "41% usado", "92% do limite" with an alert icon, or
 * "Limite excedido em R$ 120,00".
 */
export function BudgetBar({
  category,
  spent,
  limit,
  warnAt = DEFAULT_WARN_AT,
  paused = false,
  className,
}: BudgetBarProps) {
  const state = budgetBarState(spent, limit, warnAt)
  const percent = budgetPercent(spent, limit)
  const label =
    state === "exceeded"
      ? `Limite excedido em ${formatBRL((cents(spent) - cents(limit)) / 100)}`
      : state === "warning"
        ? `${percent}% do limite`
        : `${percent}% usado`

  return (
    <div className={cn("flex flex-col gap-2", className)}>
      <div className="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
        <span className="font-medium">{category}</span>
        <span className="flex items-baseline gap-1 text-sm text-muted-foreground">
          <Money value={spent} showSign={false} kind="neutral" size="sm" />
          de
          <Money value={limit} showSign={false} kind="neutral" size="sm" />
        </span>
      </div>
      <Progress
        aria-label={`Gasto em ${category}`}
        value={Math.min(100, percent)}
        indicatorClassName={paused ? "bg-muted-foreground" : INDICATOR[state]}
      />
      <p
        className={cn(
          "flex items-center gap-1.5 text-sm font-medium",
          paused ? "text-muted-foreground" : TEXT[state],
        )}
      >
        {!paused && state !== "ok" && <TriangleAlertIcon strokeWidth={1.75} className="size-4" />}
        {label}
      </p>
    </div>
  )
}
