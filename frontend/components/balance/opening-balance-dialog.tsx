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
import { parseSignedAmountToCents } from "@/lib/amount"
import { ApiError } from "@/lib/api"
import { BALANCE_KEY, setOpeningBalance } from "@/lib/balance-api"
import { balanceFailure } from "@/lib/balance-messages"

interface OpeningBalanceDialogProps {
  onClose: () => void
}

/** Informs the Opening Balance, which can only be done once. Mount it only while it is open. */
export function OpeningBalanceDialog({ onClose }: OpeningBalanceDialogProps) {
  const queryClient = useQueryClient()
  const [amount, setAmount] = useState("")
  const [submitted, setSubmitted] = useState(false)
  const [problem, setProblem] = useState<string | null>(null)

  const save = useMutation({
    mutationFn: (cents: number) => setOpeningBalance(cents),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: BALANCE_KEY })
      toast.success("Saldo inicial salvo.")
      onClose()
    },
    onError: (error) => {
      setProblem(balanceFailure(error, "salvar o saldo inicial"))
      // Informed elsewhere in the meantime: show the balance as it is now.
      if (error instanceof ApiError && error.code === "opening_balance_already_set") {
        void queryClient.invalidateQueries({ queryKey: BALANCE_KEY })
      }
    },
  })

  const cents = parseSignedAmountToCents(amount)
  const error = submitted && cents === null ? "Digite um valor, como 1.500,00 ou -200,00." : null
  const busy = save.isPending

  function submit(event: React.FormEvent) {
    event.preventDefault()
    setProblem(null)
    setSubmitted(true)
    if (cents === null) return
    save.mutate(cents)
  }

  return (
    <Dialog open onOpenChange={(open) => !open && !busy && onClose()}>
      <DialogContent showCloseButton={false}>
        <form onSubmit={submit} noValidate className="grid gap-4">
          <DialogHeader>
            <DialogTitle className="font-display text-lg font-bold">
              Informar saldo inicial
            </DialogTitle>
            <DialogDescription>
              Quanto você já tinha ao começar a usar o TabWallet. Você só pode informar o saldo
              inicial uma vez, então confira o valor antes de salvar.
            </DialogDescription>
          </DialogHeader>

          <Field data-invalid={!!error}>
            <FieldLabel htmlFor="opening-balance-amount">Saldo inicial</FieldLabel>
            <MoneyInput
              id="opening-balance-amount"
              signed
              autoFocus
              value={amount}
              onChange={setAmount}
              aria-invalid={!!error}
            />
            {error ? (
              <FieldError>{error}</FieldError>
            ) : (
              <FieldDescription>
                Pode ser zero. Se você começou devendo, digite o valor com um sinal de menos.
              </FieldDescription>
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
              Salvar saldo inicial
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
