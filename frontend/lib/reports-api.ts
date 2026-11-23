import { centsToReais } from "@/lib/amount"
import { api } from "@/lib/api"
import { BALANCE_KEY } from "@/lib/balance-api"

/**
 * Query key prefix of the reports. It sits under `BALANCE_KEY`, so whatever
 * refreshes the balance (new Transactions, invoice payments) refreshes them.
 */
export const REPORTS_KEY = [...BALANCE_KEY, "reports"] as const

/** What one Category spent in the period, by Reference Month. */
export interface CategoryExpense {
  categoryId: string
  name: string
  /** A deactivated Category stays in the report of the months it was used. */
  active: boolean
  totalCents: number
  /** In reais, for the Design System components and the chart. */
  total: number
  /** The part of the total that is a Projection (a date after today). */
  projectedCents: number
  projected: number
}

export interface ExpensesByCategory {
  /** First and last month of the period, `YYYY-MM`. */
  from: string
  to: string
  totalCents: number
  total: number
  projectedCents: number
  projected: number
  /** Biggest first. */
  categories: CategoryExpense[]
}

/** Income and expenses of one Reference Month. */
export interface MonthTotals {
  /** `YYYY-MM`. */
  month: string
  incomeCents: number
  income: number
  expenseCents: number
  expense: number
  /** Income less expenses; may be negative. */
  balanceCents: number
  balance: number
  projectedIncomeCents: number
  projectedExpenseCents: number
}

export interface IncomeVsExpenses {
  from: string
  to: string
  incomeCents: number
  expenseCents: number
  /** One entry per month of the period, oldest first, empty months included. */
  months: MonthTotals[]
}

interface CategoryPayload {
  category_id: string
  name: string
  active: boolean
  total_cents: number
  projected_cents: number
}

interface ExpensesByCategoryPayload {
  from: string
  to: string
  total_cents: number
  projected_cents: number
  categories: CategoryPayload[]
}

interface MonthPayload {
  month: string
  income_cents: number
  expense_cents: number
  balance_cents: number
  projected_income_cents: number
  projected_expense_cents: number
}

interface IncomeVsExpensesPayload {
  from: string
  to: string
  income_cents: number
  expense_cents: number
  months: MonthPayload[]
}

export interface PeriodParams {
  /** First month, `YYYY-MM`. */
  from: string
  /** Last month, `YYYY-MM`. */
  to: string
}

export async function getExpensesByCategory({
  from,
  to,
}: PeriodParams): Promise<ExpensesByCategory> {
  const payload = await api<ExpensesByCategoryPayload>(
    `/reports/expenses-by-category?from=${from}&to=${to}`,
  )
  return {
    from: payload.from,
    to: payload.to,
    totalCents: payload.total_cents,
    total: centsToReais(payload.total_cents),
    projectedCents: payload.projected_cents,
    projected: centsToReais(payload.projected_cents),
    categories: payload.categories.map((category) => ({
      categoryId: category.category_id,
      name: category.name,
      active: category.active,
      totalCents: category.total_cents,
      total: centsToReais(category.total_cents),
      projectedCents: category.projected_cents,
      projected: centsToReais(category.projected_cents),
    })),
  }
}

export async function getIncomeVsExpenses({ from, to }: PeriodParams): Promise<IncomeVsExpenses> {
  const payload = await api<IncomeVsExpensesPayload>(
    `/reports/income-vs-expenses?from=${from}&to=${to}`,
  )
  return {
    from: payload.from,
    to: payload.to,
    incomeCents: payload.income_cents,
    expenseCents: payload.expense_cents,
    months: payload.months.map((month) => ({
      month: month.month,
      incomeCents: month.income_cents,
      income: centsToReais(month.income_cents),
      expenseCents: month.expense_cents,
      expense: centsToReais(month.expense_cents),
      balanceCents: month.balance_cents,
      balance: centsToReais(month.balance_cents),
      projectedIncomeCents: month.projected_income_cents,
      projectedExpenseCents: month.projected_expense_cents,
    })),
  }
}
