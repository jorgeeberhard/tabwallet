"use client"

import { useMutation, useQueryClient } from "@tanstack/react-query"
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
import { Spinner } from "@/components/ui/spinner"
import { formatAmountInput, parseAmountToCents } from "@/lib/amount"
import { BALANCE_KEY } from "@/lib/balance-api"
import {
  CHARGES_CATEGORY_INACTIVE,
  creditCardFailure,
  creditCardFieldMessage,
} from "@/lib/credit-card-messages"
import {
  type Charge,
  CREDIT_CARDS_KEY,
  createCharge,
  INVOICES_KEY,
  type Invoice,
  updateCharge,
} from "@/lib/credit-cards-api"
import { formatMonthOfYear } from "@/lib/format"
import { todayInSaoPaulo } from "@/lib/today"
import { TRANSACTIONS_KEY } from "@/lib/transactions-api"

export const MAX_CHARGE_DESCRIPTION_LENGTH = 200

export type ChargeDialogMode =
  | { kind: "create"; cardId: string; cardName: string; invoice: Invoice }
  | { kind: "edit"; charge: Charge }

interface ChargeDialogProps {
  mode: ChargeDialogMode
  onClose: () => void
}

/** The API field behind each form field, to show server-side validation next to it. */
const fieldName = {
  amount: "amount_cents",
  date: "date",
  description: "description",
} as const

/**
 * UC21: records the interest, penalty and IOF the bank charged on an invoice,
 * or edits one already recorded. The system never calculates interest: it only
 * keeps the amount the person informs. A charge goes to the "Juros e encargos"
 * Category, joins the invoice total (so it is paid with the invoice) and counts
 * in the month of the invoice's due date. Mount it only while it is open.
 */
export function ChargeDialog({ mode, onClose }: ChargeDialogProps) {
  const queryClient = useQueryClient()
  const editing = mode.kind === "edit" ? mode.charge : null
  const [amount, setAmount] = useState(editing ? formatAmountInput(editing.amountCents) : "")
  const [date, setDate] = useState(editing ? editing.date : todayInSaoPaulo())
  const [description, setDescription] = useState(editing?.description ?? "")
  const [submitted, setSubmitted] = useState(false)
  const [problem, setProblem] = useState<string | null>(null)
  const [serverError, setServerError] = useState<unknown>(null)

  const referenceMonth = mode.kind === "edit" ? mode.charge.referenceMonth : mode.invoice.referenceMonth

  const save = useMutation({
    mutationFn: () => {
      const amountCents = parseAmountToCents(amount) as number
      const fields = {
        amountCents,
        date,
        description: description.trim() || undefined,
      }
      return mode.kind === "edit"
        ? updateCharge(mode.charge.id, fields)
        : createCharge(mode.cardId, mode.invoice.closingMonth, fields)
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: TRANSACTIONS_KEY })
      void queryClient.invalidateQueries({ queryKey: INVOICES_KEY })
      void queryClient.invalidateQueries({ queryKey: CREDIT_CARDS_KEY })
      // The invoice total changed, and with it what the Projected Balance subtracts.
      void queryClient.invalidateQueries({ queryKey: BALANCE_KEY })
      toast.success(
        editing
          ? "Encargo atualizado."
          : `Encargo lançado na fatura de ${formatMonthOfYear(referenceMonth)}.`,
      )
      onClose()
    },
    onError: (error) => {
      setServerError(error)
      setProblem(
        creditCardFailure(error, editing ? "salvar o encargo" : "lançar o encargo"),
      )
    },
  })

  const trimmed = description.trim()
  const errors = {
    amount: parseAmountToCents(amount) === null ? "Digite um valor maior que zero." : null,
    date: date ? null : "Escolha a data.",
    description:
      trimmed.length > MAX_CHARGE_DESCRIPTION_LENGTH
        ? `Use até ${MAX_CHARGE_DESCRIPTION_LENGTH} caracteres.`
        : null,
  }
  const show = (field: keyof typeof errors) =>
    (submitted ? errors[field] : null) ?? creditCardFieldMessage(serverError, fieldName[field])
  const categoryInactive = creditCardFieldMessage(serverError, "category_id") !== null

  function submit(event: React.FormEvent) {
    event.preventDefault()
    setProblem(null)
    setServerError(null)
    setSubmitted(true)
    if (Object.values(errors).some(Boolean)) return
    save.mutate()
  }

  const busy = save.isPending
  const cardName = mode.kind === "create" ? mode.cardName : mode.charge.creditCard.name

  return (
    <Dialog open onOpenChange={(open) => !open && !busy && onClose()}>
      <DialogContent showCloseButton={false}>
        <form onSubmit={submit} noValidate className="grid gap-4">
          <DialogHeader>
            <DialogTitle className="font-display text-lg font-bold">
              {editing ? "Editar encargo" : "Lançar encargos"}
            </DialogTitle>
            <DialogDescription>
              Juros, multa e IOF que o banco cobrou na fatura de {cardName} de{" "}
              {formatMonthOfYear(referenceMonth)}. O sistema não calcula juros: informe o valor
              que veio na fatura do banco.
            </DialogDescription>
          </DialogHeader>

          <Field data-invalid={!!show("amount")}>
            <FieldLabel htmlFor="charge-amount">Valor cobrado</FieldLabel>
            <MoneyInput
              id="charge-amount"
              autoFocus
              value={amount}
              onChange={setAmount}
              aria-invalid={!!show("amount")}
            />
            {show("amount") && <FieldError>{show("amount")}</FieldError>}
          </Field>

          <Field data-invalid={!!show("date")}>
            <FieldLabel htmlFor="charge-date">Data da cobrança</FieldLabel>
            <Input
              id="charge-date"
              type="date"
              value={date}
              aria-invalid={!!show("date")}
              onChange={(event) => setDate(event.target.value)}
            />
            {show("date") && <FieldError>{show("date")}</FieldError>}
          </Field>

          <Field data-invalid={!!show("description")}>
            <FieldLabel htmlFor="charge-description">Descrição (opcional)</FieldLabel>
            <Input
              id="charge-description"
              autoComplete="off"
              placeholder="Juros e encargos"
              value={description}
              aria-invalid={!!show("description")}
              onChange={(event) => setDescription(event.target.value)}
            />
            <FieldDescription>
              Fica na categoria &quot;Juros e encargos&quot; e entra no total da fatura, então é
              paga junto com ela.
            </FieldDescription>
            {show("description") && <FieldError>{show("description")}</FieldError>}
          </Field>

          <p className="flex items-start gap-1.5 text-sm text-muted-foreground">
            <InfoIcon strokeWidth={1.75} className="mt-0.5 size-4 shrink-0" aria-hidden />
            Os encargos contam como despesa em {formatMonthOfYear(referenceMonth)}, o mês do
            vencimento da fatura.
          </p>

          {problem && (
            <Alert variant="destructive">
              <TriangleAlertIcon strokeWidth={1.75} />
              <AlertDescription>
                {categoryInactive ? CHARGES_CATEGORY_INACTIVE : problem}
              </AlertDescription>
            </Alert>
          )}

          <DialogFooter className="-mx-6 -mb-6 mt-2 rounded-b-xl p-6 pt-4">
            <Button type="button" variant="outline" disabled={busy} onClick={onClose}>
              Cancelar
            </Button>
            <Button type="submit" disabled={busy}>
              {busy && <Spinner />}
              {editing ? "Salvar alterações" : "Lançar encargos"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
