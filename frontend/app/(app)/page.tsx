import { BalanceSection } from "@/components/balance/balance-section"
import { BudgetAlerts } from "@/components/budgets/budget-alerts"
import { Welcome } from "@/components/welcome"

export default function Home() {
  return (
    <section className="flex flex-col gap-6">
      <div className="flex flex-col gap-2">
        <h1 className="font-display text-3xl font-bold tracking-tight">
          Seu dinheiro, em uma aba.
        </h1>
        <Welcome />
      </div>
      <BudgetAlerts />
      <BalanceSection />
    </section>
  )
}
