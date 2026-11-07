import { api } from "@/lib/api"
import { centsToReais } from "@/lib/amount"

export interface BudgetCategory {
  id: string
  name: string
  /** A deactivated category keeps its budget, which then produces no alert. */
  active: boolean
}

interface BudgetCategoryPayload {
  id: string
  name: string
  type: "expense"
  active: boolean
}

function toCategory(payload: BudgetCategoryPayload): BudgetCategory {
  return { id: payload.id, name: payload.name, active: payload.active }
}

/** A Budget in force: the limit of an expense category, every month until changed. */
export interface Budget {
  category: BudgetCategory
  limitCents: number
  /** Limit in reais, for the Design System components. */
  limit: number
  /** `YYYY-MM`: the month the limit started in. */
  effectiveFrom: string
}

interface BudgetPayload {
  category: BudgetCategoryPayload
  limit_cents: number
  effective_from: string
}

function toBudget(payload: BudgetPayload): Budget {
  return {
    category: toCategory(payload.category),
    limitCents: payload.limit_cents,
    limit: centsToReais(payload.limit_cents),
    effectiveFrom: payload.effective_from,
  }
}

/** One category against its budget in a reference month. */
export interface BudgetStatusLine {
  category: BudgetCategory
  limitCents: number
  limit: number
  /** Expenses of the month, projections included. */
  spentCents: number
  spent: number
  /** The limit less the spending; negative once the limit is exceeded. */
  remainingCents: number
  /** Whole percent of the limit, rounded down (it passes 100 when exceeded). */
  percentUsed: number
  /** The budget alert. Never true for a deactivated category. */
  exceeded: boolean
}

export interface BudgetStatus {
  /** `YYYY-MM`. */
  month: string
  /** How many categories have an alert in the month. */
  exceededCount: number
  lines: BudgetStatusLine[]
}

interface BudgetStatusPayload {
  month: string
  exceeded_count: number
  budgets: {
    category: BudgetCategoryPayload
    limit_cents: number
    spent_cents: number
    remaining_cents: number
    percent_used: number
    exceeded: boolean
  }[]
}

function toStatus(payload: BudgetStatusPayload): BudgetStatus {
  return {
    month: payload.month,
    exceededCount: payload.exceeded_count,
    lines: payload.budgets.map((line) => ({
      category: toCategory(line.category),
      limitCents: line.limit_cents,
      limit: centsToReais(line.limit_cents),
      spentCents: line.spent_cents,
      spent: centsToReais(line.spent_cents),
      remainingCents: line.remaining_cents,
      percentUsed: line.percent_used,
      exceeded: line.exceeded,
    })),
  }
}

/** Query key of everything about budgets; a mutation invalidates this prefix. */
export const BUDGETS_KEY = ["budgets"] as const

/** Query key of the status of one month (`null`: the current month). */
export function budgetStatusKey(month: string | null) {
  return [...BUDGETS_KEY, "status", month ?? "current"] as const
}

/** The budgets in force this month, deactivated categories included. */
export async function listBudgets(): Promise<Budget[]> {
  const { budgets } = await api<{ budgets: BudgetPayload[] }>("/budgets")
  return budgets.map(toBudget)
}

/**
 * Where each category with a budget stands in a reference month (`YYYY-MM`),
 * or in the current month when `month` is `null`. The API calculates the alert
 * on every read, so this is never cached for long.
 */
export async function getBudgetStatus(month: string | null): Promise<BudgetStatus> {
  const query = month === null ? "" : `?month=${month}`
  return toStatus(await api<BudgetStatusPayload>(`/budgets/status${query}`))
}

/** Defines or changes the limit of a category, from the current month on. */
export async function setBudget(categoryId: string, limitCents: number): Promise<Budget> {
  const { budget } = await api<{ budget: BudgetPayload }>(`/budgets/${categoryId}`, {
    method: "PUT",
    body: { limit_cents: limitCents },
  })
  return toBudget(budget)
}

/** Removes the budget of a category from the current month on. */
export async function removeBudget(categoryId: string): Promise<void> {
  await api(`/budgets/${categoryId}`, { method: "DELETE" })
}
