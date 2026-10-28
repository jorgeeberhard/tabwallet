"use client"

import { useQuery } from "@tanstack/react-query"
import { ChartLineIcon, TriangleAlertIcon } from "lucide-react"
import { useMemo, useState } from "react"

import { BalanceChart } from "@/components/balance/balance-chart"
import { BalanceTable } from "@/components/balance/balance-table"
import { Money } from "@/components/money"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader } from "@/components/ui/card"
import { Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle } from "@/components/ui/empty"
import { Field, FieldLabel } from "@/components/ui/field"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { Skeleton } from "@/components/ui/skeleton"
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group"
import {
  BALANCE_SERIES_KEY,
  type Granularity,
  getBalanceHistory,
  getProjectedBalance,
} from "@/lib/balance-series-api"
import {
  buildRows,
  HISTORY_PERIODS,
  historyStart,
  lowestPoint,
  PROJECTION_PERIODS,
  projectionEnd,
} from "@/lib/balance-series"
import { formatDate, formatMonth } from "@/lib/format"
import { todayInSaoPaulo } from "@/lib/today"

/** Where each granularity starts: a month of days, a year of months back; a quarter of days, a year of months ahead. */
const DEFAULT_PERIODS: Record<Granularity, { history: string; projection: string }> = {
  day: { history: "30", projection: "90" },
  month: { history: "12", projection: "12" },
}

function dateLabel(isoDay: string, granularity: Granularity): string {
  return granularity === "day" ? formatDate(isoDay) : formatMonth(isoDay)
}

/**
 * UC10: how the balance evolved up to today and where it is expected to go in
 * the next 12 months, by day or by month. The solid line is the Current
 * Balance of the past; the dashed one is the Projected Balance (Projections
 * and Occurrences of fixed transactions; purchases not yet recorded are not
 * known, so they are not in it).
 */
export function BalanceEvolution() {
  const [granularity, setGranularity] = useState<Granularity>("month")
  const [historyId, setHistoryId] = useState(DEFAULT_PERIODS.month.history)
  const [projectionId, setProjectionId] = useState(DEFAULT_PERIODS.month.projection)

  const historyPeriod = HISTORY_PERIODS[granularity].find((option) => option.id === historyId)
  const projectionPeriod = PROJECTION_PERIODS[granularity].find(
    (option) => option.id === projectionId,
  )

  // The API decides what "today" is; this one only picks the period to ask
  // for, and the projection answer carries the real `today` back.
  const requestToday = todayInSaoPaulo()
  const from = historyStart(requestToday, granularity, historyPeriod?.amount ?? 1)
  const to = projectionEnd(requestToday, granularity, projectionPeriod?.amount ?? 1)

  const history = useQuery({
    queryKey: [...BALANCE_SERIES_KEY, "history", granularity, from],
    queryFn: () => getBalanceHistory({ granularity, from }),
  })
  const projection = useQuery({
    queryKey: [...BALANCE_SERIES_KEY, "projection", granularity, to],
    queryFn: () => getProjectedBalance({ granularity, to }),
  })

  function changeGranularity(next: Granularity) {
    setGranularity(next)
    setHistoryId(DEFAULT_PERIODS[next].history)
    setProjectionId(DEFAULT_PERIODS[next].projection)
  }

  const historyData = history.data
  const projectionData = projection.data
  const data = historyData && projectionData ? { history: historyData, projection: projectionData } : null
  const rows = useMemo(
    () =>
      historyData && projectionData
        ? buildRows(
            historyData.points,
            projectionData.points,
            projectionData.today,
            projectionData.currentBalance,
          )
        : [],
    [historyData, projectionData],
  )

  const failed = history.isError || projection.isError

  return (
    <section className="flex flex-col gap-6">
      <div className="flex flex-col gap-1">
        <h1 className="font-display text-3xl font-bold tracking-tight">Evolução do saldo</h1>
        <p className="max-w-prose text-muted-foreground">
          Como seu saldo chegou até hoje e para onde ele vai nos próximos meses, com as projeções e
          os lançamentos fixos. Compras no cartão ainda não lançadas não entram na projeção.
        </p>
      </div>

      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        <ToggleGroup
          type="single"
          aria-label="Intervalo dos pontos"
          value={granularity}
          onValueChange={(value) => value && changeGranularity(value as Granularity)}
          spacing={0}
          className="w-full rounded-lg bg-muted p-1 sm:col-span-2 lg:col-span-1 lg:w-auto"
        >
          <ToggleGroupItem
            value="month"
            className="h-9 flex-1 data-[state=on]:bg-card data-[state=on]:shadow-sm lg:flex-none lg:px-4"
          >
            Por mês
          </ToggleGroupItem>
          <ToggleGroupItem
            value="day"
            className="h-9 flex-1 data-[state=on]:bg-card data-[state=on]:shadow-sm lg:flex-none lg:px-4"
          >
            Por dia
          </ToggleGroupItem>
        </ToggleGroup>

        <Field>
          <FieldLabel htmlFor="history-period">Histórico</FieldLabel>
          <Select value={historyId} onValueChange={setHistoryId}>
            <SelectTrigger id="history-period" className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {HISTORY_PERIODS[granularity].map((option) => (
                <SelectItem key={option.id} value={option.id}>
                  {option.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </Field>

        <Field>
          <FieldLabel htmlFor="projection-period">Projeção</FieldLabel>
          <Select value={projectionId} onValueChange={setProjectionId}>
            <SelectTrigger id="projection-period" className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {PROJECTION_PERIODS[granularity].map((option) => (
                <SelectItem key={option.id} value={option.id}>
                  {option.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </Field>
      </div>

      {failed ? (
        <Alert variant="destructive">
          <TriangleAlertIcon strokeWidth={1.75} />
          <AlertDescription>
            <p>Não foi possível carregar a evolução do saldo. Verifique a conexão e tente de novo.</p>
            <Button
              size="sm"
              variant="outline"
              className="mt-2"
              onClick={() => {
                void history.refetch()
                void projection.refetch()
              }}
            >
              Tentar de novo
            </Button>
          </AlertDescription>
        </Alert>
      ) : !data ? (
        <div className="flex flex-col gap-4" aria-label="Carregando a evolução do saldo">
          <Skeleton className="h-24 w-full" />
          <Skeleton className="h-72 w-full" />
        </div>
      ) : rows.length === 0 ? (
        <Empty className="border">
          <EmptyHeader>
            <EmptyMedia variant="icon">
              <ChartLineIcon strokeWidth={1.75} />
            </EmptyMedia>
            <EmptyTitle>Nada para mostrar ainda</EmptyTitle>
            <EmptyDescription>Lance receitas e despesas para ver o saldo evoluir.</EmptyDescription>
          </EmptyHeader>
        </Empty>
      ) : (
        <Loaded
          rows={rows}
          granularity={granularity}
          today={data.projection.today}
          currentBalance={data.projection.currentBalance}
          projection={data.projection.points}
          projectionTo={data.projection.to}
        />
      )}
    </section>
  )
}

function Loaded({
  rows,
  granularity,
  today,
  currentBalance,
  projection,
  projectionTo,
}: {
  rows: ReturnType<typeof buildRows>
  granularity: Granularity
  today: string
  currentBalance: number
  projection: { date: string; balance: number }[]
  projectionTo: string
}) {
  const last = projection.at(-1)
  const lowest = lowestPoint(projection)
  const summary =
    `Gráfico do saldo ${granularity === "day" ? "por dia" : "por mês"}: histórico até hoje e projeção até ` +
    `${formatDate(projectionTo)}. Os mesmos valores estão na tabela abaixo.`

  return (
    <>
      <div className="grid gap-4 sm:grid-cols-3">
        <SummaryCard label="Saldo atual" caption="hoje">
          <Money value={currentBalance} showSign={false} size="lg" />
        </SummaryCard>
        {last && (
          <SummaryCard
            label="Saldo projetado"
            caption={`${granularity === "day" ? "em" : "no fim de"} ${dateLabel(last.date, granularity)}`}
          >
            <Money value={last.balance} showSign={false} size="lg" />
          </SummaryCard>
        )}
        {lowest && (
          <SummaryCard
            label="Menor saldo projetado"
            caption={`${granularity === "day" ? "em" : "no fim de"} ${dateLabel(lowest.date, granularity)}`}
          >
            <Money value={lowest.balance} showSign={false} size="lg" />
          </SummaryCard>
        )}
      </div>

      <Card>
        <CardContent className="pt-6">
          <BalanceChart rows={rows} granularity={granularity} today={today} summary={summary} />
        </CardContent>
      </Card>

      <details className="group rounded-lg border bg-card">
        <summary className="cursor-pointer px-4 py-3 text-sm font-semibold">
          Ver os valores em tabela
        </summary>
        <div className="px-4 pb-4">
          <BalanceTable rows={rows} granularity={granularity} today={today} />
        </div>
      </details>
    </>
  )
}

function SummaryCard({
  label,
  caption,
  children,
}: {
  label: string
  caption: string
  children: React.ReactNode
}) {
  return (
    <Card>
      <CardHeader>
        <h2 className="text-sm font-medium text-muted-foreground">{label}</h2>
      </CardHeader>
      <CardContent className="flex flex-col gap-1">
        {children}
        <p className="text-sm text-muted-foreground">{caption}</p>
      </CardContent>
    </Card>
  )
}
