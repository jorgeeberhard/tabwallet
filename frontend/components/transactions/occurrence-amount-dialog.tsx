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
import { Field, FieldError, FieldLabel } from "@/components/ui/field"
import { Spinner } from "@/components/ui/spinner"
import { formatAmountInput, parseAmountToCents } from "@/lib/amount"
import { BALANCE_KEY } from "@/lib/balance-api"
import { formatDate, formatMonth } from "@/lib/format"
import { recurringFailure } from "@/lib/recurring-messages"
import { editOccurrenceAmount, type Occurrence } from "@/lib/recurring-api"
import { TRANSACTIONS_KEY } from "@/lib/transactions-api"

interface OccurrenceAmountDialogProps {
  occurrence: Occurrence
  onClose: () => void
}

/**
 * Edits the amount of one single Occurrence of a Gasto fixo or Receita fixa
 * (UC06). The other months keep the amount of the fixed transaction. Mount it
 * only while it is open.
 */
export function OccurrenceAmountDialog({ occurrence, onClose }: OccurrenceAmountDialogProps) {
  const queryClient = useQueryClient()
  const [amount, setAmount] = useState(formatAmountInput(occurrence.amountCents))
  const [submitted, setSubmitted] = useState(false)
  const [problem, setProblem] = useState<string | null>(null)

  const save = useMutation({
    mutationFn: () =>
      editOccurrenceAmount(
        occurrence.recurringId,
        occurrence.month,
        parseAmountToCents(amount) as number,
      ),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: TRANSACTIONS_KEY })
      void queryClient.invalidateQueries({ queryKey: BALANCE_KEY })
      toast.success("Valor deste mês atualizado.")
      onClose()
    },
    onError: (error) => setProblem(recurringFailure(error, "salvar o valor")),
  })

  const amountError = parseAmountToCents(amount) === null ? "Digite um valor maior que zero." : null
  const busy = save.isPending

  function submit(event: React.FormEvent) {
    event.preventDefault()
    setProblem(null)
    setSubmitted(true)
    if (amountError) return
    save.mutate()
  }

  return (
    <Dialog open onOpenChange={(open) => !open && !busy && onClose()}>
      <DialogContent showCloseButton={false}>
        <form onSubmit={submit} noValidate className="grid gap-4">
          <DialogHeader>
            <DialogTitle className="font-display text-lg font-bold">
              Editar valor só deste mês
            </DialogTitle>
            <DialogDescription>
              {occurrence.description}, em {formatDate(occurrence.date)} (
              {formatMonth(occurrence.date)}). Os outros meses continuam com o valor do lançamento
              fixo.
            </DialogDescription>
          </DialogHeader>

          <Field data-invalid={submitted && !!amountError}>
            <FieldLabel htmlFor="occurrence-amount">Valor deste mês</FieldLabel>
            <MoneyInput
              id="occurrence-amount"
              autoFocus
              value={amount}
              onChange={setAmount}
              aria-invalid={submitted && !!amountError}
            />
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
              Salvar valor
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
