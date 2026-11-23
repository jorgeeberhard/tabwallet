import type { Metadata } from "next"

import { ReportsScreen } from "@/components/reports/reports-screen"

export const metadata: Metadata = { title: "Relatórios" }

export default function ReportsPage() {
  return <ReportsScreen />
}
