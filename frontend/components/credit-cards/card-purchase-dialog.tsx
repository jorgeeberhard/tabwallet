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
import { formatAmountInput, parseAmountToCents } from "@/lib/amount"
import { CATEGORIES_KEY, listCategories } from "@/lib/categories-api"
import {
  type CardPurchase,
  CREDIT_CARDS_KEY,
  createCardPurchase,
  INVOICES_KEY,
  listCreditCards,
  updateCardPurchase,
} from "@/lib/credit-cards-api"
import { creditCardFailure, creditCardFieldMessage } from "@/lib/credit-card-messages"
import { formatMonthOfYear } from "@/lib/format"
import { todayInSaoPaulo } from "@/lib/today"
import { TRANSACTIONS_KEY } from "@/lib/transactions-api"

export const MAX_DESCRIPTION_LENGTH = 200

export type CardPurchaseDialogMode =
  /** `creditCardId` preselects the card (the invoice screen). */
  | { kind: "create"; creditCardId?: string }
  | { kind: "edit"; purchase: CardPurchase }

interface CardPurchaseDialogProps {
  mode: CardPurchaseDialogMode
  onClose: () => void
}

/** The API field behind each form field, to show server-side validation next to it. */
const fieldName = {
  card: "credit_card_id",
  amount: "amount_cents",
  date: "date",
  description: "description",
  category: "category_id",
} as const

/**
 * Create or edit a purchase with a credit card (UC15). The invoice it enters
 * is decided by the API from the date and the card's closing day; the toast
 * says which one. Mount it only while it is open.
 */
export function CardPurchaseDialog({ mode, onClose }: CardPurchaseDialogProps) {
  const queryClient = useQueryClient()
  const editing = mode.kind === "edit" ? mode.purchase : null
  const [cardId, setCardId] = useState(
    editing?.creditCard.id ?? (mode.kind === "create" ? (mode.creditCardId ?? "") : ""),
  )
  const [amount, setAmount] = useState(editing ? formatAmountInput(editing.amountCents) : "")
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

  const save = useMutation({
    mutationFn: () => {
      const amountCents = parseAmountToCents(amount) as number
      const fields = { amountCents, date, description: description.trim(), categoryId }
      return editing
        ? updateCardPurchase(editing.id, fields)
        : createCardPurchase({ creditCardId: cardId, ...fields })
    },
    onSuccess: (purchase) => {
      void queryClient.invalidateQueries({ queryKey: TRANSACTIONS_KEY })
      void queryClient.invalidateQueries({ queryKey: INVOICES_KEY })
      void queryClient.invalidateQueries({ queryKey: CREDIT_CARDS_KEY })
      toast.success(
        `${editing ? "Compra atualizada" : "Compra lançada"} na fatura de ${formatMonthOfYear(
          purchase.referenceMonth,
        )} do ${purchase.creditCard.name}.`,
      )
      onClose()
    },
    onError: (error) => {
      setServerError(error)
      setProblem(creditCardFailure(error, editing ? "salvar a compra" : "lançar a compra"))
    },
  })

  const trimmed = description.trim()
  const errors = {
    card: cardId ? null : "Escolha um cartão.",
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
    (submitted ? errors[field] : null) ?? creditCardFieldMessage(serverError, fieldName[field])

  function submit(event: React.FormEvent) {
    event.preventDefault()
    setProblem(null)
    setServerError(null)
    setSubmitted(true)
    if (Object.values(errors).some(Boolean)) return
    save.mutate()
  }

  const busy = save.isPending
  const noCards = !cardsQuery.isPending && cardOptions.length === 0 && !editing

  return (
    <Dialog open onOpenChange={(open) => !open && !busy && onClose()}>
      <DialogContent showCloseButton={false}>
        <form onSubmit={submit} noValidate className="grid gap-4">
          <DialogHeader>
            <DialogTitle className="font-display text-lg font-bold">
              {editing ? "Editar compra no cartão" : "Nova compra no cartão"}
            </DialogTitle>
            <DialogDescription>
              A compra entra na fatura pelo dia de fechamento do cartão e não muda o saldo atual:
              ele só cai quando a fatura é paga.
            </DialogDescription>
          </DialogHeader>

          <Field data-invalid={!!show("card")}>
            <FieldLabel htmlFor="purchase-card">Cartão</FieldLabel>
            {editing ? (
              <Input id="purchase-card" value={editing.creditCard.name} readOnly disabled />
            ) : (
              <Select value={cardId} onValueChange={setCardId}>
                <SelectTrigger
                  id="purchase-card"
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

          <Field data-invalid={!!show("amount")}>
            <FieldLabel htmlFor="purchase-amount">Valor</FieldLabel>
            <MoneyInput
              id="purchase-amount"
              value={amount}
              onChange={setAmount}
              aria-invalid={!!show("amount")}
            />
            {show("amount") && <FieldError>{show("amount")}</FieldError>}
          </Field>

          <Field data-invalid={!!show("date")}>
            <FieldLabel htmlFor="purchase-date">Data da compra</FieldLabel>
            <Input
              id="purchase-date"
              type="date"
              value={date}
              aria-invalid={!!show("date")}
              onChange={(event) => setDate(event.target.value)}
            />
            {show("date") && <FieldError>{show("date")}</FieldError>}
          </Field>

          <Field data-invalid={!!show("description")}>
            <FieldLabel htmlFor="purchase-description">Descrição</FieldLabel>
            <Input
              id="purchase-description"
              autoComplete="off"
              placeholder="Ex.: Restaurante"
              value={description}
              aria-invalid={!!show("description")}
              onChange={(event) => setDescription(event.target.value)}
            />
            {show("description") && <FieldError>{show("description")}</FieldError>}
          </Field>

          <Field data-invalid={!!show("category")}>
            <FieldLabel htmlFor="purchase-category">Categoria</FieldLabel>
            <Select value={categoryId} onValueChange={setCategoryId}>
              <SelectTrigger
                id="purchase-category"
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
              {editing ? "Salvar alterações" : "Lançar compra"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
