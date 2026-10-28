import type { Metadata } from "next"
import { Suspense } from "react"

import { CardInvoicesScreen } from "@/components/credit-cards/card-invoices-screen"
import { Skeleton } from "@/components/ui/skeleton"

export const metadata: Metadata = { title: "Faturas do cartão" }

/** Reads the route param inside a Suspense boundary, so the shell can be prerendered. */
async function CardInvoices({ params }: Pick<PageProps<"/cartoes/[id]">, "params">) {
  const { id } = await params
  return <CardInvoicesScreen cardId={id} />
}

export default function CardInvoicesPage({ params }: PageProps<"/cartoes/[id]">) {
  return (
    <Suspense fallback={<Skeleton className="h-40 w-full max-w-xl" />}>
      <CardInvoices params={params} />
    </Suspense>
  )
}
