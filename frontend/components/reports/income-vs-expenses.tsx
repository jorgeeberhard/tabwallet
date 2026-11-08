"use client"

import { useQuery } from "@tanstack/react-query"
import { ChartColumnIcon, TriangleAlertIcon } from "lucide-react"
import { useState } from "react"

import { IncomeExpenseChart } from "@/components/reports/income-expense-chart"
import { IncomeExpenseTable } from "@/components/reports/income-expense-table"
import { PeriodPicker } from "@/components/reports/period-picker"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { Button } from "@/components/ui/button"
import { Card, CardContent } from "@/components/ui/card"
import { Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle } from "@/components/ui/empty"
import { Skeleton } from "@/components/ui/skeleton"
import {
  type PeriodChoice,
  describePeriod,
  periodProblem,
  resolvePeriod,
} from "@/lib/report-periods"
import { REPORTS_KEY, getIncomeVsExpenses } from "@/lib/reports-api"
import { useToday } from "@/lib/use-today"
import { monthOf } from "@/lib/year-month"

/**
 * UC11 (b): income and expenses of each month, in series, by Reference Month.
 * Same rules as the expenses by Category; income includes the Occurrences of
 * recurring incomes (Receitas fixas).
 */
export function IncomeVsExpenses() {
  const [choice, setChoice] = useState<PeriodChoice>({ presetId: "last-6" })
  const today = useToday()
  const currentMonth = today ? monthOf(today) : null
  const range = currentMonth ? resolvePeriod(currentMonth, choice) : null
  const problem = range ? periodProblem(range) : null

  const report = useQuery({
    queryKey: [...REPORTS_KEY, "income-vs-expenses", range?.from, range?.to],
    queryFn: () => getIncomeVsExpenses(range as { from: string; to: string }),
    enabled: range !== null && problem === null,
    staleTime: 0,
  })

  const periodLabel = range ? describePeriod(range) : ""
  const hasAnything =
    report.data !== undefined && (report.data.incomeCents > 0 || report.data.expenseCents > 0)

  return (
    <section aria-labelledby="income-vs-expenses-title" className="flex flex-col gap-4">
      <div className="flex flex-col gap-1">
        <h2 id="income-vs-expenses-title" className="font-display text-xl font-bold tracking-tight">
          Receitas × despesas por mês
        </h2>
        <p className="max-w-prose text-muted-foreground">
          Quanto entrou e quanto saiu em cada mês, pelas mesmas regras das despesas por categoria.
          As receitas incluem as receitas fixas.
        </p>
      </div>

      {range && currentMonth && (
        <PeriodPicker
          idPrefix="series"
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
            <p>Não foi possível carregar receitas e despesas. Verifique a conexão e tente de novo.</p>
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
        <div className="flex flex-col gap-4" aria-label="Carregando receitas e despesas">
          <Skeleton className="h-72 w-full" />
        </div>
      ) : !hasAnything ? (
        <Empty className="border">
          <EmptyHeader>
            <EmptyMedia variant="icon">
              <ChartColumnIcon strokeWidth={1.75} />
            </EmptyMedia>
            <EmptyTitle>Nada lançado em {periodLabel}</EmptyTitle>
            <EmptyDescription>
              Lance receitas e despesas ou escolha outro período para comparar mês a mês.
            </EmptyDescription>
          </EmptyHeader>
        </Empty>
      ) : (
        <>
          <Card>
            <CardContent className="pt-6">
              <IncomeExpenseChart
                months={report.data.months}
                summary={`Gráfico de colunas com as receitas e as despesas de cada mês em ${periodLabel}. Os mesmos valores estão na tabela abaixo.`}
              />
            </CardContent>
          </Card>
          <details className="group rounded-lg border bg-card">
            <summary className="cursor-pointer px-4 py-3 text-sm font-semibold">
              Ver os valores em tabela
            </summary>
            <div className="px-4 pb-4">
              <IncomeExpenseTable report={report.data} />
            </div>
          </details>
        </>
      )}
    </section>
  )
}
