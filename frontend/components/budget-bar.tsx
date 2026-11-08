import { cn } from "cn"
import { TriangleAlertIcon } from "lucide-react"

import { Money } from "@/components/money"
import { Progress } from "@/components/ui/progress"
import { centsToReais } from "@/lib/amount"
import { formatBRLCents } from "@/lib/format"

export type BudgetBarState = "ok" | "warning" | "exceeded"

/** From which share of the limit the bar warns (Design System default). */
export const DEFAULT_WARN_AT = 0.8

/**
 * `exceeded` only when the spending is above the limit (exactly at the limit
 * is not); `warning` from `warnAt` of it; `ok` below. Amounts in cents.
 */
export function budgetBarState(
  spentCents: number,
  limitCents: number,
  warnAt: number = DEFAULT_WARN_AT,
): BudgetBarState {
  if (spentCents > limitCents) return "exceeded"
  if (spentCents >= limitCents * warnAt) return "warning"
  return "ok"
}

/** The share of the limit already spent, as a whole percent rounded down. */
export function budgetPercent(spentCents: number, limitCents: number): number {
  return Math.floor((spentCents * 100) / limitCents)
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
  /** Spent in the month, in cents. */
  spentCents: number
  /** The monthly limit, in cents. */
  limitCents: number
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
 * `BudgetBar`). The state is always written next to the bar, never only
 * coloured: "41% usado", "92% do limite" with an alert icon, or "Limite
 * excedido em R$ 120,00".
 */
export function BudgetBar({
  category,
  spentCents,
  limitCents,
  warnAt = DEFAULT_WARN_AT,
  paused = false,
  className,
}: BudgetBarProps) {
  const state = budgetBarState(spentCents, limitCents, warnAt)
  const percent = budgetPercent(spentCents, limitCents)
  const label =
    state === "exceeded"
      ? `Limite excedido em ${formatBRLCents(spentCents - limitCents)}`
      : state === "warning"
        ? `${percent}% do limite`
        : `${percent}% usado`

  return (
    <div className={cn("flex flex-col gap-2", className)}>
      <div className="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
        <span className="font-medium">{category}</span>
        <span className="flex items-baseline gap-1 text-sm text-muted-foreground">
          <Money value={centsToReais(spentCents)} showSign={false} kind="neutral" size="sm" />
          de
          <Money value={centsToReais(limitCents)} showSign={false} kind="neutral" size="sm" />
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
