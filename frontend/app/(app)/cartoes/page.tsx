import type { Metadata } from "next"

import { CreditCardsScreen } from "@/components/credit-cards/credit-cards-screen"

export const metadata: Metadata = { title: "Cartões" }

export default function CreditCardsPage() {
  return <CreditCardsScreen />
}
