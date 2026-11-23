"use client"

import { Bar, BarChart, CartesianGrid, Tooltip, XAxis, YAxis } from "recharts"
import type { TooltipContentProps } from "recharts"

import { Money } from "@/components/money"
import { ChartContainer, type ChartConfig } from "@/components/ui/chart"
import type { CategoryExpense } from "@/lib/reports-api"

const CONFIG = {
  actual: { label: "Já lançado", color: "var(--chart-3)" },
  projected: { label: "Projeção", color: "var(--chart-3)" },
} satisfies ChartConfig

const compact = new Intl.NumberFormat("pt-BR", {
  style: "currency",
  currency: "BRL",
  notation: "compact",
  maximumFractionDigits: 1,
})

function axisMoney(reais: number): string {
  return compact.format(reais).replace(/\s/g, " ")
}

interface BarRow {
  name: string
  actual: number
  projected: number
  total: number
}

function toRow(category: CategoryExpense): BarRow {
  return {
    name: category.name,
    actual: category.total - category.projected,
    projected: category.projected,
    total: category.total,
  }
}

function ChartTooltip({ active, payload }: TooltipContentProps) {
  if (!active || !payload?.length) return null
  const row = payload[0].payload as BarRow
  return (
    <div className="grid gap-1 rounded-lg border bg-background px-3 py-2 text-xs shadow-xl">
      <span className="font-medium">{row.name}</span>
      <Money value={row.total} kind="expense" showSign={false} size="sm" />
      {row.projected > 0 && (
        <span className="text-muted-foreground">
          Inclui <Money value={row.projected} kind="neutral" showSign={false} size="sm" /> de projeção
        </span>
      )}
    </div>
  )
}

export interface CategoryChartProps {
  categories: CategoryExpense[]
  /** Text for assistive technology; the same data is in the table. */
  summary: string
}

/**
 * Expenses by Category as horizontal bars, the biggest first, with the name
 * of each Category written beside its bar (no colour key to decode). The part
 * of a bar that is a Projection is lighter, and the legend says so in words.
 * One hue only: the categories are told apart by their labels, not by colour.
 */
export function CategoryChart({ categories, summary }: CategoryChartProps) {
  const rows = categories.map(toRow)
  const hasProjection = rows.some((row) => row.projected > 0)
  return (
    <div className="flex flex-col gap-3">
      <ChartContainer
        config={CONFIG}
        className="aspect-auto w-full"
        style={{ height: Math.max(160, rows.length * 44 + 40) }}
        role="img"
        aria-label={summary}
      >
        <BarChart
          data={rows}
          layout="vertical"
          margin={{ top: 4, right: 16, bottom: 0, left: 0 }}
          barCategoryGap="25%"
        >
          <CartesianGrid horizontal={false} />
          <XAxis
            type="number"
            tickLine={false}
            axisLine={false}
            tickFormatter={(value: number) => axisMoney(value)}
            minTickGap={24}
          />
          <YAxis
            type="category"
            dataKey="name"
            tickLine={false}
            axisLine={false}
            width={104}
            interval={0}
            tickFormatter={(value: string) => (value.length > 15 ? `${value.slice(0, 14)}…` : value)}
          />
          <Tooltip content={ChartTooltip} cursor={{ fill: "var(--muted)" }} />
          <Bar
            dataKey="actual"
            name="Já lançado"
            stackId="spent"
            fill="var(--color-actual)"
            isAnimationActive={false}
          />
          <Bar
            dataKey="projected"
            name="Projeção"
            stackId="spent"
            fill="var(--color-projected)"
            fillOpacity={0.4}
            radius={[0, 4, 4, 0]}
            isAnimationActive={false}
          />
        </BarChart>
      </ChartContainer>
      <ul className="flex flex-wrap gap-x-6 gap-y-1 text-sm text-muted-foreground">
        <li className="flex items-center gap-2">
          <span className="inline-block h-3 w-6 rounded-sm bg-chart-3" aria-hidden="true" />
          Barra cheia: despesas já lançadas, no mês em que contam
        </li>
        {hasProjection && (
          <li className="flex items-center gap-2">
            <span className="inline-block h-3 w-6 rounded-sm bg-chart-3/40" aria-hidden="true" />
            Barra clara: projeção (data depois de hoje)
          </li>
        )}
      </ul>
    </div>
  )
}
