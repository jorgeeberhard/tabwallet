"use client"

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { TriangleAlertIcon } from "lucide-react"
import { useState } from "react"
import { toast } from "sonner"

import { Money } from "@/components/money"
import { MoneyInput } from "@/components/money-input"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import {
  AlertDialog,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog"
import { Button } from "@/components/ui/button"
import { Checkbox } from "@/components/ui/checkbox"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Field, FieldDescription, FieldError, FieldLabel } from "@/components/ui/field"
import { Skeleton } from "@/components/ui/skeleton"
import { Spinner } from "@/components/ui/spinner"
import { parseAmountToCents } from "@/lib/amount"
import { BALANCE_KEY } from "@/lib/balance-api"
import { creditCardFailure, creditCardFieldMessage } from "@/lib/credit-card-messages"
import { CREDIT_CARDS_KEY, INVOICES_KEY, type InvoiceInstallment } from "@/lib/credit-cards-api"
import { formatBRL, formatMonthOfYear } from "@/lib/format"
import {
  ANTICIPABLE_KEY,
  anticipableKey,
  anticipateInstallments,
  discountTooLarge,
  listAnticipableInstallments,
  summarizeAnticipation,
} from "@/lib/installment-anticipation"
import { TRANSACTIONS_KEY } from "@/lib/transactions-api"

interface AnticipateInstallmentsDialogProps {
  cardId: string
  cardName: string
  onClose: () => void
}

/** The chosen Installments grouped by the invoice they are in, earliest first. */
function groupByInvoice(items: InvoiceInstallment[]) {
  const groups = new Map<string, InvoiceInstallment[]>()
  for (const item of items) {
    const key = item.invoice.referenceMonth
    groups.set(key, [...(groups.get(key) ?? []), item])
  }
  return [...groups.entries()]
}

function plural(count: number, one: string, many: string) {
  return count === 1 ? one : many
}

/**
 * UC19: brings Installments of invoices after the open one to the open
 * invoice, with an optional Discount. The person chooses the Installments and
 * types the Discount, then a confirmation warns that the operation cannot be
 * undone; only then is it sent (the API requires the confirmation too).
 * Mount it only while it is open.
 */
export function AnticipateInstallmentsDialog({
  cardId,
  cardName,
  onClose,
}: AnticipateInstallmentsDialogProps) {
  const queryClient = useQueryClient()
  const [selected, setSelected] = useState<Set<string>>(new Set())
  const [discount, setDiscount] = useState("")
  const [step, setStep] = useState<"choose" | "confirm">("choose")
  const [submitted, setSubmitted] = useState(false)
  const [problem, setProblem] = useState<string | null>(null)
  const [discountServerError, setDiscountServerError] = useState<string | null>(null)

  const anticipable = useQuery({
    queryKey: anticipableKey(cardId),
    queryFn: () => listAnticipableInstallments(cardId),
  })
  const items = anticipable.data?.installments ?? []
  const openInvoice = anticipable.data?.openInvoice
  const chosen = items.filter((item) => selected.has(item.id))

  const discountText = discount.trim()
  const discountCents = discountText === "" ? 0 : parseAmountToCents(discountText)
  const discountError =
    discountCents === null
      ? "Digite um valor maior que zero ou deixe em branco."
      : discountTooLarge(discountCents, chosen) && chosen.length > 0 && discountCents > 0
        ? "O desconto precisa ser menor que o total das parcelas escolhidas."
        : null
  const summary = summarizeAnticipation(chosen, discountCents ?? 0)
  const openMonth = openInvoice ? formatMonthOfYear(openInvoice.referenceMonth) : ""

  const anticipate = useMutation({
    mutationFn: () =>
      anticipateInstallments(cardId, {
        installmentIds: chosen.map((item) => item.id),
        discountCents: discountCents && discountCents > 0 ? discountCents : undefined,
      }),
    onSuccess: (result) => {
      void queryClient.invalidateQueries({ queryKey: INVOICES_KEY })
      void queryClient.invalidateQueries({ queryKey: CREDIT_CARDS_KEY })
      void queryClient.invalidateQueries({ queryKey: TRANSACTIONS_KEY })
      void queryClient.invalidateQueries({ queryKey: ANTICIPABLE_KEY })
      // The Projected Balance and the history follow the invoices.
      void queryClient.invalidateQueries({ queryKey: BALANCE_KEY })
      toast.success(
        `Parcelas antecipadas para a fatura de ${formatMonthOfYear(result.invoice.referenceMonth)}.`,
      )
      onClose()
    },
    onError: (error) => {
      setStep("choose")
      setDiscountServerError(creditCardFieldMessage(error, "discount_cents"))
      setProblem(creditCardFailure(error, "antecipar as parcelas"))
      // The list may be out of date (an Installment was paid or moved).
      void queryClient.invalidateQueries({ queryKey: ANTICIPABLE_KEY })
    },
  })
  const busy = anticipate.isPending

  function toggle(id: string, checked: boolean) {
    setSelected((current) => {
      const next = new Set(current)
      if (checked) next.add(id)
      else next.delete(id)
      return next
    })
    setDiscountServerError(null)
  }

  function toggleAll(checked: boolean) {
    setSelected(checked ? new Set(items.map((item) => item.id)) : new Set())
    setDiscountServerError(null)
  }

  function review(event: React.FormEvent) {
    event.preventDefault()
    setProblem(null)
    setSubmitted(true)
    if (chosen.length === 0 || discountError) return
    setStep("confirm")
  }

  const allChosen = items.length > 0 && chosen.length === items.length
  const shownDiscountError = submitted ? (discountError ?? discountServerError) : null

  return (
    <>
      <Dialog open={step === "choose"} onOpenChange={(open) => !open && !busy && onClose()}>
        <DialogContent showCloseButton={false}>
          <form onSubmit={review} noValidate className="grid gap-4">
            <DialogHeader>
              <DialogTitle className="font-display text-lg font-bold">
                Antecipar parcelas
              </DialogTitle>
              <DialogDescription>
                Traga parcelas de faturas futuras do {cardName} para a fatura aberta
                {openMonth && ` de ${openMonth}`}. Elas passam a contar nesse mês.
              </DialogDescription>
            </DialogHeader>

            {anticipable.isPending ? (
              <div className="flex flex-col gap-2" aria-label="Carregando parcelas">
                <Skeleton className="h-8 w-full" />
                <Skeleton className="h-8 w-full" />
                <Skeleton className="h-8 w-full" />
              </div>
            ) : anticipable.isError ? (
              <Alert variant="destructive">
                <TriangleAlertIcon strokeWidth={1.75} />
                <AlertDescription>
                  <p>Não foi possível carregar as parcelas deste cartão.</p>
                  <Button
                    type="button"
                    size="sm"
                    variant="outline"
                    className="mt-2"
                    onClick={() => void anticipable.refetch()}
                  >
                    Tentar de novo
                  </Button>
                </AlertDescription>
              </Alert>
            ) : items.length === 0 ? (
              <Alert variant="info">
                <AlertDescription>
                  Este cartão não tem parcelas em faturas futuras para antecipar. Parcelas da
                  fatura aberta, de faturas fechadas ou pagas e as já antecipadas não entram aqui.
                </AlertDescription>
              </Alert>
            ) : (
              <fieldset className="grid gap-2">
                <legend className="mb-1 text-sm font-medium">Parcelas a antecipar</legend>
                <label className="flex items-center gap-2 text-sm">
                  <Checkbox
                    checked={allChosen}
                    onCheckedChange={(checked) => toggleAll(checked === true)}
                    aria-label="Selecionar todas as parcelas"
                  />
                  <span className="font-medium">Selecionar todas</span>
                </label>
                <div className="flex max-h-64 flex-col gap-3 overflow-y-auto rounded-lg border p-3">
                  {groupByInvoice(items).map(([month, group]) => (
                    <div key={month} className="flex flex-col gap-1.5">
                      <p className="text-xs font-medium text-muted-foreground">
                        Fatura de {formatMonthOfYear(month)}
                      </p>
                      {group.map((item) => (
                        <label key={item.id} className="flex items-center gap-2 text-sm">
                          <Checkbox
                            checked={selected.has(item.id)}
                            onCheckedChange={(checked) => toggle(item.id, checked === true)}
                          />
                          <span className="min-w-0 flex-1 truncate">
                            {item.description}{" "}
                            <span className="tabular-nums text-muted-foreground">
                              {item.number}/{item.installmentCount}
                            </span>
                          </span>
                          <Money value={item.amount} showSign={false} size="sm" />
                        </label>
                      ))}
                    </div>
                  ))}
                </div>
                {submitted && chosen.length === 0 && (
                  <FieldError>Escolha pelo menos uma parcela.</FieldError>
                )}
              </fieldset>
            )}

            {items.length > 0 && (
              <>
                <Field data-invalid={!!shownDiscountError}>
                  <FieldLabel htmlFor="anticipation-discount">Desconto (opcional)</FieldLabel>
                  <MoneyInput
                    id="anticipation-discount"
                    value={discount}
                    onChange={(value) => {
                      setDiscount(value)
                      setDiscountServerError(null)
                    }}
                    aria-invalid={!!shownDiscountError}
                  />
                  <FieldDescription>
                    O valor que o banco abateu. Ele é dividido entre as parcelas escolhidas na
                    proporção do valor de cada uma e também reduz a despesa da categoria da compra.
                  </FieldDescription>
                  {shownDiscountError && <FieldError>{shownDiscountError}</FieldError>}
                </Field>

                <dl className="grid grid-cols-[1fr_auto] gap-x-4 gap-y-1 rounded-lg bg-muted p-3 text-sm">
                  <dt className="text-muted-foreground">
                    {summary.count} {plural(summary.count, "parcela escolhida", "parcelas escolhidas")}
                  </dt>
                  <dd className="text-right tabular-nums">{formatBRL(summary.originalCents / 100)}</dd>
                  <dt className="text-muted-foreground">Desconto</dt>
                  <dd className="text-right tabular-nums">
                    {summary.discountCents > 0 ? `− ${formatBRL(summary.discountCents / 100)}` : formatBRL(0)}
                  </dd>
                  <dt className="font-medium">
                    Total na fatura{openMonth && ` de ${openMonth}`}
                  </dt>
                  <dd className="text-right font-semibold tabular-nums" data-testid="anticipation-total">
                    {formatBRL(Math.max(summary.totalCents, 0) / 100)}
                  </dd>
                </dl>
              </>
            )}

            {problem && (
              <Alert variant="destructive">
                <TriangleAlertIcon strokeWidth={1.75} />
                <AlertDescription>{problem}</AlertDescription>
              </Alert>
            )}

            <DialogFooter className="-mx-6 -mb-6 mt-2 rounded-b-xl p-6 pt-4">
              <Button type="button" variant="outline" onClick={onClose}>
                Cancelar
              </Button>
              <Button type="submit" disabled={items.length === 0}>
                Continuar
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      <AlertDialog open={step === "confirm"} onOpenChange={(open) => !open && !busy && setStep("choose")}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>
              Antecipar {summary.count} {plural(summary.count, "parcela", "parcelas")}?
            </AlertDialogTitle>
            <AlertDialogDescription>
              {summary.count} {plural(summary.count, "parcela de", "parcelas de")} {cardName} (
              {formatBRL(summary.originalCents / 100)}
              {summary.discountCents > 0 && ` com desconto de ${formatBRL(summary.discountCents / 100)}`})
              {" "}vão para a fatura de {openMonth}, que ficará{" "}
              {formatBRL(Math.max(summary.totalCents, 0) / 100)} mais alta.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <Alert variant="warning">
            <TriangleAlertIcon strokeWidth={1.75} />
            <AlertTitle>Essa operação não pode ser desfeita.</AlertTitle>
            <AlertDescription>
              Depois de confirmar, as parcelas não voltam para as faturas de origem e o desconto
              não pode ser desfeito.
            </AlertDescription>
          </Alert>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={busy}>Voltar</AlertDialogCancel>
            <Button disabled={busy} onClick={() => anticipate.mutate()}>
              {busy && <Spinner />}
              Antecipar parcelas
            </Button>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  )
}
