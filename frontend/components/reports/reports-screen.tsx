"use client"

import { BalanceEvolution } from "@/components/balance/balance-evolution"
import { ExpensesByCategory } from "@/components/reports/expenses-by-category"
import { IncomeVsExpenses } from "@/components/reports/income-vs-expenses"

/**
 * UC11: where the money goes and how the balance evolves. Three views, each
 * with its own period: expenses by Category (default: this month), income x
 * expenses per month, and the balance history already served by the balance
 * evolution view.
 */
export function ReportsScreen() {
  return (
    <div className="flex flex-col gap-10">
      <div className="flex flex-col gap-1">
        <h1 className="font-display text-3xl font-bold tracking-tight">Relatórios</h1>
        <p className="max-w-prose text-muted-foreground">
          Para onde o dinheiro vai e como o saldo evolui. Os valores contam no mês de referência:
          o da própria data, ou o do vencimento da fatura para o que está no cartão.
        </p>
      </div>
      <ExpensesByCategory />
      <IncomeVsExpenses />
      <BalanceEvolution embedded />
    </div>
  )
}
