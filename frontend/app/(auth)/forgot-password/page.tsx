import type { Metadata } from "next"

import { ForgotPasswordForm } from "@/components/auth/forgot-password-form"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"

export const metadata: Metadata = { title: "Esqueci a senha | TabWallet" }

export default function ForgotPasswordPage() {
  return (
    <Card>
      <CardHeader>
        <h1>
          <CardTitle className="font-display text-2xl font-bold tracking-tight">
            Esqueci a senha
          </CardTitle>
        </h1>
        <CardDescription>
          Informe o e-mail da sua conta e enviaremos um link para criar uma nova senha.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <ForgotPasswordForm />
      </CardContent>
    </Card>
  )
}
