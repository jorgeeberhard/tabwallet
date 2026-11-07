"use client"

import { Bar, BarChart, CartesianGrid, Cell, Tooltip, XAxis, YAxis } from "recharts"
import type { TooltipContentProps } from "recharts"

import { Money } from "@/components/money"
import { ChartContainer, type ChartConfig } from "@/components/ui/chart"
import { formatMonthOfYear } from "@/lib/format"
import type { MonthTotals } from "@/lib/reports-api"

const CONFIG = {
  income: { label: "Receitas", color: "var(--chart-2)" },
  expense: { label: "Despesas", color: "var(--chart-3)" },
} satisfies ChartConfig

const SHORT_MONTHS = ["jan", "fev", "mar", "abr", "mai", "jun", "jul", "ago", "set", "out", "nov", "dez"]

/** "2026-01" as "jan/26". */
export function shortMonth(yearMonth: string): string {
  const [year, month] = yearMonth.split("-")
  return `${SHORT_MONTHS[Number(month) - 1]}/${year.slice(2)}`
}

const compact = new Intl.NumberFormat("pt-BR", {
  style: "currency",
  currency: "BRL",
  notation: "compact",
  maximumFractionDigits: 1,
})

function axisMoney(reais: number): string {
  return compact.format(reais).replace(/\s/g, " ")
}

function ChartTooltip({ active, payload }: TooltipContentProps) {
  if (!active || !payload?.length) return null
  const month = payload[0].payload as MonthTotals
  return (
    <div className="grid gap-1 rounded-lg border bg-background px-3 py-2 text-xs shadow-xl">
      <span className="font-medium">{formatMonthOfYear(month.month)}</span>
      <span className="flex items-center justify-between gap-4">
        Receitas <Money value={month.income} kind="income" showSign={false} size="sm" />
      </span>
      <span className="flex items-center justify-between gap-4">
        Despesas <Money value={month.expense} kind="expense" showSign={false} size="sm" />
      </span>
      {(month.projectedIncomeCents > 0 || month.projectedExpenseCents > 0) && (
        <span className="text-muted-foreground">Inclui projeções</span>
      )}
    </div>
  )
}

export interface IncomeExpenseChartProps {
  months: MonthTotals[]
  /** Text for assistive technology; the same data is in the table. */
  summary: string
}

/**
 * Income and expenses side by side for each month, in the colours the Design
 * System gives them (income green, expense red) with a written legend. A
 * lighter bar means the month includes Projections; the table next to it
 * says so in words and carries every value.
 */
export function IncomeExpenseChart({ months, summary }: IncomeExpenseChartProps) {
  const hasProjection = months.some(
    (month) => month.projectedIncomeCents > 0 || month.projectedExpenseCents > 0,
  )
  return (
    <div className="flex flex-col gap-3">
      <ChartContainer
        config={CONFIG}
        className="aspect-auto h-72 w-full"
        role="img"
        aria-label={summary}
      >
        <BarChart data={months} margin={{ top: 8, right: 16, bottom: 0, left: 0 }} barGap={2}>
          <CartesianGrid vertical={false} />
          <XAxis
            dataKey="month"
            tickLine={false}
            axisLine={false}
            minTickGap={16}
            tickFormatter={(value: string) => shortMonth(value)}
          />
          <YAxis
            tickLine={false}
            axisLine={false}
            width={76}
            tickFormatter={(value: number) => axisMoney(value)}
          />
          <Tooltip content={ChartTooltip} cursor={{ fill: "var(--muted)" }} />
          <Bar dataKey="income" name="Receitas" fill="var(--color-income)" radius={[4, 4, 0, 0]} isAnimationActive={false}>
            {months.map((month) => (
              <Cell key={month.month} fillOpacity={month.projectedIncomeCents > 0 ? 0.45 : 1} />
            ))}
          </Bar>
          <Bar dataKey="expense" name="Despesas" fill="var(--color-expense)" radius={[4, 4, 0, 0]} isAnimationActive={false}>
            {months.map((month) => (
              <Cell key={month.month} fillOpacity={month.projectedExpenseCents > 0 ? 0.45 : 1} />
            ))}
          </Bar>
        </BarChart>
      </ChartContainer>
      <ul className="flex flex-wrap gap-x-6 gap-y-1 text-sm text-muted-foreground">
        <li className="flex items-center gap-2">
          <span className="inline-block h-3 w-6 rounded-sm bg-chart-2" aria-hidden="true" />
          Receitas
        </li>
        <li className="flex items-center gap-2">
          <span className="inline-block h-3 w-6 rounded-sm bg-chart-3" aria-hidden="true" />
          Despesas
        </li>
        {hasProjection && (
          <li className="flex items-center gap-2">
            <span className="inline-block h-3 w-6 rounded-sm bg-foreground/30" aria-hidden="true" />
            Barra clara: o mês inclui projeções
          </li>
        )}
      </ul>
    </div>
  )
}
