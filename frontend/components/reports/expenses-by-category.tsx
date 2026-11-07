"use client"

import { useQuery } from "@tanstack/react-query"
import { ChartPieIcon, TriangleAlertIcon } from "lucide-react"
import { useState } from "react"

import { CategoryChart } from "@/components/reports/category-chart"
import { CategoryTable } from "@/components/reports/category-table"
import { PeriodPicker } from "@/components/reports/period-picker"
import { Money } from "@/components/money"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { Button } from "@/components/ui/button"
import { Card, CardContent } from "@/components/ui/card"
import { Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle } from "@/components/ui/empty"
import { Skeleton } from "@/components/ui/skeleton"
import {
  type PeriodChoice,
  describePeriod,
  monthOf,
  periodProblem,
  resolvePeriod,
} from "@/lib/report-periods"
import { REPORTS_KEY, getExpensesByCategory } from "@/lib/reports-api"
import { useToday } from "@/lib/use-today"

/**
 * UC11 (a): what each Category spent in a period, by Reference Month. The
 * current month is the default. Card purchases, Installments and Charges
 * count in the month their invoice is due, Projections of the period are
 * included, and invoice payments are not expenses, so they are not here.
 */
export function ExpensesByCategory() {
  const [choice, setChoice] = useState<PeriodChoice>({ presetId: "this-month" })
  // The API decides what is a Projection; this "today" only picks the months to ask for.
  const today = useToday()
  const currentMonth = today ? monthOf(today) : null
  const range = currentMonth ? resolvePeriod(currentMonth, choice) : null
  const problem = range ? periodProblem(range) : null

  const report = useQuery({
    queryKey: [...REPORTS_KEY, "expenses-by-category", range?.from, range?.to],
    queryFn: () => getExpensesByCategory(range as { from: string; to: string }),
    enabled: range !== null && problem === null,
    // Reports are cheap to ask again and must not show yesterday's totals.
    staleTime: 0,
  })

  const periodLabel = range ? describePeriod(range) : ""

  return (
    <section aria-labelledby="expenses-by-category-title" className="flex flex-col gap-4">
      <div className="flex flex-col gap-1">
        <h2 id="expenses-by-category-title" className="font-display text-xl font-bold tracking-tight">
          Despesas por categoria
        </h2>
        <p className="max-w-prose text-muted-foreground">
          Para onde o dinheiro foi, no mês em que cada despesa conta: as do cartão contam no mês do
          vencimento da fatura. Inclui encargos e projeções; pagamentos de fatura e saldo rolado não
          são despesas.
        </p>
      </div>

      {range && currentMonth && (
        <PeriodPicker
          idPrefix="categories"
          label="Período"
          choice={choice}
          range={range}
          currentMonth={currentMonth}
          onChange={setChoice}
        />
      )}

      {problem ? null : report.isError ? (
        <Alert variant="destructive">
          <TriangleAlertIcon strokeWidth={1.75} />
          <AlertDescription>
            <p>Não foi possível carregar as despesas por categoria. Verifique a conexão e tente de novo.</p>
            <Button
              size="sm"
              variant="outline"
              className="mt-2"
              onClick={() => void report.refetch()}
            >
              Tentar de novo
            </Button>
          </AlertDescription>
        </Alert>
      ) : !report.data ? (
        <div className="flex flex-col gap-4" aria-label="Carregando as despesas por categoria">
          <Skeleton className="h-24 w-full" />
          <Skeleton className="h-64 w-full" />
        </div>
      ) : report.data.categories.length === 0 ? (
        <Empty className="border">
          <EmptyHeader>
            <EmptyMedia variant="icon">
              <ChartPieIcon strokeWidth={1.75} />
            </EmptyMedia>
            <EmptyTitle>Nenhuma despesa em {periodLabel}</EmptyTitle>
            <EmptyDescription>
              Lance despesas ou escolha outro período para ver a divisão por categoria.
            </EmptyDescription>
          </EmptyHeader>
        </Empty>
      ) : (
        <>
          <Card>
            <CardContent className="flex flex-col gap-1">
              <h3 className="text-sm font-medium text-muted-foreground">
                Total de despesas em {periodLabel}
              </h3>
              <Money value={report.data.total} kind="expense" showSign={false} size="lg" />
              {report.data.projectedCents > 0 && (
                <p className="text-sm text-muted-foreground">
                  Inclui <Money value={report.data.projected} kind="neutral" showSign={false} size="sm" />{" "}
                  de projeção (datas depois de hoje).
                </p>
              )}
            </CardContent>
          </Card>
          <Card>
            <CardContent className="pt-6">
              <CategoryChart
                categories={report.data.categories}
                summary={`Gráfico de barras das despesas por categoria em ${periodLabel}. Os mesmos valores estão na tabela abaixo.`}
              />
            </CardContent>
          </Card>
          <details className="group rounded-lg border bg-card">
            <summary className="cursor-pointer px-4 py-3 text-sm font-semibold">
              Ver os valores em tabela
            </summary>
            <div className="px-4 pb-4">
              <CategoryTable report={report.data} periodLabel={periodLabel} />
            </div>
          </details>
        </>
      )}
    </section>
  )
}
