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
import { budgetFailure } from "@/lib/budget-messages"
import { BUDGETS_KEY, setBudget } from "@/lib/budgets-api"
import { formatMonthOfYear } from "@/lib/format"

export interface BudgetDialogTarget {
  categoryId: string
  categoryName: string
  /** The limit in force, when the budget is being changed; `null` for a new one. */
  currentLimitCents: number | null
}

interface BudgetDialogProps {
  target: BudgetDialogTarget
  /** The current month (`YYYY-MM`): the month the new limit starts to apply in. */
  currentMonth: string
  onClose: () => void
}

/**
 * UC12: sets or changes the monthly limit of an expense category. The limit
 * applies from the current month on, every month, until it is changed again;
 * past months keep what they had. Mount it only while it is open.
 */
export function BudgetDialog({ target, currentMonth, onClose }: BudgetDialogProps) {
  const queryClient = useQueryClient()
  const [limit, setLimit] = useState(
    target.currentLimitCents === null ? "" : formatAmountInput(target.currentLimitCents),
  )
  const [submitted, setSubmitted] = useState(false)
  const [problem, setProblem] = useState<string | null>(null)

  const save = useMutation({
    mutationFn: () => setBudget(target.categoryId, parseAmountToCents(limit) as number),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: BUDGETS_KEY })
      toast.success(`Orçamento de ${target.categoryName} salvo.`)
      onClose()
    },
    onError: (error) => setProblem(budgetFailure(error, "salvar o orçamento")),
  })

  const limitError =
    parseAmountToCents(limit) === null ? "Digite um limite maior que zero." : null
  const busy = save.isPending

  function submit(event: React.FormEvent) {
    event.preventDefault()
    setProblem(null)
    setSubmitted(true)
    if (limitError) return
    save.mutate()
  }

  return (
    <Dialog open onOpenChange={(open) => !open && !busy && onClose()}>
      <DialogContent showCloseButton={false}>
        <form onSubmit={submit} noValidate className="grid gap-4">
          <DialogHeader>
            <DialogTitle className="font-display text-lg font-bold">
              {target.currentLimitCents === null ? "Definir orçamento" : "Alterar orçamento"}
            </DialogTitle>
            <DialogDescription>
              Limite mensal de {target.categoryName}. Ele vale todo mês, a partir de{" "}
              {formatMonthOfYear(currentMonth)}, até você alterá-lo. Os meses anteriores não
              mudam.
            </DialogDescription>
          </DialogHeader>

          <Field data-invalid={submitted && !!limitError}>
            <FieldLabel htmlFor="budget-limit">Limite por mês</FieldLabel>
            <MoneyInput
              id="budget-limit"
              autoFocus
              value={limit}
              onChange={setLimit}
              aria-invalid={submitted && !!limitError}
            />
            {submitted && limitError && <FieldError>{limitError}</FieldError>}
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
              Salvar orçamento
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
