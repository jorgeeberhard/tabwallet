import type { Metadata } from "next"
import { Suspense } from "react"

import { ConfirmEmail } from "@/components/auth/confirm-email"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Skeleton } from "@/components/ui/skeleton"

export const metadata: Metadata = { title: "Confirmar e-mail | TabWallet" }

export default function ConfirmEmailPage({ searchParams }: PageProps<"/confirm-email">) {
  return (
    <Card>
      <CardHeader>
        <h1>
          <CardTitle className="font-display text-2xl font-bold tracking-tight">
            Confirmar e-mail
          </CardTitle>
        </h1>
        <CardDescription>Estamos confirmando o seu cadastro no TabWallet.</CardDescription>
      </CardHeader>
      <CardContent>
        <Suspense fallback={<Skeleton className="h-24 w-full" />}>
          <ConfirmEmailContent searchParams={searchParams} />
        </Suspense>
      </CardContent>
    </Card>
  )
}

/** Reads the token from the link in the e-mail (`?token=...`). */
async function ConfirmEmailContent({
  searchParams,
}: Pick<PageProps<"/confirm-email">, "searchParams">) {
  const { token } = await searchParams
  return <ConfirmEmail token={typeof token === "string" ? token : ""} />
}
