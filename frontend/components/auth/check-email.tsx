import { MailCheckIcon } from "lucide-react"
import Link from "next/link"

import { ResendConfirmationButton } from "@/components/auth/resend-confirmation"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"

/**
 * Shown after the sign-up request. The text is the same whether the e-mail is
 * new, already waiting for confirmation or already has an account (the API
 * does not tell), so it never promises that an account was created.
 */
export function CheckEmail({ email }: { email: string }) {
  return (
    <div className="flex flex-col gap-6">
      <Alert>
        <MailCheckIcon strokeWidth={1.75} />
        <AlertTitle>Confira seu e-mail</AlertTitle>
        <AlertDescription>
          Enviamos um e-mail de confirmação para {email}. Abra o link dele para criar a conta; o
          link vale por 24 horas e só pode ser usado uma vez. Veja também a caixa de spam.
        </AlertDescription>
      </Alert>
      <ResendConfirmationButton email={email} />
      <p className="text-center text-sm text-muted-foreground">
        <Link href="/login" className="font-medium text-primary underline-offset-4 hover:underline">
          Voltar para entrar
        </Link>
      </p>
    </div>
  )
}
