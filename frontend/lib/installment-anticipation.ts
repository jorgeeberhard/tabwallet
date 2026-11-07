import { api } from "@/lib/api"
import {
  type Invoice,
  type InvoiceCycle,
  type InvoiceInstallment,
  type InvoiceInstallmentPayload,
  toInvoiceInstallment,
} from "@/lib/credit-cards-api"

/** Query key of the Installments a card lets the person anticipate. */
export const ANTICIPABLE_KEY = ["anticipable-installments"] as const

export function anticipableKey(cardId: string) {
  return [...ANTICIPABLE_KEY, cardId] as const
}

/** The Installments of a card that sit in invoices after the open one, and that open invoice. */
export interface AnticipableInstallments {
  /** The invoice that currently receives purchases: where the Installments would go. */
  openInvoice: InvoiceCycle
  /** Earliest invoice first. */
  installments: InvoiceInstallment[]
}

interface InvoiceCyclePayload {
  closing_month: string
  closing_date: string
  due_date: string
  reference_month: string
  state: InvoiceCycle["state"]
}

export async function listAnticipableInstallments(
  cardId: string,
): Promise<AnticipableInstallments> {
  const payload = await api<{
    open_invoice: InvoiceCyclePayload
    installments: InvoiceInstallmentPayload[]
  }>(`/credit-cards/${cardId}/anticipable-installments`)
  return {
    openInvoice: {
      closingMonth: payload.open_invoice.closing_month,
      closingDate: payload.open_invoice.closing_date,
      dueDate: payload.open_invoice.due_date,
      referenceMonth: payload.open_invoice.reference_month,
      state: payload.open_invoice.state,
    },
    installments: payload.installments.map(toInvoiceInstallment),
  }
}

export interface AnticipationInput {
  installmentIds: string[]
  /** Cents of Discount, more than zero; leave out for none. */
  discountCents?: number
}

/** What an Installment Anticipation did. */
export interface Anticipation {
  originalTotalCents: number
  discountCents: number
  /** What the Installments add up to in the open invoice after the Discount. */
  totalCents: number
  /** The open invoice after receiving them. */
  invoice: Pick<Invoice, "closingMonth" | "referenceMonth" | "totalCents">
}

/**
 * UC19: brings the chosen Installments to the open invoice. It cannot be
 * undone, so the API only does it with `confirm: true`; the dialog that calls
 * this has already shown the warning and asked.
 */
export async function anticipateInstallments(
  cardId: string,
  input: AnticipationInput,
): Promise<Anticipation> {
  const { anticipation } = await api<{
    anticipation: {
      original_total_cents: number
      discount_cents: number
      total_cents: number
      invoice: { closing_month: string; reference_month: string; total_cents: number }
    }
  }>(`/credit-cards/${cardId}/installment-anticipations`, {
    method: "POST",
    body: {
      installment_ids: input.installmentIds,
      discount_cents: input.discountCents,
      confirm: true,
    },
  })
  return {
    originalTotalCents: anticipation.original_total_cents,
    discountCents: anticipation.discount_cents,
    totalCents: anticipation.total_cents,
    invoice: {
      closingMonth: anticipation.invoice.closing_month,
      referenceMonth: anticipation.invoice.reference_month,
      totalCents: anticipation.invoice.total_cents,
    },
  }
}

/** The numbers the dialog shows for the chosen Installments and the Discount. */
export interface AnticipationSummary {
  count: number
  originalCents: number
  discountCents: number
  /** What the chosen Installments cost in the open invoice. */
  totalCents: number
}

export function summarizeAnticipation(
  chosen: Pick<InvoiceInstallment, "amountCents">[],
  discountCents: number,
): AnticipationSummary {
  const originalCents = chosen.reduce((sum, item) => sum + item.amountCents, 0)
  return {
    count: chosen.length,
    originalCents,
    discountCents,
    totalCents: originalCents - discountCents,
  }
}

/**
 * Whether a Discount is more than the chosen Installments allow: it can never
 * reach their sum, because an Installment cannot be left free. The API
 * checks the exact split between them.
 */
export function discountTooLarge(
  discountCents: number,
  chosen: Pick<InvoiceInstallment, "amountCents">[],
): boolean {
  return discountCents >= summarizeAnticipation(chosen, 0).originalCents
}
