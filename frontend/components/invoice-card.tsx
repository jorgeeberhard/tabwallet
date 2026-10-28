import { cn } from "cn"

import { Money } from "@/components/money"
import { Badge } from "@/components/ui/badge"
import { Card, CardContent, CardHeader } from "@/components/ui/card"
import { formatDate, formatMonthOfYear } from "@/lib/format"
import type { Invoice, InvoiceState } from "@/lib/credit-cards-api"

const STATE_LABEL: Record<InvoiceState, string> = {
  open: "Aberta",
  closed: "Fechada",
  paid: "Paga",
}

const STATE_VARIANT = {
  open: "brand",
  closed: "warning",
  paid: "income",
} as const

/** The badge text of an invoice state ("Aberta", "Fechada", "Paga"). */
export function invoiceStateLabel(state: InvoiceState): string {
  return STATE_LABEL[state]
}

export interface InvoiceCardProps {
  cardName: string
  invoice: Invoice
  className?: string
}

/**
 * One credit-card invoice (Design System `InvoiceCard`): card, month, total,
 * state, closing and due dates. The month is the one of the due date, where
 * the invoice's purchases count (the Reference Month). Advance payments join
 * with the ticket that creates invoice payments.
 */
export function InvoiceCard({ cardName, invoice, className }: InvoiceCardProps) {
  return (
    <Card className={cn("w-full max-w-xl", className)}>
      <CardHeader>
        <div className="flex items-start justify-between gap-4">
          <div className="flex flex-col gap-1">
            <h2 className="text-sm font-medium text-muted-foreground">Fatura de {cardName}</h2>
            <p className="font-display text-lg font-bold first-letter:uppercase">
              {formatMonthOfYear(invoice.referenceMonth)}
            </p>
          </div>
          <Badge variant={STATE_VARIANT[invoice.state]}>{STATE_LABEL[invoice.state]}</Badge>
        </div>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        <Money value={invoice.total} showSign={false} size="lg" />
        {invoice.state !== "paid" && invoice.remainingCents !== invoice.totalCents && (
          <p className="text-sm text-muted-foreground">
            Restante: <Money value={invoice.remaining} showSign={false} size="sm" />
          </p>
        )}
        <dl className="grid grid-cols-2 gap-x-4 text-sm">
          <div className="flex flex-col">
            <dt className="text-muted-foreground">
              {invoice.state === "open" ? "Fecha em" : "Fechou em"}
            </dt>
            <dd className="font-medium">
              <time dateTime={invoice.closingDate}>{formatDate(invoice.closingDate)}</time>
            </dd>
          </div>
          <div className="flex flex-col">
            <dt className="text-muted-foreground">
              {invoice.state === "paid" ? "Venceu em" : "Vence em"}
            </dt>
            <dd className="font-medium">
              <time dateTime={invoice.dueDate}>{formatDate(invoice.dueDate)}</time>
            </dd>
          </div>
        </dl>
      </CardContent>
    </Card>
  )
}
