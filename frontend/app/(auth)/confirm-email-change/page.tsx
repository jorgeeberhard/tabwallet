import type { Metadata } from "next"
import { Suspense } from "react"

import { ConfirmEmailChange } from "@/components/auth/confirm-email-change"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Skeleton } from "@/components/ui/skeleton"

export const metadata: Metadata = { title: "Confirmar troca de e-mail | TabWallet" }

export default function ConfirmEmailChangePage({
  searchParams,
}: PageProps<"/confirm-email-change">) {
  return (
    <Card>
      <CardHeader>
        <h1>
          <CardTitle className="font-display text-2xl font-bold tracking-tight">
            Confirmar troca de e-mail
          </CardTitle>
        </h1>
        <CardDescription>Estamos confirmando o novo e-mail da sua conta.</CardDescription>
      </CardHeader>
      <CardContent>
        <Suspense fallback={<Skeleton className="h-24 w-full" />}>
          <ConfirmEmailChangeContent searchParams={searchParams} />
        </Suspense>
      </CardContent>
    </Card>
  )
}

/** Reads the token from the link in the e-mail (`?token=...`). */
async function ConfirmEmailChangeContent({
  searchParams,
}: Pick<PageProps<"/confirm-email-change">, "searchParams">) {
  const { token } = await searchParams
  return <ConfirmEmailChange token={typeof token === "string" ? token : ""} />
}
