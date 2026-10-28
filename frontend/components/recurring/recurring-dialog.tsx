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
import { CATEGORIES_KEY, type CategoryType, listCategories } from "@/lib/categories-api"
import {
  createRecurring,
  RECURRING_KEY,
  type RecurringTransaction,
  updateRecurring,
} from "@/lib/recurring-api"
import { recurringFailure, recurringFieldMessage } from "@/lib/recurring-messages"
import { todayInSaoPaulo } from "@/lib/today"
import { TRANSACTIONS_KEY } from "@/lib/transactions-api"

const MAX_DESCRIPTION_LENGTH = 200

export type RecurringDialogMode =
  | { kind: "create"; type: CategoryType }
  | { kind: "edit"; recurring: RecurringTransaction }

interface RecurringDialogProps {
  mode: RecurringDialogMode
  onClose: () => void
}

/**
 * Create a Gasto fixo or Receita fixa, or edit one. When editing, the type and
 * the day of the month are fixed, and a new amount applies from this month on.
 * Mount it only while it is open.
 */
export function RecurringDialog({ mode, onClose }: RecurringDialogProps) {
  const queryClient = useQueryClient()
  const editing = mode.kind === "edit" ? mode.recurring : null
  const thisMonth = todayInSaoPaulo().slice(0, 7)
  const [type, setType] = useState<CategoryType>(
    editing ? editing.type : mode.kind === "create" ? mode.type : "expense",
  )
  const [amount, setAmount] = useState(editing ? formatAmountInput(editing.amountCents) : "")
  const [day, setDay] = useState(editing ? String(editing.dayOfMonth) : "")
  const [description, setDescription] = useState(editing?.description ?? "")
  const [categoryId, setCategoryId] = useState(editing?.category.id ?? "")
  const [startMonth, setStartMonth] = useState(thisMonth)
  const [submitted, setSubmitted] = useState(false)
  const [problem, setProblem] = useState<string | null>(null)
  const [serverError, setServerError] = useState<unknown>(null)

  const categoriesQuery = useQuery({ queryKey: CATEGORIES_KEY, queryFn: listCategories })
  // Only active categories of the right type are offered. When editing, the
  // current one stays in the list even if it was deactivated since.
  const options = (categoriesQuery.data ?? []).filter(
    (category) =>
      category.type === type && (category.active || category.id === editing?.category.id),
  )

  const save = useMutation({
    mutationFn: () => {
      const amountCents = parseAmountToCents(amount) as number
      if (editing) {
        return updateRecurring(editing.id, {
          amountCents,
          description: description.trim(),
          categoryId,
        })
      }
      return createRecurring({
        type,
        amountCents,
        dayOfMonth: Number(day),
        description: description.trim(),
        categoryId,
        startMonth,
      })
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: RECURRING_KEY })
      void queryClient.invalidateQueries({ queryKey: TRANSACTIONS_KEY })
      void queryClient.invalidateQueries({ queryKey: BALANCE_KEY })
      toast.success(editing ? "Lançamento fixo atualizado." : "Lançamento fixo cadastrado.")
      onClose()
    },
    onError: (error) => {
      setServerError(error)
      setProblem(
        recurringFailure(error, editing ? "salvar o lançamento fixo" : "cadastrar o lançamento fixo"),
      )
    },
  })

  const trimmed = description.trim()
  const dayNumber = Number(day)
  const errors = {
    amount: parseAmountToCents(amount) === null ? "Digite um valor maior que zero." : null,
    day:
      editing || (Number.isInteger(dayNumber) && dayNumber >= 1 && dayNumber <= 31)
        ? null
        : "Escolha um dia do mês de 1 a 31.",
    description:
      trimmed.length === 0
        ? "Digite uma descrição."
        : trimmed.length > MAX_DESCRIPTION_LENGTH
          ? `Use até ${MAX_DESCRIPTION_LENGTH} caracteres.`
          : null,
    category: categoryId ? null : "Escolha uma categoria.",
    startMonth:
      editing || (startMonth && startMonth >= thisMonth)
        ? null
        : "Escolha o mês atual ou um mês futuro.",
  }
  const show = (field: keyof typeof errors) =>
    (submitted ? errors[field] : null) ?? recurringFieldMessage(serverError, fieldName[field])

  function submit(event: React.FormEvent) {
    event.preventDefault()
    setProblem(null)
    setServerError(null)
    setSubmitted(true)
    if (Object.values(errors).some(Boolean)) return
    save.mutate()
  }

  function changeType(next: CategoryType) {
    setType(next)
    // A category of the other type is not valid for the new type.
    setCategoryId("")
  }

  const busy = save.isPending

  return (
    <Dialog open onOpenChange={(open) => !open && !busy && onClose()}>
      <DialogContent showCloseButton={false}>
        <form onSubmit={submit} noValidate className="grid gap-4">
          <DialogHeader>
            <DialogTitle className="font-display text-lg font-bold">
              {editing ? "Editar lançamento fixo" : "Novo lançamento fixo"}
            </DialogTitle>
            <DialogDescription>
              {editing
                ? `Todo dia ${editing.dayOfMonth}. Altere o valor, a descrição ou a categoria.`
                : "Cadastre um gasto ou uma receita que se repete todo mês, como aluguel ou salário."}
            </DialogDescription>
          </DialogHeader>

          {!editing && (
            <Field>
              <FieldLabel id="recurring-type-label">Tipo</FieldLabel>
              <ToggleGroup
                type="single"
                aria-labelledby="recurring-type-label"
                value={type}
                onValueChange={(value) => value && changeType(value as CategoryType)}
                spacing={0}
                className="w-full rounded-lg bg-muted p-1"
              >
                <ToggleGroupItem
                  value="expense"
                  className="flex-1 data-[state=on]:bg-card data-[state=on]:shadow-sm"
                >
                  Gasto fixo
                </ToggleGroupItem>
                <ToggleGroupItem
                  value="income"
                  className="flex-1 data-[state=on]:bg-card data-[state=on]:shadow-sm"
                >
                  Receita fixa
                </ToggleGroupItem>
              </ToggleGroup>
            </Field>
          )}

          <Field data-invalid={!!show("amount")}>
            <FieldLabel htmlFor="recurring-amount">Valor</FieldLabel>
            <MoneyInput
              id="recurring-amount"
              autoFocus
              value={amount}
              onChange={setAmount}
              aria-invalid={!!show("amount")}
            />
            {show("amount") && <FieldError>{show("amount")}</FieldError>}
            {editing && (
              <p className="flex items-start gap-1.5 text-sm text-muted-foreground">
                <InfoIcon strokeWidth={1.75} className="mt-0.5 size-4 shrink-0" aria-hidden />
                O novo valor vale a partir deste mês. Os meses anteriores não mudam.
              </p>
            )}
          </Field>

          {!editing && (
            <div className="grid gap-4 sm:grid-cols-2">
              <Field data-invalid={!!show("day")}>
                <FieldLabel htmlFor="recurring-day">Dia do mês</FieldLabel>
                <Input
                  id="recurring-day"
                  type="number"
                  inputMode="numeric"
                  min={1}
                  max={31}
                  placeholder="Ex.: 5"
                  value={day}
                  aria-invalid={!!show("day")}
                  onChange={(event) => setDay(event.target.value)}
                />
                {show("day") ? (
                  <FieldError>{show("day")}</FieldError>
                ) : (
                  <p className="text-sm text-muted-foreground">
                    Em meses mais curtos, vale o último dia.
                  </p>
                )}
              </Field>
              <Field data-invalid={!!show("startMonth")}>
                <FieldLabel htmlFor="recurring-start">Começa em</FieldLabel>
                <Input
                  id="recurring-start"
                  type="month"
                  min={thisMonth}
                  value={startMonth}
                  aria-invalid={!!show("startMonth")}
                  onChange={(event) => setStartMonth(event.target.value)}
                />
                {show("startMonth") && <FieldError>{show("startMonth")}</FieldError>}
              </Field>
            </div>
          )}

          <Field data-invalid={!!show("description")}>
            <FieldLabel htmlFor="recurring-description">Descrição</FieldLabel>
            <Input
              id="recurring-description"
              autoComplete="off"
              placeholder="Ex.: Aluguel"
              value={description}
              aria-invalid={!!show("description")}
              onChange={(event) => setDescription(event.target.value)}
            />
            {show("description") && <FieldError>{show("description")}</FieldError>}
          </Field>

          <Field data-invalid={!!show("category")}>
            <FieldLabel htmlFor="recurring-category">Categoria</FieldLabel>
            <Select value={categoryId} onValueChange={setCategoryId}>
              <SelectTrigger
                id="recurring-category"
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
              {editing ? "Salvar alterações" : "Cadastrar"}
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
  day: "day_of_month",
  description: "description",
  category: "category_id",
  startMonth: "start_month",
} as const
