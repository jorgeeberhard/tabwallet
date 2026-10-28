import { api } from "@/lib/api"
import type { CategoryType } from "@/lib/categories-api"
import { centsToReais } from "@/lib/amount"

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
 * One entry of the statement. Only Transactions exist today; card purchases
 * and invoice payments will join as other `kind`s with their own shapes.
 */
export type StatementItem = { kind: "transaction" } & Transaction

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
  query.set("limit", String(filters.limit ?? PAGE_SIZE))
  if (filters.offset) query.set("offset", String(filters.offset))

  const payload = await api<{
    items: ({ kind: "transaction" } & TransactionPayload)[]
    total: number
    limit: number
    offset: number
  }>(`/transactions?${query.toString()}`)
  return {
    items: payload.items.map((item) => ({ kind: "transaction", ...toTransaction(item) })),
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
