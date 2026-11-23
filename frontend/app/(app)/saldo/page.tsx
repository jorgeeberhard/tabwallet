import type { Metadata } from "next"

import { BalanceEvolution } from "@/components/balance/balance-evolution"

export const metadata: Metadata = { title: "Evolução do saldo" }

export default function BalanceEvolutionPage() {
  return <BalanceEvolution />
}
