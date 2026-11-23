import { api } from "@/lib/api"
import type { CategoryType } from "@/lib/categories-api"
import { centsToReais } from "@/lib/amount"
import {
  type CardPurchase,
  type CardPurchasePayload,
  type Charge,
  type ChargePayload,
  toCharge,
  type InstallmentPurchase,
  type InstallmentPurchasePayload,
  toCardPurchase,
  toInstallmentPurchase,
  type InvoicePaymentItem,
  type InvoicePaymentItemPayload,
  toInvoicePaymentItem,
} from "@/lib/credit-cards-api"
import { type Occurrence, type OccurrencePayload, toOccurrence } from "@/lib/recurring-api"

export type TransactionType = CategoryType

export interface Transaction {
  id: string
  type: TransactionType
  /** Always greater than zero; `type` gives the direction. */
  amountCents: number
  /** Amount in reais, for the Design System components. */
  amount: number
  /** A calendar day, `YYYY-MM-DD`. */
  date: string
  description: string
  category: { id: string; name: string; active: boolean }
  /** The date is after today: not in the current balance yet. */
  isProjection: boolean
}

interface TransactionPayload {
  id: string
  type: TransactionType
  amount_cents: number
  date: string
  description: string
  category: { id: string; name: string; type: TransactionType; active: boolean }
  is_projection: boolean
}

function toTransaction(payload: TransactionPayload): Transaction {
  return {
    id: payload.id,
    type: payload.type,
    amountCents: payload.amount_cents,
    amount: centsToReais(payload.amount_cents),
    date: payload.date,
    description: payload.description,
    category: {
      id: payload.category.id,
      name: payload.category.name,
      active: payload.category.active,
    },
    isProjection: payload.is_projection,
  }
}

/**
 * One entry of the statement. `kind` says what the rest of the object is: a
 * Transaction, a card purchase (with its card and invoice) or an Occurrence
 * of a Gasto fixo or Receita fixa (computed by the API on read) or an Invoice
 * Payment (early, or the automatic one of a due date), which leaves the
 * balance but is not an expense.
 */
export type StatementItem =
  | ({ kind: "transaction" } & Transaction)
  | ({ kind: "card_purchase" } & CardPurchase)
  | ({ kind: "installment_purchase" } & InstallmentPurchase)
  | ({ kind: "charge" } & Charge)
  | ({ kind: "occurrence" } & Occurrence)
  | ({ kind: "invoice_payment" } & InvoicePaymentItem)

/** The key that identifies a statement item in a list. */
export function statementItemKey(item: StatementItem): string {
  if (item.kind === "occurrence") return `${item.recurringId}:${item.month}`
  // The automatic payment has no id: the invoice it pays identifies it.
  if (item.kind === "invoice_payment") {
    return item.id ?? `automatic:${item.creditCard.id}:${item.invoice.closingMonth}`
  }
  return item.id
}

export interface Statement {
  items: StatementItem[]
  total: number
  limit: number
  offset: number
}

export interface TransactionFilters {
  /** `YYYY-MM-DD`, inclusive. */
  from?: string
  /** `YYYY-MM-DD`, inclusive. */
  to?: string
  categoryId?: string
  type?: TransactionType
  /** Only purchases and invoice payments of this credit card. */
  creditCardId?: string
  limit?: number
  offset?: number
}

/** Page size of the statement; the API default and a good fit for one screen. */
export const PAGE_SIZE = 50

/** Query key prefix of every statement query; mutations invalidate it. */
export const TRANSACTIONS_KEY = ["transactions"] as const

export function statementKey(filters: TransactionFilters) {
  return [...TRANSACTIONS_KEY, filters] as const
}

export async function listTransactions(filters: TransactionFilters = {}): Promise<Statement> {
  const query = new URLSearchParams()
  if (filters.from) query.set("from", filters.from)
  if (filters.to) query.set("to", filters.to)
  if (filters.categoryId) query.set("category_id", filters.categoryId)
  if (filters.type) query.set("type", filters.type)
  if (filters.creditCardId) query.set("credit_card_id", filters.creditCardId)
  query.set("limit", String(filters.limit ?? PAGE_SIZE))
  if (filters.offset) query.set("offset", String(filters.offset))

  const payload = await api<{
    items: (
      | ({ kind: "transaction" } & TransactionPayload)
      | ({ kind: "card_purchase" } & CardPurchasePayload)
      | ({ kind: "installment_purchase" } & InstallmentPurchasePayload)
      | ({ kind: "charge" } & ChargePayload)
      | ({ kind: "occurrence" } & OccurrencePayload)
      | ({ kind: "invoice_payment" } & InvoicePaymentItemPayload)
    )[]
    total: number
    limit: number
    offset: number
  }>(`/transactions?${query.toString()}`)
  return {
    items: payload.items.map(
      (item): StatementItem =>
        item.kind === "card_purchase"
          ? { kind: "card_purchase", ...toCardPurchase(item) }
          : item.kind === "installment_purchase"
            ? { kind: "installment_purchase", ...toInstallmentPurchase(item) }
            : item.kind === "charge"
              ? { kind: "charge", ...toCharge(item) }
              : item.kind === "occurrence"
                ? { kind: "occurrence", ...toOccurrence(item) }
                : item.kind === "invoice_payment"
                  ? { kind: "invoice_payment", ...toInvoicePaymentItem(item) }
                  : { kind: "transaction", ...toTransaction(item) },
    ),
    total: payload.total,
    limit: payload.limit,
    offset: payload.offset,
  }
}

export interface TransactionInput {
  type: TransactionType
  amountCents: number
  date: string
  description: string
  categoryId: string
}

export async function createTransaction(input: TransactionInput): Promise<Transaction> {
  const { transaction } = await api<{ transaction: TransactionPayload }>("/transactions", {
    method: "POST",
    body: {
      type: input.type,
      amount_cents: input.amountCents,
      date: input.date,
      description: input.description,
      category_id: input.categoryId,
    },
  })
  return toTransaction(transaction)
}

/** What an edit may change: the type never changes. */
export type TransactionChanges = Partial<Omit<TransactionInput, "type">>

export async function updateTransaction(
  id: string,
  changes: TransactionChanges,
): Promise<Transaction> {
  const { transaction } = await api<{ transaction: TransactionPayload }>(`/transactions/${id}`, {
    method: "PATCH",
    body: {
      amount_cents: changes.amountCents,
      date: changes.date,
      description: changes.description,
      category_id: changes.categoryId,
    },
  })
  return toTransaction(transaction)
}

export async function deleteTransaction(id: string): Promise<void> {
  await api(`/transactions/${id}`, { method: "DELETE" })
}
