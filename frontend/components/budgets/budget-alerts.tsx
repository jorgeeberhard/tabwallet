"use client"

import { useQuery } from "@tanstack/react-query"
import { PiggyBankIcon, TriangleAlertIcon } from "lucide-react"
import Link from "next/link"

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Badge } from "@/components/ui/badge"
import { budgetStatusKey, type BudgetStatusLine, getBudgetStatus } from "@/lib/budgets-api"
import { formatBRLCents, formatMonthOfYear } from "@/lib/format"

/** How many alerts the home page spells out before summing up the rest. */
const MAX_VISIBLE_ALERTS = 3

/** The current month's budget status. Never served stale: the alert is derived on every read. */
function useCurrentBudgetStatus() {
  return useQuery({
    queryKey: budgetStatusKey(null),
    queryFn: () => getBudgetStatus(null),
    staleTime: 0,
  })
}

/** The categories over their limit, the furthest over first. */
function exceededLines(lines: BudgetStatusLine[]): BudgetStatusLine[] {
  return lines
    .filter((line) => line.exceeded)
    .sort((a, b) => a.remainingCents - b.remainingCents)
}

/**
 * The budget alerts on the home page, one message per category that
 * spent more than its limit this month (projections included). Shows nothing
 * while there is no alert or while the status is loading; the alert is only
 * inside the app, never by e-mail or push.
 */
export function BudgetAlerts() {
  const status = useCurrentBudgetStatus()
  if (!status.data) return null
  const exceeded = exceededLines(status.data.lines)
  if (exceeded.length === 0) return null

  const visible = exceeded.slice(0, MAX_VISIBLE_ALERTS)
  const hidden = exceeded.length - visible.length
  return (
    <section aria-label="Alertas de orçamento" className="flex max-w-xl flex-col gap-2">
      {visible.map((line) => (
        <Alert key={line.category.id} variant="destructive">
          <TriangleAlertIcon strokeWidth={1.75} />
          <AlertTitle>
            Você passou {formatBRLCents(-line.remainingCents)} do limite de {line.category.name}.
          </AlertTitle>
          <AlertDescription>
            Gasto de {formatBRLCents(line.spentCents)} em {formatBRLCents(line.limitCents)} de limite em{" "}
            {formatMonthOfYear(status.data.month)}.
          </AlertDescription>
        </Alert>
      ))}
      <p className="text-sm text-muted-foreground">
        {hidden > 0 && `E mais ${hidden} ${hidden === 1 ? "categoria" : "categorias"} acima do limite. `}
        <Link href="/orcamentos" className="-my-2 inline-block py-2 font-semibold underline underline-offset-4">
          Ver orçamentos
        </Link>
      </p>
    </section>
  )
}

/**
 * The "Orçamentos" shortcut of the home page, with a badge counting the
 * categories over their limit this month.
 */
export function BudgetsShortcut({ className }: { className: string }) {
  const status = useCurrentBudgetStatus()
  const count = status.data?.exceededCount ?? 0
  return (
    <Link href="/orcamentos" className={className}>
      <PiggyBankIcon strokeWidth={1.75} className="size-4" />
      Orçamentos
      {count > 0 && (
        <Badge variant="expense">{count} acima do limite</Badge>
      )}
    </Link>
  )
}
