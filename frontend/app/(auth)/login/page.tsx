import type { Metadata } from "next"

import { LoginForm } from "@/components/auth/login-form"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"

export const metadata: Metadata = { title: "Entrar | TabWallet" }

export default function LoginPage() {
  return (
    <Card>
      <CardHeader>
        <h1>
          <CardTitle className="font-display text-2xl font-bold tracking-tight">
            Entrar
          </CardTitle>
        </h1>
        <CardDescription>Acesse sua conta para ver seu dinheiro em uma aba.</CardDescription>
      </CardHeader>
      <CardContent>
        <LoginForm />
      </CardContent>
    </Card>
  )
}
