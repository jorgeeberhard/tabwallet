import { LandmarkIcon } from "lucide-react"

import { Money } from "@/components/money"
import { Badge } from "@/components/ui/badge"
import type { InvoicePayment } from "@/lib/credit-cards-api"
import { formatDate, formatMonthOfYear } from "@/lib/format"

const PAYMENT_TITLE: Record<InvoicePayment["origin"], string> = {
  early: "Pagamento antecipado",
  automatic: "Pagamento automático",
  corrected: "Pagamento corrigido",
}

const PAYMENT_BADGE: Record<InvoicePayment["origin"], string> = {
  early: "Antecipado",
  automatic: "No vencimento",
  corrected: "Corrigido",
}

interface InvoicePaymentRowProps {
  payment: InvoicePayment
  /** The card it pays for; hidden on the invoice screen, which is about one card. */
  cardName?: string
  /** The reference month (`YYYY-MM`) of the invoice it pays. */
  referenceMonth: string
}

/**
 * One Invoice Payment (Design System `TransactionRow`): an Early Payment the
 * person made or the automatic one of the due date. It leaves the Current
 * Balance but is not an expense, so it is marked "Pagamento de fatura", shows
 * the card and the invoice, and uses a neutral amount instead of the expense
 * colour (the "−" still says it leaves the balance).
 */
export function InvoicePaymentRow({ payment, cardName, referenceMonth }: InvoicePaymentRowProps) {
  const title = PAYMENT_TITLE[payment.origin]
  const badge = PAYMENT_BADGE[payment.origin]

  return (
    <li className="flex items-center gap-3 bg-muted/40 px-4 py-3">
      <span className="flex size-9 shrink-0 items-center justify-center rounded-full bg-secondary text-secondary-foreground">
        <LandmarkIcon strokeWidth={1.75} className="size-4" aria-hidden />
      </span>

      <div className="flex min-w-0 flex-1 flex-col gap-1">
        <span className="truncate font-medium">
          {title} da fatura de {formatMonthOfYear(referenceMonth)}
        </span>
        <span className="flex flex-wrap items-center gap-x-2 gap-y-1 text-sm text-muted-foreground">
          <time dateTime={payment.date}>{formatDate(payment.date)}</time>
          <Badge variant="secondary">Pagamento de fatura</Badge>
          {cardName && <Badge variant="outline">{cardName}</Badge>}
          <Badge variant="outline">{badge}</Badge>
          {payment.isProjection && <Badge variant="warning">Projeção</Badge>}
        </span>
      </div>

      <Money value={-payment.amount} kind="neutral" />
    </li>
  )
}
