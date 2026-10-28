import { cn } from "cn"

import { Money } from "@/components/money"
import { Card, CardContent, CardHeader } from "@/components/ui/card"

export interface BalanceCardProps {
  /** Balance in reais. Neutral ink; red with "−" only when negative. */
  balance: number
  /** Defaults to "Saldo atual". */
  label?: string
  /** Optional context shown beside the label, e.g. "Setembro de 2026". */
  period?: string
  className?: string
  /** Extra content under the figure, e.g. the Opening Balance line. */
  children?: React.ReactNode
}

/**
 * The balance (Design System `BalanceCard`): the first thing on the home page,
 * one per page. The month's income and expense of the Design System version
 * are left out until a ticket computes them.
 */
export function BalanceCard({
  balance,
  label = "Saldo atual",
  period,
  className,
  children,
}: BalanceCardProps) {
  return (
    <Card className={cn("w-full max-w-xl", className)}>
      <CardHeader>
        <div className="flex items-baseline justify-between gap-4">
          <h2 className="text-sm font-medium text-muted-foreground">{label}</h2>
          {period && <p className="text-sm text-muted-foreground">{period}</p>}
        </div>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        <Money value={balance} showSign={false} size="lg" />
        {children}
      </CardContent>
    </Card>
  )
}
