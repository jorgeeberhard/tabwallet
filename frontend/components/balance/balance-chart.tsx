"use client"

import { CartesianGrid, Line, LineChart, ReferenceLine, Tooltip, XAxis, YAxis } from "recharts"
import type { TooltipContentProps } from "recharts"

import { Money } from "@/components/money"
import { ChartContainer, type ChartConfig } from "@/components/ui/chart"
import type { Granularity } from "@/lib/balance-series-api"
import type { SeriesRow } from "@/lib/balance-series"
import { formatDate } from "@/lib/format"

const CONFIG = {
  history: { label: "Saldo atual (histórico)", color: "var(--chart-1)" },
  projected: { label: "Saldo projetado", color: "var(--chart-5)" },
} satisfies ChartConfig

const SHORT_MONTHS = [
  "jan",
  "fev",
  "mar",
  "abr",
  "mai",
  "jun",
  "jul",
  "ago",
  "set",
  "out",
  "nov",
  "dez",
]

/** The label under an axis tick: "15/01" for days, "jan/26" for months. */
export function tickLabel(isoDay: string, granularity: Granularity): string {
  const [year, month, day] = isoDay.split("-")
  return granularity === "day"
    ? `${day}/${month}`
    : `${SHORT_MONTHS[Number(month) - 1]}/${year.slice(2)}`
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
  const row = payload[0].payload as SeriesRow
  const value = row.projected ?? row.history
  if (value === undefined) return null
  return (
    <div className="grid gap-1 rounded-lg border bg-background px-3 py-2 text-xs shadow-xl">
      <span className="font-medium">{formatDate(row.date)}</span>
      <Money value={value} showSign={false} size="sm" />
      <span className="text-muted-foreground">
        {row.kind === "projected" ? "Saldo projetado" : "Saldo atual (histórico)"}
      </span>
    </div>
  )
}

export interface BalanceChartProps {
  rows: SeriesRow[]
  granularity: Granularity
  /** Today, where the solid line (history) hands over to the dashed one (projected). */
  today: string
  /** Text for assistive technology; the same data is in the table. */
  summary: string
}

/**
 * The balance over time: a solid line for the history and a dashed, neutral
 * line for the projection, with "Hoje" marked between them. The distinction
 * does not rely on colour alone (dash, vertical mark and a written legend),
 * and the table next to it carries every value.
 */
export function BalanceChart({ rows, granularity, today, summary }: BalanceChartProps) {
  return (
    <div className="flex flex-col gap-3">
      <ChartContainer
        config={CONFIG}
        className="aspect-auto h-72 w-full"
        role="img"
        aria-label={summary}
      >
        <LineChart data={rows} margin={{ top: 16, right: 16, bottom: 0, left: 0 }}>
          <CartesianGrid vertical={false} />
          <XAxis
            dataKey="date"
            tickLine={false}
            axisLine={false}
            minTickGap={28}
            tickFormatter={(value: string) => tickLabel(value, granularity)}
          />
          <YAxis
            tickLine={false}
            axisLine={false}
            width={76}
            tickFormatter={(value: number) => axisMoney(value)}
          />
          <Tooltip content={ChartTooltip} />
          <ReferenceLine y={0} stroke="var(--border)" />
          <ReferenceLine
            x={today}
            stroke="var(--muted-foreground)"
            strokeDasharray="2 3"
            label={{ value: "Hoje", position: "insideTopRight", fill: "var(--muted-foreground)" }}
          />
          <Line
            dataKey="history"
            name="Saldo atual (histórico)"
            type="monotone"
            stroke="var(--color-history)"
            strokeWidth={2}
            dot={false}
            isAnimationActive={false}
          />
          <Line
            dataKey="projected"
            name="Saldo projetado"
            type="monotone"
            stroke="var(--color-projected)"
            strokeWidth={2}
            strokeDasharray="6 4"
            dot={false}
            isAnimationActive={false}
          />
        </LineChart>
      </ChartContainer>
      <ul className="flex flex-wrap gap-x-6 gap-y-1 text-sm text-muted-foreground">
        <li className="flex items-center gap-2">
          <svg width="28" height="8" aria-hidden="true">
            <line x1="0" y1="4" x2="28" y2="4" stroke="var(--chart-1)" strokeWidth="2" />
          </svg>
          Linha contínua: saldo atual ao longo do tempo (histórico)
        </li>
        <li className="flex items-center gap-2">
          <svg width="28" height="8" aria-hidden="true">
            <line
              x1="0"
              y1="4"
              x2="28"
              y2="4"
              stroke="var(--chart-5)"
              strokeWidth="2"
              strokeDasharray="6 4"
            />
          </svg>
          Linha tracejada: saldo projetado
        </li>
      </ul>
    </div>
  )
}
