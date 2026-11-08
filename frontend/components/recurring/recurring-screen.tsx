"use client"

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { PlusIcon, RepeatIcon, TriangleAlertIcon } from "lucide-react"
import { useState } from "react"
import { toast } from "sonner"

import {
  RecurringDialog,
  type RecurringDialogMode,
} from "@/components/recurring/recurring-dialog"
import { RecurringRow } from "@/components/recurring/recurring-row"
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
import { Button } from "@/components/ui/button"
import { Card } from "@/components/ui/card"
import { Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle } from "@/components/ui/empty"
import { Skeleton } from "@/components/ui/skeleton"
import { Spinner } from "@/components/ui/spinner"
import { BALANCE_KEY } from "@/lib/balance-api"
import {
  endRecurring,
  listRecurring,
  RECURRING_KEY,
  type RecurringTransaction,
} from "@/lib/recurring-api"
import { recurringFailure } from "@/lib/recurring-messages"
import { TRANSACTIONS_KEY } from "@/lib/transactions-api"

/** UC05: the user's Gastos fixos and Receitas fixas, to create, edit and end. */
export function RecurringScreen() {
  const queryClient = useQueryClient()
  const [dialog, setDialog] = useState<RecurringDialogMode | null>(null)
  const [toEnd, setToEnd] = useState<RecurringTransaction | null>(null)

  const recurring = useQuery({ queryKey: RECURRING_KEY, queryFn: listRecurring })

  const end = useMutation({
    mutationFn: (item: RecurringTransaction) => endRecurring(item.id),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: RECURRING_KEY })
      void queryClient.invalidateQueries({ queryKey: TRANSACTIONS_KEY })
      void queryClient.invalidateQueries({ queryKey: BALANCE_KEY })
      toast.success("Lançamento fixo encerrado.")
      setToEnd(null)
    },
    onError: (error) => {
      toast.error(recurringFailure(error, "encerrar o lançamento fixo"))
      setToEnd(null)
    },
  })

  const all = recurring.data ?? []
  const active = all.filter((item) => !item.ended)
  const ended = all.filter((item) => item.ended)

  return (
    <section className="flex flex-col gap-6">
      <div className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
        <div className="flex flex-col gap-1">
          <h1 className="font-display text-3xl font-bold tracking-tight">
            Gastos e receitas fixos
          </h1>
          <p className="max-w-prose text-muted-foreground">
            Aluguel, salário e outros valores que se repetem todo mês. As próximas ocorrências
            aparecem como projeção e entram no saldo quando o dia chega.
          </p>
        </div>
        <Button onClick={() => setDialog({ kind: "create", type: "expense" })}>
          <PlusIcon strokeWidth={1.75} />
          Novo lançamento fixo
        </Button>
      </div>

      {recurring.isError ? (
        <Alert variant="destructive">
          <TriangleAlertIcon strokeWidth={1.75} />
          <AlertDescription>
            <p>
              Não foi possível carregar os lançamentos fixos. Verifique a conexão e tente de novo.
            </p>
            <Button
              size="sm"
              variant="outline"
              className="mt-2"
              onClick={() => void recurring.refetch()}
            >
              Tentar de novo
            </Button>
          </AlertDescription>
        </Alert>
      ) : recurring.isPending ? (
        <div className="flex flex-col gap-2" aria-label="Carregando lançamentos fixos">
          <Skeleton className="h-16 w-full" />
          <Skeleton className="h-16 w-full" />
        </div>
      ) : all.length === 0 ? (
        <Empty className="border">
          <EmptyHeader>
            <EmptyMedia variant="icon">
              <RepeatIcon strokeWidth={1.75} />
            </EmptyMedia>
            <EmptyTitle>Nenhum lançamento fixo ainda</EmptyTitle>
            <EmptyDescription>
              Cadastre o aluguel, o salário ou qualquer valor mensal para ver o que vem pela frente
              sem lançar tudo de novo.
            </EmptyDescription>
          </EmptyHeader>
          <Button onClick={() => setDialog({ kind: "create", type: "expense" })}>
            Novo lançamento fixo
          </Button>
        </Empty>
      ) : (
        <div className="flex flex-col gap-6">
          {active.length > 0 && (
            <div className="flex flex-col gap-2">
              <h2 className="font-display text-lg font-bold tracking-tight">Em andamento</h2>
              <Card className="gap-0 py-0">
                <ul aria-label="Lançamentos fixos em andamento" className="divide-y">
                  {active.map((item) => (
                    <RecurringRow
                      key={item.id}
                      recurring={item}
                      onEdit={(recurring) => setDialog({ kind: "edit", recurring })}
                      onEnd={setToEnd}
                    />
                  ))}
                </ul>
              </Card>
            </div>
          )}
          {ended.length > 0 && (
            <div className="flex flex-col gap-2">
              <h2 className="font-display text-lg font-bold tracking-tight">Encerrados</h2>
              <Card className="gap-0 py-0">
                <ul aria-label="Lançamentos fixos encerrados" className="divide-y">
                  {ended.map((item) => (
                    <RecurringRow
                      key={item.id}
                      recurring={item}
                      onEdit={(recurring) => setDialog({ kind: "edit", recurring })}
                      onEnd={setToEnd}
                    />
                  ))}
                </ul>
              </Card>
            </div>
          )}
        </div>
      )}

      {dialog && (
        <RecurringDialog
          // A new key per target resets the form state.
          key={dialog.kind === "edit" ? dialog.recurring.id : "create"}
          mode={dialog}
          onClose={() => setDialog(null)}
        />
      )}

      <AlertDialog open={toEnd !== null} onOpenChange={(open) => !open && setToEnd(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Encerrar lançamento fixo?</AlertDialogTitle>
            <AlertDialogDescription>
              {toEnd
                ? `"${toEnd.description}" deixa de aparecer nos próximos meses. As ocorrências que já aconteceram continuam no seu histórico. Essa ação não pode ser desfeita.`
                : ""}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={end.isPending}>Cancelar</AlertDialogCancel>
            <Button
              variant="destructive"
              disabled={end.isPending}
              onClick={() => toEnd && end.mutate(toEnd)}
            >
              {end.isPending && <Spinner />}
              Encerrar
            </Button>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </section>
  )
}
