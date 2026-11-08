"use client"

import { useMutation, useQueryClient } from "@tanstack/react-query"
import { TriangleAlertIcon } from "lucide-react"
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
import { Spinner } from "@/components/ui/spinner"
import { formatAmountInput, parseAmountToCents } from "@/lib/amount"
import { BALANCE_KEY } from "@/lib/balance-api"
import { creditCardFailure } from "@/lib/credit-card-messages"
import { CREDIT_CARDS_KEY, INVOICES_KEY, type Invoice, payInvoice } from "@/lib/credit-cards-api"
import { formatBRL, formatMonthOfYear } from "@/lib/format"
import { TRANSACTIONS_KEY } from "@/lib/transactions-api"

interface PayInvoiceDialogProps {
  cardId: string
  cardName: string
  invoice: Invoice
  onClose: () => void
}

/**
 * Pays an Open or Closed invoice before the due date, in full or in
 * part. The amount starts as everything that is still to pay and cannot be
 * more than that. The payment leaves the Current Balance at once; whatever is
 * left is debited on its own on the due date. Mount it only while it is open.
 */
export function PayInvoiceDialog({ cardId, cardName, invoice, onClose }: PayInvoiceDialogProps) {
  const queryClient = useQueryClient()
  const [amount, setAmount] = useState(formatAmountInput(invoice.remainingCents))
  const [submitted, setSubmitted] = useState(false)
  const [problem, setProblem] = useState<string | null>(null)

  const pay = useMutation({
    mutationFn: () =>
      payInvoice(cardId, invoice.closingMonth, parseAmountToCents(amount) as number),
    onSuccess: ({ payment }) => {
      void queryClient.invalidateQueries({ queryKey: INVOICES_KEY })
      void queryClient.invalidateQueries({ queryKey: CREDIT_CARDS_KEY })
      void queryClient.invalidateQueries({ queryKey: TRANSACTIONS_KEY })
      // The Current Balance, the history and the Projected Balance all changed.
      void queryClient.invalidateQueries({ queryKey: BALANCE_KEY })
      toast.success(`Pagamento de ${formatBRL(payment.amount)} registrado.`)
      onClose()
    },
    onError: (error) => setProblem(creditCardFailure(error, "registrar o pagamento")),
  })

  const cents = parseAmountToCents(amount)
  const amountError =
    cents === null
      ? "Digite um valor maior que zero."
      : cents > invoice.remainingCents
        ? `O valor não pode passar do restante da fatura (${formatBRL(invoice.remaining)}).`
        : null
  const busy = pay.isPending

  function submit(event: React.FormEvent) {
    event.preventDefault()
    setProblem(null)
    setSubmitted(true)
    if (amountError) return
    pay.mutate()
  }

  return (
    <Dialog open onOpenChange={(open) => !open && !busy && onClose()}>
      <DialogContent showCloseButton={false}>
        <form onSubmit={submit} noValidate className="grid gap-4">
          <DialogHeader>
            <DialogTitle className="font-display text-lg font-bold">Pagar antecipado</DialogTitle>
            <DialogDescription>
              Fatura de {cardName} de {formatMonthOfYear(invoice.referenceMonth)}. O valor sai do
              seu saldo hoje. O que não for pago agora é debitado sozinho no vencimento.
            </DialogDescription>
          </DialogHeader>

          <Field data-invalid={submitted && !!amountError}>
            <FieldLabel htmlFor="pay-invoice-amount">Valor a pagar</FieldLabel>
            <MoneyInput
              id="pay-invoice-amount"
              autoFocus
              value={amount}
              onChange={setAmount}
              aria-invalid={submitted && !!amountError}
            />
            <FieldDescription>
              Ainda falta pagar {formatBRL(invoice.remaining)}. Pague tudo ou só uma parte.
            </FieldDescription>
            {submitted && amountError && <FieldError>{amountError}</FieldError>}
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
              Pagar
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
