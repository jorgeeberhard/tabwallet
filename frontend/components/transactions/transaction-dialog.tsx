"use client"

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { InfoIcon, TriangleAlertIcon } from "lucide-react"
import { useState } from "react"
import { toast } from "sonner"

import { MoneyInput } from "@/components/money-input"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Field, FieldError, FieldLabel } from "@/components/ui/field"
import { Input } from "@/components/ui/input"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { Spinner } from "@/components/ui/spinner"
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group"
import { formatAmountInput, parseAmountToCents } from "@/lib/amount"
import { BALANCE_KEY } from "@/lib/balance-api"
import { CATEGORIES_KEY, listCategories } from "@/lib/categories-api"
import { todayInSaoPaulo } from "@/lib/today"
import {
  createTransaction,
  type Transaction,
  TRANSACTIONS_KEY,
  type TransactionType,
  updateTransaction,
} from "@/lib/transactions-api"
import { transactionFailure, transactionFieldMessage } from "@/lib/transaction-messages"

export const MAX_DESCRIPTION_LENGTH = 200

export type TransactionDialogMode =
  | { kind: "create"; type: TransactionType }
  | { kind: "edit"; transaction: Transaction }

interface TransactionDialogProps {
  mode: TransactionDialogMode
  onClose: () => void
}

/** Create or edit a Transaction. Mount it only while it is open. */
export function TransactionDialog({ mode, onClose }: TransactionDialogProps) {
  const queryClient = useQueryClient()
  const editing = mode.kind === "edit" ? mode.transaction : null
  const [type, setType] = useState<TransactionType>(editing ? editing.type : mode.kind === "create" ? mode.type : "expense")
  const [amount, setAmount] = useState(editing ? formatAmountInput(editing.amountCents) : "")
  const [date, setDate] = useState(editing ? editing.date : todayInSaoPaulo())
  const [description, setDescription] = useState(editing?.description ?? "")
  const [categoryId, setCategoryId] = useState(editing?.category.id ?? "")
  const [submitted, setSubmitted] = useState(false)
  const [problem, setProblem] = useState<string | null>(null)
  const [serverError, setServerError] = useState<unknown>(null)

  const categoriesQuery = useQuery({ queryKey: CATEGORIES_KEY, queryFn: listCategories })
  // Only active categories of the Transaction's type are offered. When
  // editing, the current one stays in the list even if it was deactivated since.
  const options = (categoriesQuery.data ?? []).filter(
    (category) =>
      category.type === type && (category.active || category.id === editing?.category.id),
  )

  const save = useMutation({
    mutationFn: () => {
      const amountCents = parseAmountToCents(amount) as number
      const fields = { amountCents, date, description: description.trim(), categoryId }
      return editing
        ? updateTransaction(editing.id, fields)
        : createTransaction({ type, ...fields })
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: TRANSACTIONS_KEY })
      void queryClient.invalidateQueries({ queryKey: BALANCE_KEY })
      toast.success(editing ? "Transação atualizada." : "Transação lançada.")
      onClose()
    },
    onError: (error) => {
      setServerError(error)
      setProblem(transactionFailure(error, editing ? "salvar a transação" : "lançar a transação"))
    },
  })

  const trimmed = description.trim()
  const errors = {
    amount: parseAmountToCents(amount) === null ? "Digite um valor maior que zero." : null,
    date: date ? null : "Escolha a data.",
    description:
      trimmed.length === 0
        ? "Digite uma descrição."
        : trimmed.length > MAX_DESCRIPTION_LENGTH
          ? `Use até ${MAX_DESCRIPTION_LENGTH} caracteres.`
          : null,
    category: categoryId ? null : "Escolha uma categoria.",
  }
  const show = (field: keyof typeof errors) =>
    (submitted ? errors[field] : null) ?? transactionFieldMessage(serverError, fieldName[field])

  function submit(event: React.FormEvent) {
    event.preventDefault()
    setProblem(null)
    setServerError(null)
    setSubmitted(true)
    if (Object.values(errors).some(Boolean)) return
    save.mutate()
  }

  function changeType(next: TransactionType) {
    setType(next)
    // A category of the other type is not valid for the new type.
    setCategoryId("")
  }

  const isFuture = date > todayInSaoPaulo()
  const busy = save.isPending

  return (
    <Dialog open onOpenChange={(open) => !open && !busy && onClose()}>
      <DialogContent showCloseButton={false}>
        <form onSubmit={submit} noValidate className="grid gap-4">
          <DialogHeader>
            <DialogTitle className="font-display text-lg font-bold">
              {editing ? "Editar transação" : "Nova transação"}
            </DialogTitle>
            <DialogDescription>
              {editing
                ? `Altere o valor, a data, a descrição ou a categoria desta ${type === "income" ? "receita" : "despesa"}.`
                : "Registre uma receita ou uma despesa que não passou pelo cartão de crédito."}
            </DialogDescription>
          </DialogHeader>

          {!editing && (
            <Field>
              <FieldLabel id="transaction-type-label">Tipo</FieldLabel>
              <ToggleGroup
                type="single"
                aria-labelledby="transaction-type-label"
                value={type}
                onValueChange={(value) => value && changeType(value as TransactionType)}
                spacing={0}
                className="w-full rounded-lg bg-muted p-1"
              >
                <ToggleGroupItem
                  value="expense"
                  className="flex-1 data-[state=on]:bg-card data-[state=on]:shadow-sm"
                >
                  Despesa
                </ToggleGroupItem>
                <ToggleGroupItem
                  value="income"
                  className="flex-1 data-[state=on]:bg-card data-[state=on]:shadow-sm"
                >
                  Receita
                </ToggleGroupItem>
              </ToggleGroup>
            </Field>
          )}

          <Field data-invalid={!!show("amount")}>
            <FieldLabel htmlFor="transaction-amount">Valor</FieldLabel>
            <MoneyInput
              id="transaction-amount"
              autoFocus
              value={amount}
              onChange={setAmount}
              aria-invalid={!!show("amount")}
            />
            {show("amount") && <FieldError>{show("amount")}</FieldError>}
          </Field>

          <Field data-invalid={!!show("date")}>
            <FieldLabel htmlFor="transaction-date">Data</FieldLabel>
            <Input
              id="transaction-date"
              type="date"
              value={date}
              aria-invalid={!!show("date")}
              onChange={(event) => setDate(event.target.value)}
            />
            {show("date") && <FieldError>{show("date")}</FieldError>}
            {isFuture && (
              <p className="flex items-start gap-1.5 text-sm text-muted-foreground">
                <InfoIcon strokeWidth={1.75} className="mt-0.5 size-4 shrink-0" aria-hidden />
                Data futura: vira uma projeção e só entra no saldo atual quando o dia chegar.
              </p>
            )}
          </Field>

          <Field data-invalid={!!show("description")}>
            <FieldLabel htmlFor="transaction-description">Descrição</FieldLabel>
            <Input
              id="transaction-description"
              autoComplete="off"
              placeholder="Ex.: Supermercado"
              value={description}
              aria-invalid={!!show("description")}
              onChange={(event) => setDescription(event.target.value)}
            />
            {show("description") && <FieldError>{show("description")}</FieldError>}
          </Field>

          <Field data-invalid={!!show("category")}>
            <FieldLabel htmlFor="transaction-category">Categoria</FieldLabel>
            <Select value={categoryId} onValueChange={setCategoryId}>
              <SelectTrigger
                id="transaction-category"
                className="w-full"
                aria-invalid={!!show("category")}
              >
                <SelectValue
                  placeholder={categoriesQuery.isPending ? "Carregando..." : "Escolha uma categoria"}
                />
              </SelectTrigger>
              <SelectContent>
                {options.map((category) => (
                  <SelectItem key={category.id} value={category.id}>
                    {category.name}
                    {!category.active && " (desativada)"}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            {show("category") && <FieldError>{show("category")}</FieldError>}
            {categoriesQuery.isError && (
              <FieldError>Não foi possível carregar as categorias. Feche e tente de novo.</FieldError>
            )}
          </Field>

          {problem && (
            <Alert variant="destructive">
              <TriangleAlertIcon strokeWidth={1.75} />
              <AlertDescription>{problem}</AlertDescription>
            </Alert>
          )}

          <DialogFooter className="-mx-6 -mb-6 mt-2 rounded-b-xl p-6 pt-4">
            <Button type="button" variant="outline" disabled={busy} onClick={onClose}>
              Cancelar
            </Button>
            <Button type="submit" disabled={busy}>
              {busy && <Spinner />}
              {editing ? "Salvar alterações" : "Lançar"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}

/** The API field behind each form field, to show server-side validation next to it. */
const fieldName = {
  amount: "amount_cents",
  date: "date",
  description: "description",
  category: "category_id",
} as const
