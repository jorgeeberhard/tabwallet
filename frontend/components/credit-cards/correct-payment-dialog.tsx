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
import { Spinner } from "@/components/ui/spinner"
import { centsToReais, formatAmountInput, parseAmountOrZeroToCents } from "@/lib/amount"
import { BALANCE_KEY } from "@/lib/balance-api"
import { creditCardFailure } from "@/lib/credit-card-messages"
import {
  CREDIT_CARDS_KEY,
  correctInvoicePayment,
  INVOICES_KEY,
  type Invoice,
} from "@/lib/credit-cards-api"
import { formatBRL, formatMonthOfYear } from "@/lib/format"
import { TRANSACTIONS_KEY } from "@/lib/transactions-api"

interface CorrectPaymentDialogProps {
  cardId: string
  cardName: string
  invoice: Invoice
  onClose: () => void
}

/**
 * Corrects the payment of a paid invoice to the amount really paid at
 * the bank. The amount starts as what is paid now and cannot be more than the
 * invoice owed at the due date. Whatever is left unpaid becomes the Carried
 * Balance of the next invoice, which this dialog shows before saving. Mount it
 * only while it is open.
 */
export function CorrectPaymentDialog({
  cardId,
  cardName,
  invoice,
  onClose,
}: CorrectPaymentDialogProps) {
  const queryClient = useQueryClient()
  const { maxCents, correctedCents } = invoice.paymentCorrection
  const [amount, setAmount] = useState(formatAmountInput(correctedCents ?? maxCents))
  const [submitted, setSubmitted] = useState(false)
  const [problem, setProblem] = useState<string | null>(null)

  const correct = useMutation({
    mutationFn: () =>
      correctInvoicePayment(cardId, invoice.closingMonth, parseAmountOrZeroToCents(amount) as number),
    onSuccess: ({ invoice: updated }) => {
      void queryClient.invalidateQueries({ queryKey: INVOICES_KEY })
      void queryClient.invalidateQueries({ queryKey: CREDIT_CARDS_KEY })
      void queryClient.invalidateQueries({ queryKey: TRANSACTIONS_KEY })
      // The Current Balance, the history and the Projected Balance all changed.
      void queryClient.invalidateQueries({ queryKey: BALANCE_KEY })
      toast.success(
        updated.carriedForwardCents > 0
          ? `Pagamento corrigido. ${formatBRL(centsToReais(updated.carriedForwardCents))} seguem como saldo rolado na próxima fatura.`
          : "Pagamento corrigido.",
      )
      onClose()
    },
    onError: (error) => setProblem(creditCardFailure(error, "corrigir o pagamento")),
  })

  const cents = parseAmountOrZeroToCents(amount)
  const amountError =
    cents === null
      ? "Digite o valor que você pagou (pode ser zero)."
      : cents > maxCents
        ? `O valor não pode passar do que a fatura devia no vencimento (${formatBRL(centsToReais(maxCents))}).`
        : null
  const carried = cents !== null && cents <= maxCents ? maxCents - cents : null
  const busy = correct.isPending

  function submit(event: React.FormEvent) {
    event.preventDefault()
    setProblem(null)
    setSubmitted(true)
    if (amountError) return
    correct.mutate()
  }

  return (
    <Dialog open onOpenChange={(open) => !open && !busy && onClose()}>
      <DialogContent showCloseButton={false}>
        <form onSubmit={submit} noValidate className="grid gap-4">
          <DialogHeader>
            <DialogTitle className="font-display text-lg font-bold">Corrigir pagamento</DialogTitle>
            <DialogDescription>
              Fatura de {cardName} de {formatMonthOfYear(invoice.referenceMonth)}. No vencimento o
              sistema debitou {formatBRL(centsToReais(maxCents))}. Informe quanto você realmente pagou no
              banco.
            </DialogDescription>
          </DialogHeader>

          <Field data-invalid={submitted && !!amountError}>
            <FieldLabel htmlFor="correct-payment-amount">Valor realmente pago</FieldLabel>
            <MoneyInput
              id="correct-payment-amount"
              autoFocus
              value={amount}
              onChange={setAmount}
              aria-invalid={submitted && !!amountError}
            />
            <FieldDescription>
              Pague tudo ou só uma parte. O saldo é ajustado para o valor informado.
            </FieldDescription>
            {submitted && amountError && <FieldError>{amountError}</FieldError>}
          </Field>

          {carried !== null && carried > 0 && (
            <Alert>
              <InfoIcon strokeWidth={1.75} />
              <AlertDescription>
                {formatBRL(centsToReais(carried))} que ficaram sem pagar vão como saldo rolado para a
                próxima fatura. Eles não contam como nova despesa. Se o banco cobrou juros, lance
                como encargos na próxima fatura.
              </AlertDescription>
            </Alert>
          )}

          <p className="text-sm text-muted-foreground">
            Você pode corrigir de novo até a fatura seguinte ser paga.
          </p>

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
              Salvar
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
