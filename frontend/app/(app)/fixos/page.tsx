import type { Metadata } from "next"

import { RecurringScreen } from "@/components/recurring/recurring-screen"

export const metadata: Metadata = { title: "Gastos e receitas fixos" }

export default function RecurringPage() {
  return <RecurringScreen />
}
