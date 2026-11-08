import type { Metadata } from "next"

import { SignupForm } from "@/components/auth/signup-form"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"

export const metadata: Metadata = { title: "Criar conta | TabWallet" }

export default function SignupPage() {
  return (
    <Card>
      <CardHeader>
        <h1>
          <CardTitle className="font-display text-2xl font-bold tracking-tight">
            Criar conta
          </CardTitle>
        </h1>
        <CardDescription>
          Só e-mail e senha. Seus dados ficam separados dos de outras pessoas.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <SignupForm />
      </CardContent>
    </Card>
  )
}
