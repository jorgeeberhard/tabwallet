"use client"

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { ChevronLeftIcon, ChevronRightIcon, PiggyBankIcon, TriangleAlertIcon } from "lucide-react"
import { useState } from "react"
import { toast } from "sonner"

import { BudgetBar } from "@/components/budget-bar"
import { BudgetDialog, type BudgetDialogTarget } from "@/components/budgets/budget-dialog"
import { Alert, AlertDescription } from "@/components/ui/alert"
import {
  AlertDialog,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card } from "@/components/ui/card"
import { Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle } from "@/components/ui/empty"
import { Skeleton } from "@/components/ui/skeleton"
import { Spinner } from "@/components/ui/spinner"
import { budgetFailure } from "@/lib/budget-messages"
import { MONTHS_AHEAD, monthOf, shiftMonth } from "@/lib/budget-month"
import {
  BUDGETS_KEY,
  type BudgetStatusLine,
  budgetStatusKey,
  getBudgetStatus,
  removeBudget,
} from "@/lib/budgets-api"
import { CATEGORIES_KEY, listCategories } from "@/lib/categories-api"
import { formatMonthOfYear } from "@/lib/format"
import { useToday } from "@/lib/use-today"

/** Most critical first: alerts, then the closest to the limit. */
function byUrgency(a: BudgetStatusLine, b: BudgetStatusLine): number {
  if (a.exceeded !== b.exceeded) return a.exceeded ? -1 : 1
  return b.percentUsed - a.percentUsed
}

/** UC12 and UC13: the monthly limit of each expense category against what was spent. */
export function BudgetsScreen() {
  const today = useToday()
  if (today === null) {
    return (
      <section className="flex flex-col gap-6">
        <Skeleton className="h-10 w-64" />
        <Skeleton className="h-40 w-full" />
      </section>
    )
  }
  return <BudgetsContent currentMonth={monthOf(today)} />
}

function BudgetsContent({ currentMonth }: { currentMonth: string }) {
  const queryClient = useQueryClient()
  const [month, setMonth] = useState(currentMonth)
  const [dialog, setDialog] = useState<BudgetDialogTarget | null>(null)
  const [toRemove, setToRemove] = useState<BudgetStatusLine | null>(null)

  const status = useQuery({
    queryKey: budgetStatusKey(month),
    queryFn: () => getBudgetStatus(month),
    // The alert is calculated on every read: never show a stale one.
    staleTime: 0,
  })
  const categories = useQuery({ queryKey: CATEGORIES_KEY, queryFn: listCategories })

  const remove = useMutation({
    mutationFn: (line: BudgetStatusLine) => removeBudget(line.category.id),
    onSuccess: (_, line) => {
      void queryClient.invalidateQueries({ queryKey: BUDGETS_KEY })
      toast.success(`Orçamento de ${line.category.name} removido.`)
      setToRemove(null)
    },
    onError: (error) => {
      toast.error(budgetFailure(error, "remover o orçamento"))
      setToRemove(null)
    },
  })

  const lines = status.data?.lines ?? []
  const activeLines = lines.filter((line) => line.category.active).sort(byUrgency)
  const paused = lines.filter((line) => !line.category.active)
  const budgeted = new Set(lines.map((line) => line.category.id))
  const withoutBudget = (categories.data ?? []).filter(
    (category) => category.type === "expense" && category.active && !budgeted.has(category.id),
  )
  const isPast = month < currentMonth
  const canGoForward = month < shiftMonth(currentMonth, MONTHS_AHEAD)

  function openDialog(line: BudgetStatusLine) {
    setDialog({
      categoryId: line.category.id,
      categoryName: line.category.name,
      currentLimitCents: line.limitCents,
    })
  }

  return (
    <section className="flex flex-col gap-6">
      <div className="flex flex-col gap-1">
        <h1 className="font-display text-3xl font-bold tracking-tight">Orçamentos</h1>
        <p className="max-w-prose text-muted-foreground">
          Defina quanto quer gastar por mês em cada categoria de despesa. O limite vale todo mês
          até você alterá-lo, e o gasto inclui as projeções do mês, as compras no cartão e as
          parcelas, no mês do vencimento da fatura.
        </p>
      </div>

      <div className="flex items-center gap-2" role="group" aria-label="Mês de referência">
        <Button
          variant="outline"
          size="icon"
          aria-label="Mês anterior"
          onClick={() => setMonth(shiftMonth(month, -1))}
        >
          <ChevronLeftIcon strokeWidth={1.75} />
        </Button>
        <p
          className="min-w-44 text-center font-display text-lg font-bold first-letter:uppercase"
          aria-live="polite"
        >
          {formatMonthOfYear(month)}
        </p>
        <Button
          variant="outline"
          size="icon"
          aria-label="Próximo mês"
          disabled={!canGoForward}
          onClick={() => setMonth(shiftMonth(month, 1))}
        >
          <ChevronRightIcon strokeWidth={1.75} />
        </Button>
        {month !== currentMonth && (
          <Button variant="ghost" size="sm" onClick={() => setMonth(currentMonth)}>
            Mês atual
          </Button>
        )}
      </div>

      {isPast && (
        <p className="text-sm text-muted-foreground">
          Um mês passado mostra o limite que ele tinha. Alterar um orçamento vale a partir de{" "}
          {formatMonthOfYear(currentMonth)}.
        </p>
      )}

      {status.isError ? (
        <Alert variant="destructive">
          <TriangleAlertIcon strokeWidth={1.75} />
          <AlertDescription>
            <p>Não foi possível carregar os orçamentos. Verifique a conexão e tente de novo.</p>
            <Button size="sm" variant="outline" className="mt-2" onClick={() => void status.refetch()}>
              Tentar de novo
            </Button>
          </AlertDescription>
        </Alert>
      ) : status.isPending ? (
        <div className="flex flex-col gap-3" aria-label="Carregando orçamentos">
          <Skeleton className="h-24 w-full" />
          <Skeleton className="h-24 w-full" />
        </div>
      ) : (
        <>
          {activeLines.length === 0 ? (
            <Empty className="border">
              <EmptyHeader>
                <EmptyMedia variant="icon">
                  <PiggyBankIcon strokeWidth={1.75} />
                </EmptyMedia>
                <EmptyTitle>Nenhum orçamento neste mês</EmptyTitle>
                <EmptyDescription>
                  Escolha uma categoria abaixo e defina quanto quer gastar por mês nela.
                </EmptyDescription>
              </EmptyHeader>
            </Empty>
          ) : (
            <Card className="gap-0 py-0">
              <ul aria-label="Orçamentos do mês" className="divide-y">
                {activeLines.map((line) => (
                  <li key={line.category.id} className="flex flex-col gap-3 px-4 py-4">
                    <BudgetBar
                      category={line.category.name}
                      spent={line.spent}
                      limit={line.limit}
                    />
                    <div className="flex gap-2">
                      <Button
                        variant="outline"
                        size="sm"
                        aria-label={`Alterar limite do orçamento de ${line.category.name}`}
                        onClick={() => openDialog(line)}
                      >
                        Alterar limite
                      </Button>
                      <Button
                        variant="ghost"
                        size="sm"
                        aria-label={`Remover orçamento de ${line.category.name}`}
                        onClick={() => setToRemove(line)}
                      >
                        Remover
                      </Button>
                    </div>
                  </li>
                ))}
              </ul>
            </Card>
          )}

          {withoutBudget.length > 0 && (
            <div className="flex flex-col gap-3">
              <h2 className="font-display text-xl font-bold tracking-tight">Sem orçamento</h2>
              <Card className="gap-0 py-0">
                <ul aria-label="Categorias sem orçamento" className="divide-y">
                  {withoutBudget.map((category) => (
                    <li key={category.id} className="flex items-center gap-3 px-4 py-3">
                      <span className="min-w-0 flex-1 truncate font-medium">{category.name}</span>
                      <Button
                        variant="outline"
                        size="sm"
                        aria-label={`Definir limite do orçamento de ${category.name}`}
                        onClick={() =>
                          setDialog({
                            categoryId: category.id,
                            categoryName: category.name,
                            currentLimitCents: null,
                          })
                        }
                      >
                        Definir limite
                      </Button>
                    </li>
                  ))}
                </ul>
              </Card>
            </div>
          )}

          {paused.length > 0 && (
            <div className="flex flex-col gap-3">
              <h2 className="font-display text-xl font-bold tracking-tight">
                Categorias desativadas
              </h2>
              <p className="text-sm text-muted-foreground">
                O orçamento fica guardado, mas não gera alerta. Ele volta a valer se você reativar
                a categoria.
              </p>
              <Card className="gap-0 py-0">
                <ul aria-label="Orçamentos de categorias desativadas" className="divide-y">
                  {paused.map((line) => (
                    <li key={line.category.id} className="flex flex-col gap-3 px-4 py-4">
                      <div className="flex items-center gap-2">
                        <Badge variant="secondary">Desativada</Badge>
                      </div>
                      <BudgetBar
                        category={line.category.name}
                        spent={line.spent}
                        limit={line.limit}
                        paused
                      />
                      <div className="flex gap-2">
                        <Button
                          variant="ghost"
                          size="sm"
                          aria-label={`Remover orçamento de ${line.category.name}`}
                          onClick={() => setToRemove(line)}
                        >
                          Remover
                        </Button>
                      </div>
                    </li>
                  ))}
                </ul>
              </Card>
            </div>
          )}
        </>
      )}

      {dialog && (
        <BudgetDialog
          // A new key per category resets the form state.
          key={dialog.categoryId}
          target={dialog}
          currentMonth={currentMonth}
          onClose={() => setDialog(null)}
        />
      )}

      <AlertDialog open={toRemove !== null} onOpenChange={(open) => !open && setToRemove(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Remover orçamento?</AlertDialogTitle>
            <AlertDialogDescription>
              {toRemove
                ? `${toRemove.category.name} deixa de ter limite a partir de ${formatMonthOfYear(currentMonth)}. Os meses anteriores continuam com o limite que tinham.`
                : ""}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={remove.isPending}>Cancelar</AlertDialogCancel>
            <Button
              variant="destructive"
              disabled={remove.isPending}
              onClick={() => toRemove && remove.mutate(toRemove)}
            >
              {remove.isPending && <Spinner />}
              Remover
            </Button>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </section>
  )
}
