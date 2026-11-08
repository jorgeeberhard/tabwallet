import { centsToReais } from "@/lib/amount"
import { api } from "@/lib/api"
import type { CategoryType } from "@/lib/categories-api"

/** A Gasto fixo (expense) or Receita fixa (income). */
export interface RecurringTransaction {
  id: string
  type: CategoryType
  /** The amount in force this month, in cents. */
  amountCents: number
  /** Amount in reais, for the Design System components. */
  amount: number
  /** 1 to 31; in shorter months the Occurrence falls on the last day. */
  dayOfMonth: number
  description: string
  category: { id: string; name: string; active: boolean }
  /** First month with an Occurrence, `YYYY-MM`. */
  startMonth: string
  /** Last month with an Occurrence, `YYYY-MM`, once it was ended. */
  endMonth: string | null
  ended: boolean
  /** The day (`YYYY-MM-DD`) of the next Occurrence after today, if any. */
  nextDate: string | null
}

interface CategoryPayload {
  id: string
  name: string
  type: CategoryType
  active: boolean
}

interface RecurringPayload {
  id: string
  type: CategoryType
  amount_cents: number
  day_of_month: number
  description: string
  category: CategoryPayload
  start_month: string
  end_month: string | null
  ended: boolean
  next_date: string | null
}

function toRecurring(payload: RecurringPayload): RecurringTransaction {
  return {
    id: payload.id,
    type: payload.type,
    amountCents: payload.amount_cents,
    amount: centsToReais(payload.amount_cents),
    dayOfMonth: payload.day_of_month,
    description: payload.description,
    category: {
      id: payload.category.id,
      name: payload.category.name,
      active: payload.category.active,
    },
    startMonth: payload.start_month,
    endMonth: payload.end_month,
    ended: payload.ended,
    nextDate: payload.next_date,
  }
}

/**
 * The Transaction a recurring one makes in one month. It is computed by the
 * API on every read, never stored, and is a Projection while its date is
 * after today.
 */
export interface Occurrence {
  recurringId: string
  /** `YYYY-MM`: with `recurringId`, it identifies the Occurrence. */
  month: string
  type: CategoryType
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
  /** The amount of this single Occurrence was edited. */
  isEdited: boolean
}

export interface OccurrencePayload {
  recurring_id: string
  month: string
  type: CategoryType
  amount_cents: number
  date: string
  description: string
  category: CategoryPayload
  is_projection: boolean
  is_edited: boolean
}

export function toOccurrence(payload: OccurrencePayload): Occurrence {
  return {
    recurringId: payload.recurring_id,
    month: payload.month,
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
    isEdited: payload.is_edited,
  }
}

/** Query key of the list of recurring Transactions; mutations invalidate it. */
export const RECURRING_KEY = ["recurring-transactions"] as const

export async function listRecurring(): Promise<RecurringTransaction[]> {
  const { recurring_transactions } = await api<{ recurring_transactions: RecurringPayload[] }>(
    "/recurring-transactions",
  )
  return recurring_transactions.map(toRecurring)
}

export interface RecurringInput {
  type: CategoryType
  amountCents: number
  dayOfMonth: number
  description: string
  categoryId: string
  /** `YYYY-MM`; the API defaults to this month and refuses an earlier one. */
  startMonth?: string
}

export async function createRecurring(input: RecurringInput): Promise<RecurringTransaction> {
  const { recurring_transaction } = await api<{ recurring_transaction: RecurringPayload }>(
    "/recurring-transactions",
    {
      method: "POST",
      body: {
        type: input.type,
        amount_cents: input.amountCents,
        day_of_month: input.dayOfMonth,
        description: input.description,
        category_id: input.categoryId,
        start_month: input.startMonth,
      },
    },
  )
  return toRecurring(recurring_transaction)
}

/** What an edit may change: the type and the day never change. */
export type RecurringChanges = Partial<Pick<RecurringInput, "amountCents" | "description" | "categoryId">>

/** A new amount applies from this month on; earlier months keep theirs. */
export async function updateRecurring(
  id: string,
  changes: RecurringChanges,
): Promise<RecurringTransaction> {
  const { recurring_transaction } = await api<{ recurring_transaction: RecurringPayload }>(
    `/recurring-transactions/${id}`,
    {
      method: "PATCH",
      body: {
        amount_cents: changes.amountCents,
        description: changes.description,
        category_id: changes.categoryId,
      },
    },
  )
  return toRecurring(recurring_transaction)
}

/** Ends it: future Occurrences disappear, the ones that already fell stay. */
export async function endRecurring(id: string): Promise<RecurringTransaction> {
  const { recurring_transaction } = await api<{ recurring_transaction: RecurringPayload }>(
    `/recurring-transactions/${id}/end`,
    { method: "POST" },
  )
  return toRecurring(recurring_transaction)
}

/** Edits the amount of one single Occurrence; the others keep theirs. */
export async function editOccurrenceAmount(
  recurringId: string,
  month: string,
  amountCents: number,
): Promise<Occurrence> {
  const { occurrence } = await api<{ occurrence: OccurrencePayload }>(
    `/recurring-transactions/${recurringId}/occurrences/${month}`,
    { method: "PATCH", body: { amount_cents: amountCents } },
  )
  return toOccurrence(occurrence)
}

/** Deletes one single Occurrence; the others stay. */
export async function skipOccurrence(recurringId: string, month: string): Promise<void> {
  await api(`/recurring-transactions/${recurringId}/occurrences/${month}`, { method: "DELETE" })
}
