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
import { Field, FieldDescription, FieldError, FieldLabel } from "@/components/ui/field"
import { Input } from "@/components/ui/input"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { Spinner } from "@/components/ui/spinner"
import { centsToReais, formatAmountInput, parseAmountToCents } from "@/lib/amount"
import { CATEGORIES_KEY, listCategories } from "@/lib/categories-api"
import {
  CREDIT_CARDS_KEY,
  createInstallmentPurchase,
  INVOICES_KEY,
  type InstallmentPurchase,
  type InvoiceInstallment,
  listCreditCards,
  updateInstallmentPurchase,
} from "@/lib/credit-cards-api"
import { creditCardFailure, creditCardFieldMessage } from "@/lib/credit-card-messages"
import { formatBRL, formatMonthOfYear } from "@/lib/format"
import { MAX_INSTALLMENTS, MIN_INSTALLMENTS, splitInstallments } from "@/lib/installments"
import { todayInSaoPaulo } from "@/lib/today"
import { TRANSACTIONS_KEY } from "@/lib/transactions-api"

export const MAX_DESCRIPTION_LENGTH = 200

/** What the dialog needs to edit a purchase: both the statement and the invoice have it. */
export interface InstallmentPurchaseTarget {
  id: string
  totalCents: number
  installmentCount: number
  date: string
  description: string
  category: { id: string; name: string; active: boolean }
  creditCard: { id: string; name: string }
  /** Some Installment is in a paid invoice: only the Category can change. */
  hasPaidInstallments: boolean
  /** Some Installment was anticipated: same limits as a paid one. */
  hasAnticipatedInstallments: boolean
}

/** The purchase an invoice line belongs to, as the dialog edits it. */
export function targetOfInvoiceInstallment(item: InvoiceInstallment): InstallmentPurchaseTarget {
  return {
    id: item.installmentPurchaseId,
    totalCents: item.totalCents,
    installmentCount: item.installmentCount,
    date: item.date,
    description: item.description,
    category: item.category,
    creditCard: item.creditCard,
    hasPaidInstallments: item.purchaseHasPaidInstallments,
    hasAnticipatedInstallments: item.purchaseHasAnticipatedInstallments,
  }
}

export type InstallmentPurchaseDialogMode =
  /** `creditCardId` preselects the card (the invoice screen). */
  | { kind: "create"; creditCardId?: string }
  | { kind: "edit"; purchase: InstallmentPurchaseTarget }

interface InstallmentPurchaseDialogProps {
  mode: InstallmentPurchaseDialogMode
  onClose: () => void
}

/** The API field behind each form field, to show server-side validation next to it. */
const fieldName = {
  card: "credit_card_id",
  total: "total_cents",
  count: "installment_count",
  date: "date",
  description: "description",
  category: "category_id",
} as const

/** The text under the total: how it will be split. */
export function splitSummary(totalCents: number | null, count: number): string | null {
  if (totalCents === null) return null
  const parts = splitInstallments(totalCents, count)
  if (!parts) return null
  const [first, ...rest] = parts
  const others = rest[0]
  if (others === undefined || first === others) {
    return `${count} parcelas de ${formatBRL(centsToReais(first))}.`
  }
  return `1ª parcela de ${formatBRL(centsToReais(first))} e ${rest.length} de ${formatBRL(
    centsToReais(others),
  )}.`
}

/**
 * Create or edit a purchase split in Installments (UC18). The person informs
 * the total, already with interest if any, and the number of Installments;
 * the API splits it in exact cents and puts each Installment in a consecutive
 * invoice. While some Installment is in a paid invoice only the Category can
 * change. Mount it only while it is open.
 */
export function InstallmentPurchaseDialog({ mode, onClose }: InstallmentPurchaseDialogProps) {
  const queryClient = useQueryClient()
  const editing = mode.kind === "edit" ? mode.purchase : null
  const locked = (editing?.hasPaidInstallments || editing?.hasAnticipatedInstallments) ?? false
  const [cardId, setCardId] = useState(
    editing?.creditCard.id ?? (mode.kind === "create" ? (mode.creditCardId ?? "") : ""),
  )
  const [total, setTotal] = useState(editing ? formatAmountInput(editing.totalCents) : "")
  const [count, setCount] = useState(editing ? String(editing.installmentCount) : "")
  const [date, setDate] = useState(editing ? editing.date : todayInSaoPaulo())
  const [description, setDescription] = useState(editing?.description ?? "")
  const [categoryId, setCategoryId] = useState(editing?.category.id ?? "")
  const [submitted, setSubmitted] = useState(false)
  const [problem, setProblem] = useState<string | null>(null)
  const [serverError, setServerError] = useState<unknown>(null)

  const cardsQuery = useQuery({ queryKey: CREDIT_CARDS_KEY, queryFn: listCreditCards })
  const categoriesQuery = useQuery({ queryKey: CATEGORIES_KEY, queryFn: listCategories })
  // Only active cards take new purchases.
  const cardOptions = (cardsQuery.data ?? []).filter((card) => card.active)
  // Only active expense categories; when editing, the current one stays even if
  // it was deactivated since.
  const categoryOptions = (categoriesQuery.data ?? []).filter(
    (category) =>
      category.type === "expense" && (category.active || category.id === editing?.category.id),
  )

  const totalCents = parseAmountToCents(total)
  const countNumber = count.trim() === "" ? NaN : Number(count)
  const countIsValid =
    Number.isInteger(countNumber) &&
    countNumber >= MIN_INSTALLMENTS &&
    countNumber <= MAX_INSTALLMENTS

  const save = useMutation({
    mutationFn: (): Promise<InstallmentPurchase> => {
      if (editing) {
        // With paid Installments the API only takes the Category.
        return updateInstallmentPurchase(
          editing.id,
          locked
            ? { categoryId }
            : {
                totalCents: totalCents as number,
                installmentCount: countNumber,
                date,
                description: description.trim(),
                categoryId,
              },
        )
      }
      return createInstallmentPurchase({
        creditCardId: cardId,
        totalCents: totalCents as number,
        installmentCount: countNumber,
        date,
        description: description.trim(),
        categoryId,
      })
    },
    onSuccess: (purchase) => {
      void queryClient.invalidateQueries({ queryKey: TRANSACTIONS_KEY })
      void queryClient.invalidateQueries({ queryKey: INVOICES_KEY })
      void queryClient.invalidateQueries({ queryKey: CREDIT_CARDS_KEY })
      const first = purchase.installments[0]
      toast.success(
        first
          ? `${editing ? "Compra parcelada atualizada" : "Compra parcelada lançada"}: a ${
              first.number === 1 ? "primeira parcela" : `parcela ${first.number}`
            } está na fatura de ${formatMonthOfYear(first.referenceMonth)} do ${
              purchase.creditCard.name
            }.`
          : "Compra parcelada atualizada.",
      )
      onClose()
    },
    onError: (error) => {
      setServerError(error)
      setProblem(
        creditCardFailure(error, editing ? "salvar a compra parcelada" : "lançar a compra parcelada"),
      )
    },
  })

  const trimmed = description.trim()
  const errors = {
    card: cardId ? null : "Escolha um cartão.",
    total: totalCents === null ? "Digite um valor maior que zero." : null,
    count: countIsValid ? null : `Escolha de ${MIN_INSTALLMENTS} a ${MAX_INSTALLMENTS} parcelas.`,
    date: date ? null : "Escolha a data.",
    description:
      trimmed.length === 0
        ? "Digite uma descrição."
        : trimmed.length > MAX_DESCRIPTION_LENGTH
          ? `Use até ${MAX_DESCRIPTION_LENGTH} caracteres.`
          : null,
    category: categoryId ? null : "Escolha uma categoria.",
  }
  if (!errors.total && !errors.count && !splitInstallments(totalCents as number, countNumber)) {
    errors.total = "O valor precisa dar pelo menos R$ 0,01 para cada parcela."
  }
  const show = (field: keyof typeof errors) =>
    (submitted ? errors[field] : null) ?? creditCardFieldMessage(serverError, fieldName[field])

  function submit(event: React.FormEvent) {
    event.preventDefault()
    setProblem(null)
    setServerError(null)
    setSubmitted(true)
    // With paid Installments only the Category is sent, so only it must be valid.
    const relevant = locked ? errors.category : Object.values(errors).some(Boolean)
    if (relevant) return
    save.mutate()
  }

  const busy = save.isPending
  const noCards = !cardsQuery.isPending && cardOptions.length === 0 && !editing
  const summary = splitSummary(totalCents, countIsValid ? countNumber : 0)

  return (
    <Dialog open onOpenChange={(open) => !open && !busy && onClose()}>
      <DialogContent showCloseButton={false}>
        <form onSubmit={submit} noValidate className="grid gap-4">
          <DialogHeader>
            <DialogTitle className="font-display text-lg font-bold">
              {editing ? "Editar compra parcelada" : "Nova compra parcelada"}
            </DialogTitle>
            <DialogDescription>
              Cada parcela entra numa fatura seguinte, a primeira na que receberia a compra à
              vista. Nada disso muda o saldo atual: ele só cai quando a fatura é paga.
            </DialogDescription>
          </DialogHeader>

          {locked && (
            <Alert variant="warning">
              <InfoIcon strokeWidth={1.75} />
              <AlertDescription>
                {editing?.hasAnticipatedInstallments
                  ? "Esta compra tem parcelas antecipadas, e a antecipação não pode ser desfeita. "
                  : "Esta compra já tem parcelas em fatura paga. "}
                Só a categoria pode mudar; para parar de pagar, cancele as parcelas restantes.
              </AlertDescription>
            </Alert>
          )}

          <Field data-invalid={!!show("card")}>
            <FieldLabel htmlFor="installment-card">Cartão</FieldLabel>
            {editing ? (
              <Input id="installment-card" value={editing.creditCard.name} readOnly disabled />
            ) : (
              <Select value={cardId} onValueChange={setCardId}>
                <SelectTrigger
                  id="installment-card"
                  className="w-full"
                  aria-invalid={!!show("card")}
                >
                  <SelectValue
                    placeholder={cardsQuery.isPending ? "Carregando..." : "Escolha um cartão"}
                  />
                </SelectTrigger>
                <SelectContent>
                  {cardOptions.map((card) => (
                    <SelectItem key={card.id} value={card.id}>
                      {card.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            )}
            {show("card") && <FieldError>{show("card")}</FieldError>}
            {noCards && (
              <p className="flex items-start gap-1.5 text-sm text-muted-foreground">
                <InfoIcon strokeWidth={1.75} className="mt-0.5 size-4 shrink-0" aria-hidden />
                Nenhum cartão ativo. Cadastre um em Cartões antes de lançar compras.
              </p>
            )}
          </Field>

          <div className="grid gap-4 sm:grid-cols-[1fr_8rem]">
            <Field data-invalid={!!show("total")}>
              <FieldLabel htmlFor="installment-total">Valor total</FieldLabel>
              <MoneyInput
                id="installment-total"
                value={total}
                onChange={setTotal}
                disabled={locked}
                aria-invalid={!!show("total")}
              />
              {show("total") && <FieldError>{show("total")}</FieldError>}
            </Field>

            <Field data-invalid={!!show("count")}>
              <FieldLabel htmlFor="installment-count">Parcelas</FieldLabel>
              <Input
                id="installment-count"
                type="number"
                inputMode="numeric"
                min={MIN_INSTALLMENTS}
                max={MAX_INSTALLMENTS}
                step={1}
                placeholder="Ex.: 10"
                value={count}
                disabled={locked}
                aria-invalid={!!show("count")}
                onChange={(event) => setCount(event.target.value)}
              />
              {show("count") && <FieldError>{show("count")}</FieldError>}
            </Field>
          </div>
          <p className="-mt-2 text-sm text-muted-foreground" aria-live="polite">
            {summary ??
              "Informe o valor total, já com juros se houver: o sistema não calcula juros. O centavo que sobrar da divisão fica na primeira parcela."}
          </p>

          <Field data-invalid={!!show("date")}>
            <FieldLabel htmlFor="installment-date">Data da compra</FieldLabel>
            <Input
              id="installment-date"
              type="date"
              value={date}
              disabled={locked}
              aria-invalid={!!show("date")}
              onChange={(event) => setDate(event.target.value)}
            />
            {show("date") && <FieldError>{show("date")}</FieldError>}
            {!show("date") && !locked && (
              <FieldDescription>
                Até o dia de fechamento do cartão, a primeira parcela vai para a fatura do mês; depois,
                para a do mês seguinte.
              </FieldDescription>
            )}
          </Field>

          <Field data-invalid={!!show("description")}>
            <FieldLabel htmlFor="installment-description">Descrição</FieldLabel>
            <Input
              id="installment-description"
              autoComplete="off"
              placeholder="Ex.: Geladeira"
              value={description}
              disabled={locked}
              aria-invalid={!!show("description")}
              onChange={(event) => setDescription(event.target.value)}
            />
            {show("description") && <FieldError>{show("description")}</FieldError>}
          </Field>

          <Field data-invalid={!!show("category")}>
            <FieldLabel htmlFor="installment-category">Categoria</FieldLabel>
            <Select value={categoryId} onValueChange={setCategoryId}>
              <SelectTrigger
                id="installment-category"
                className="w-full"
                aria-invalid={!!show("category")}
              >
                <SelectValue
                  placeholder={categoriesQuery.isPending ? "Carregando..." : "Escolha uma categoria"}
                />
              </SelectTrigger>
              <SelectContent>
                {categoryOptions.map((category) => (
                  <SelectItem key={category.id} value={category.id}>
                    {category.name}
                    {!category.active && " (desativada)"}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            {show("category") && <FieldError>{show("category")}</FieldError>}
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
            <Button type="submit" disabled={busy || noCards}>
              {busy && <Spinner />}
              {editing ? "Salvar alterações" : "Lançar compra parcelada"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
