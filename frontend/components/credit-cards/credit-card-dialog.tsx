"use client"

import { useMutation, useQueryClient } from "@tanstack/react-query"
import { InfoIcon, TriangleAlertIcon } from "lucide-react"
import { useState } from "react"
import { toast } from "sonner"

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
import { Spinner } from "@/components/ui/spinner"
import {
  CREDIT_CARDS_KEY,
  type CreditCard,
  createCreditCard,
  INVOICES_KEY,
  updateCreditCard,
} from "@/lib/credit-cards-api"
import { creditCardFailure } from "@/lib/credit-card-messages"

export const MAX_CARD_NAME_LENGTH = 60

export type CreditCardDialogMode = { kind: "create" } | { kind: "edit"; card: CreditCard }

interface CreditCardDialogProps {
  mode: CreditCardDialogMode
  onClose: () => void
}

/** A day of the month as typed: 1 to 31, or null. */
function parseDay(text: string): number | null {
  if (!/^\d{1,2}$/.test(text.trim())) return null
  const day = Number(text)
  return day >= 1 && day <= 31 ? day : null
}

/** Create a credit card or edit its name and days. Mount it only while it is open. */
export function CreditCardDialog({ mode, onClose }: CreditCardDialogProps) {
  const queryClient = useQueryClient()
  const editing = mode.kind === "edit" ? mode.card : null
  const [name, setName] = useState(editing?.name ?? "")
  const [closingDay, setClosingDay] = useState(editing ? String(editing.closingDay) : "")
  const [dueDay, setDueDay] = useState(editing ? String(editing.dueDay) : "")
  const [submitted, setSubmitted] = useState(false)
  const [problem, setProblem] = useState<string | null>(null)

  const save = useMutation({
    mutationFn: () => {
      const fields = {
        name: name.trim(),
        closingDay: parseDay(closingDay) as number,
        dueDay: parseDay(dueDay) as number,
      }
      return editing ? updateCreditCard(editing.id, fields) : createCreditCard(fields)
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: CREDIT_CARDS_KEY })
      void queryClient.invalidateQueries({ queryKey: INVOICES_KEY })
      toast.success(editing ? "Cartão atualizado." : "Cartão cadastrado.")
      onClose()
    },
    onError: (error) =>
      setProblem(creditCardFailure(error, editing ? "salvar o cartão" : "cadastrar o cartão")),
  })

  const trimmed = name.trim()
  const errors = {
    name:
      trimmed.length === 0
        ? "Digite o nome do cartão."
        : trimmed.length > MAX_CARD_NAME_LENGTH
          ? `Use até ${MAX_CARD_NAME_LENGTH} caracteres.`
          : null,
    closingDay: parseDay(closingDay) === null ? "Escolha um dia de 1 a 31." : null,
    dueDay: parseDay(dueDay) === null ? "Escolha um dia de 1 a 31." : null,
  }
  const show = (field: keyof typeof errors) => (submitted ? errors[field] : null)

  function submit(event: React.FormEvent) {
    event.preventDefault()
    setProblem(null)
    setSubmitted(true)
    if (Object.values(errors).some(Boolean)) return
    save.mutate()
  }

  const busy = save.isPending

  return (
    <Dialog open onOpenChange={(open) => !open && !busy && onClose()}>
      <DialogContent showCloseButton={false}>
        <form onSubmit={submit} noValidate className="grid gap-4">
          <DialogHeader>
            <DialogTitle className="font-display text-lg font-bold">
              {editing ? "Editar cartão" : "Novo cartão"}
            </DialogTitle>
            <DialogDescription>
              Compras feitas até o dia de fechamento (inclusive) entram na fatura que fecha no mês;
              depois dele, na do mês seguinte. Em meses mais curtos vale o último dia.
            </DialogDescription>
          </DialogHeader>

          <Field data-invalid={!!show("name")}>
            <FieldLabel htmlFor="card-name">Nome</FieldLabel>
            <Input
              id="card-name"
              autoFocus
              autoComplete="off"
              placeholder="Ex.: Nubank"
              value={name}
              aria-invalid={!!show("name")}
              onChange={(event) => setName(event.target.value)}
            />
            {show("name") && <FieldError>{show("name")}</FieldError>}
          </Field>

          <div className="grid grid-cols-2 gap-4">
            <Field data-invalid={!!show("closingDay")}>
              <FieldLabel htmlFor="card-closing-day">Dia de fechamento</FieldLabel>
              <Input
                id="card-closing-day"
                inputMode="numeric"
                autoComplete="off"
                placeholder="1 a 31"
                value={closingDay}
                aria-invalid={!!show("closingDay")}
                onChange={(event) => setClosingDay(event.target.value)}
              />
              {show("closingDay") && <FieldError>{show("closingDay")}</FieldError>}
            </Field>
            <Field data-invalid={!!show("dueDay")}>
              <FieldLabel htmlFor="card-due-day">Dia de vencimento</FieldLabel>
              <Input
                id="card-due-day"
                inputMode="numeric"
                autoComplete="off"
                placeholder="1 a 31"
                value={dueDay}
                aria-invalid={!!show("dueDay")}
                onChange={(event) => setDueDay(event.target.value)}
              />
              {show("dueDay") && <FieldError>{show("dueDay")}</FieldError>}
            </Field>
          </div>

          {editing && (
            <p className="flex items-start gap-1.5 text-sm text-muted-foreground">
              <InfoIcon strokeWidth={1.75} className="mt-0.5 size-4 shrink-0" aria-hidden />
              Novos dias valem só para faturas ainda abertas. As faturas fechadas e as pagas
              mantêm as datas que tinham.
            </p>
          )}

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
              {editing ? "Salvar alterações" : "Cadastrar cartão"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
