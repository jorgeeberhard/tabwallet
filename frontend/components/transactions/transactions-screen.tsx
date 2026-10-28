"use client"

import {
  useInfiniteQuery,
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query"
import { InboxIcon, PlusIcon, TriangleAlertIcon } from "lucide-react"
import { useState } from "react"
import { toast } from "sonner"

import {
  TransactionDialog,
  type TransactionDialogMode,
} from "@/components/transactions/transaction-dialog"
import { OccurrenceAmountDialog } from "@/components/transactions/occurrence-amount-dialog"
import { OccurrenceRow } from "@/components/transactions/occurrence-row"
import { TransactionRow } from "@/components/transactions/transaction-row"
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
import { Field, FieldLabel } from "@/components/ui/field"
import { Input } from "@/components/ui/input"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { Skeleton } from "@/components/ui/skeleton"
import { Spinner } from "@/components/ui/spinner"
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group"
import { BALANCE_KEY } from "@/lib/balance-api"
import { CATEGORIES_KEY, listCategories } from "@/lib/categories-api"
import { formatDate, formatMonth } from "@/lib/format"
import { type Occurrence, skipOccurrence } from "@/lib/recurring-api"
import { recurringFailure } from "@/lib/recurring-messages"
import { transactionFailure } from "@/lib/transaction-messages"
import {
  deleteTransaction,
  listTransactions,
  PAGE_SIZE,
  type StatementItem,
  statementItemKey,
  statementKey,
  type Transaction,
  TRANSACTIONS_KEY,
  type TransactionFilters,
  type TransactionType,
} from "@/lib/transactions-api"

type TypeFilter = "all" | TransactionType

const TYPE_FILTERS: { value: TypeFilter; label: string }[] = [
  { value: "all", label: "Todas" },
  { value: "income", label: "Receitas" },
  { value: "expense", label: "Despesas" },
]

const ALL_CATEGORIES = "all"

/** Groups consecutive items by the month of their date (the list is newest first). */
function groupByMonth(items: StatementItem[]) {
  const groups: { month: string; items: StatementItem[] }[] = []
  for (const item of items) {
    const month = formatMonth(item.date)
    const last = groups[groups.length - 1]
    if (last?.month === month) last.items.push(item)
    else groups.push({ month, items: [item] })
  }
  return groups
}

/** UC04, UC06, UC07 and UC08: the statement of income and expenses outside the card. */
export function TransactionsScreen() {
  const queryClient = useQueryClient()
  const [type, setType] = useState<TypeFilter>("all")
  const [categoryId, setCategoryId] = useState(ALL_CATEGORIES)
  const [from, setFrom] = useState("")
  const [to, setTo] = useState("")
  const [dialog, setDialog] = useState<TransactionDialogMode | null>(null)
  const [toDelete, setToDelete] = useState<Transaction | null>(null)
  const [occurrenceToEdit, setOccurrenceToEdit] = useState<Occurrence | null>(null)
  const [occurrenceToSkip, setOccurrenceToSkip] = useState<Occurrence | null>(null)

  const periodIsBackwards = !!from && !!to && from > to
  const filters: TransactionFilters = {
    type: type === "all" ? undefined : type,
    categoryId: categoryId === ALL_CATEGORIES ? undefined : categoryId,
    from: from || undefined,
    // A backwards period would be refused by the API: wait for the person to fix it.
    to: to || undefined,
  }
  const hasFilters = type !== "all" || categoryId !== ALL_CATEGORIES || !!from || !!to

  const categories = useQuery({ queryKey: CATEGORIES_KEY, queryFn: listCategories })
  const statement = useInfiniteQuery({
    queryKey: statementKey(filters),
    queryFn: ({ pageParam }) => listTransactions({ ...filters, offset: pageParam }),
    initialPageParam: 0,
    getNextPageParam: (last) =>
      last.offset + last.items.length < last.total ? last.offset + PAGE_SIZE : undefined,
    enabled: !periodIsBackwards,
  })

  const remove = useMutation({
    mutationFn: (transaction: Transaction) => deleteTransaction(transaction.id),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: TRANSACTIONS_KEY })
      void queryClient.invalidateQueries({ queryKey: BALANCE_KEY })
      toast.success("Transação excluída.")
      setToDelete(null)
    },
    onError: (error) => {
      toast.error(transactionFailure(error, "excluir a transação"))
      setToDelete(null)
    },
  })

  const skip = useMutation({
    mutationFn: (occurrence: Occurrence) =>
      skipOccurrence(occurrence.recurringId, occurrence.month),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: TRANSACTIONS_KEY })
      void queryClient.invalidateQueries({ queryKey: BALANCE_KEY })
      toast.success("Ocorrência excluída só deste mês.")
      setOccurrenceToSkip(null)
    },
    onError: (error) => {
      toast.error(recurringFailure(error, "excluir a ocorrência"))
      setOccurrenceToSkip(null)
    },
  })

  function clearFilters() {
    setType("all")
    setCategoryId(ALL_CATEGORIES)
    setFrom("")
    setTo("")
  }

  function changeType(next: TypeFilter) {
    setType(next)
    // The chosen category may be of the other type.
    const chosen = categories.data?.find((category) => category.id === categoryId)
    if (chosen && next !== "all" && chosen.type !== next) setCategoryId(ALL_CATEGORIES)
  }

  const categoryOptions = (categories.data ?? []).filter(
    (category) => type === "all" || category.type === type,
  )
  const items = statement.data?.pages.flatMap((page) => page.items) ?? []
  const total = statement.data?.pages[0]?.total ?? 0
  const newDialogType: TransactionType = type === "income" ? "income" : "expense"

  return (
    <section className="flex flex-col gap-6">
      <div className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
        <div className="flex flex-col gap-1">
          <h1 className="font-display text-3xl font-bold tracking-tight">Transações</h1>
          <p className="max-w-prose text-muted-foreground">
            Receitas e despesas que não passaram pelo cartão de crédito, inclusive as ocorrências
            dos seus gastos e receitas fixos. Lançamentos com data futura aparecem como projeção.
          </p>
        </div>
        <Button onClick={() => setDialog({ kind: "create", type: newDialogType })}>
          <PlusIcon strokeWidth={1.75} />
          Nova transação
        </Button>
      </div>

      <form
        aria-label="Filtros"
        onSubmit={(event) => event.preventDefault()}
        className="grid gap-4 sm:grid-cols-2 lg:grid-cols-[auto_1fr_10rem_10rem_auto] lg:items-end"
      >
        <ToggleGroup
          type="single"
          aria-label="Tipo de transação"
          value={type}
          onValueChange={(value) => value && changeType(value as TypeFilter)}
          spacing={0}
          className="w-full rounded-lg bg-muted p-1 sm:col-span-2 lg:col-span-1 lg:w-auto"
        >
          {TYPE_FILTERS.map(({ value, label }) => (
            <ToggleGroupItem
              key={value}
              value={value}
              className="h-9 flex-1 data-[state=on]:bg-card data-[state=on]:shadow-sm lg:flex-none lg:px-4"
            >
              {label}
            </ToggleGroupItem>
          ))}
        </ToggleGroup>

        <Field>
          <FieldLabel htmlFor="filter-category">Categoria</FieldLabel>
          <Select value={categoryId} onValueChange={setCategoryId}>
            <SelectTrigger id="filter-category" className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value={ALL_CATEGORIES}>Todas as categorias</SelectItem>
              {categoryOptions.map((category) => (
                <SelectItem key={category.id} value={category.id}>
                  {category.name}
                  {!category.active && " (desativada)"}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </Field>

        <Field data-invalid={periodIsBackwards}>
          <FieldLabel htmlFor="filter-from">De</FieldLabel>
          <Input
            id="filter-from"
            type="date"
            value={from}
            aria-invalid={periodIsBackwards}
            onChange={(event) => setFrom(event.target.value)}
          />
        </Field>
        <Field data-invalid={periodIsBackwards}>
          <FieldLabel htmlFor="filter-to">Até</FieldLabel>
          <Input
            id="filter-to"
            type="date"
            value={to}
            aria-invalid={periodIsBackwards}
            onChange={(event) => setTo(event.target.value)}
          />
        </Field>

        {hasFilters && (
          <Button type="button" variant="ghost" onClick={clearFilters}>
            Limpar filtros
          </Button>
        )}
      </form>

      {periodIsBackwards ? (
        <Alert variant="warning">
          <TriangleAlertIcon strokeWidth={1.75} />
          <AlertDescription>A data inicial não pode ser depois da data final.</AlertDescription>
        </Alert>
      ) : statement.isError ? (
        <Alert variant="destructive">
          <TriangleAlertIcon strokeWidth={1.75} />
          <AlertDescription>
            <p>Não foi possível carregar as transações. Verifique a conexão e tente de novo.</p>
            <Button
              size="sm"
              variant="outline"
              className="mt-2"
              onClick={() => void statement.refetch()}
            >
              Tentar de novo
            </Button>
          </AlertDescription>
        </Alert>
      ) : statement.isPending ? (
        <div className="flex flex-col gap-2" aria-label="Carregando transações">
          <Skeleton className="h-16 w-full" />
          <Skeleton className="h-16 w-full" />
          <Skeleton className="h-16 w-full" />
        </div>
      ) : items.length === 0 ? (
        <Empty className="border">
          <EmptyHeader>
            <EmptyMedia variant="icon">
              <InboxIcon strokeWidth={1.75} />
            </EmptyMedia>
            <EmptyTitle>
              {hasFilters ? "Nenhuma transação com estes filtros" : "Nenhuma transação ainda"}
            </EmptyTitle>
            <EmptyDescription>
              {hasFilters
                ? "Ajuste ou limpe os filtros para ver outras transações."
                : "Lance sua primeira receita ou despesa para começar a acompanhar o seu dinheiro."}
            </EmptyDescription>
          </EmptyHeader>
          {hasFilters ? (
            <Button variant="ghost" onClick={clearFilters}>
              Limpar filtros
            </Button>
          ) : (
            <Button onClick={() => setDialog({ kind: "create", type: "expense" })}>
              Nova transação
            </Button>
          )}
        </Empty>
      ) : (
        <div className="flex flex-col gap-6">
          {groupByMonth(items).map((group) => (
            <div key={group.month} className="flex flex-col gap-2">
              <h2 className="font-display text-lg font-bold tracking-tight first-letter:uppercase">
                {group.month}
              </h2>
              <Card className="gap-0 py-0">
                <ul aria-label={`Transações de ${group.month}`} className="divide-y">
                  {group.items.map((item) =>
                    item.kind === "occurrence" ? (
                      <OccurrenceRow
                        key={statementItemKey(item)}
                        occurrence={item}
                        onEditAmount={setOccurrenceToEdit}
                        onSkip={setOccurrenceToSkip}
                      />
                    ) : (
                      <TransactionRow
                        key={statementItemKey(item)}
                        transaction={item}
                        onEdit={(transaction) => setDialog({ kind: "edit", transaction })}
                        onDelete={setToDelete}
                      />
                    ),
                  )}
                </ul>
              </Card>
            </div>
          ))}
          <div className="flex flex-col items-center gap-2">
            <p className="text-sm text-muted-foreground">
              Mostrando {items.length} de {total}
            </p>
            {statement.hasNextPage && (
              <Button
                variant="outline"
                disabled={statement.isFetchingNextPage}
                onClick={() => void statement.fetchNextPage()}
              >
                {statement.isFetchingNextPage && <Spinner />}
                Carregar mais
              </Button>
            )}
          </div>
        </div>
      )}

      {dialog && (
        <TransactionDialog
          // A new key per target resets the form state.
          key={dialog.kind === "edit" ? dialog.transaction.id : "create"}
          mode={dialog}
          onClose={() => setDialog(null)}
        />
      )}

      {occurrenceToEdit && (
        <OccurrenceAmountDialog
          key={statementItemKey({ kind: "occurrence", ...occurrenceToEdit })}
          occurrence={occurrenceToEdit}
          onClose={() => setOccurrenceToEdit(null)}
        />
      )}

      <AlertDialog
        open={occurrenceToSkip !== null}
        onOpenChange={(open) => !open && setOccurrenceToSkip(null)}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Excluir só este mês?</AlertDialogTitle>
            <AlertDialogDescription>
              {occurrenceToSkip
                ? `"${occurrenceToSkip.description}" de ${formatDate(occurrenceToSkip.date)} será removido da sua lista. Os outros meses continuam. Essa ação não pode ser desfeita.`
                : ""}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={skip.isPending}>Cancelar</AlertDialogCancel>
            <Button
              variant="destructive"
              disabled={skip.isPending}
              onClick={() => occurrenceToSkip && skip.mutate(occurrenceToSkip)}
            >
              {skip.isPending && <Spinner />}
              Excluir só este mês
            </Button>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      <AlertDialog open={toDelete !== null} onOpenChange={(open) => !open && setToDelete(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Excluir transação?</AlertDialogTitle>
            <AlertDialogDescription>
              {toDelete
                ? `"${toDelete.description}" será removida da sua lista. Essa ação não pode ser desfeita.`
                : ""}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={remove.isPending}>Cancelar</AlertDialogCancel>
            <Button
              variant="destructive"
              disabled={remove.isPending}
              onClick={() => toDelete && remove.mutate(toDelete)}
            >
              {remove.isPending && <Spinner />}
              Excluir
            </Button>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </section>
  )
}
