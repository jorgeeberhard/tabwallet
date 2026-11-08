import type { Metadata } from "next"
import Link from "next/link"
import { Suspense } from "react"

import { ResetPasswordForm } from "@/components/auth/reset-password-form"
import { buttonVariants } from "@/components/ui/button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Skeleton } from "@/components/ui/skeleton"

export const metadata: Metadata = { title: "Redefinir senha | TabWallet" }

export default function ResetPasswordPage({ searchParams }: PageProps<"/reset-password">) {
  return (
    <Card>
      <CardHeader>
        <h1>
          <CardTitle className="font-display text-2xl font-bold tracking-tight">
            Redefinir senha
          </CardTitle>
        </h1>
        <CardDescription>Escolha uma nova senha para a sua conta.</CardDescription>
      </CardHeader>
      <CardContent>
        <Suspense fallback={<Skeleton className="h-48 w-full" />}>
          <ResetPasswordContent searchParams={searchParams} />
        </Suspense>
      </CardContent>
    </Card>
  )
}

/** Reads the token from the link in the e-mail (`?token=...`). */
async function ResetPasswordContent({
  searchParams,
}: Pick<PageProps<"/reset-password">, "searchParams">) {
  const { token } = await searchParams
  if (typeof token !== "string" || token === "") {
    return (
      <div className="flex flex-col gap-4">
        <p className="text-sm text-muted-foreground">
          Este link está incompleto. Peça um novo link para redefinir a senha.
        </p>
        <Link
          href="/forgot-password"
          className={buttonVariants({ size: "lg", className: "w-full" })}
        >
          Pedir novo link
        </Link>
      </div>
    )
  }
  return <ResetPasswordForm token={token} />
}
