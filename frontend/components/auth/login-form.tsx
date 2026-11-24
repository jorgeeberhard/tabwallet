"use client"

import { zodResolver } from "@hookform/resolvers/zod"
import { MailWarningIcon, TriangleAlertIcon } from "lucide-react"
import Link from "next/link"
import { useRouter } from "next/navigation"
import { useState } from "react"
import { useForm } from "react-hook-form"

import { ResendConfirmationButton } from "@/components/auth/resend-confirmation"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Button } from "@/components/ui/button"
import { Field, FieldError, FieldGroup, FieldLabel } from "@/components/ui/field"
import { Input } from "@/components/ui/input"
import { Spinner } from "@/components/ui/spinner"
import { logIn } from "@/lib/api"
import { type LogInValues, logInSchema } from "@/lib/auth-schemas"
import { genericFailure, isApiError, messageForCode } from "@/lib/auth-messages"

export function LoginForm() {
  const router = useRouter()
  const [failure, setFailure] = useState<string | null>(null)
  // Set when the e-mail and password match a sign-up still waiting for the
  // confirmation link: the person is told so and can ask for the e-mail again.
  const [unconfirmedEmail, setUnconfirmedEmail] = useState<string | null>(null)
  const {
    register,
    handleSubmit,
    formState: { errors, isSubmitting },
  } = useForm<LogInValues>({
    resolver: zodResolver(logInSchema),
    defaultValues: { email: "", password: "" },
  })

  async function onSubmit(values: LogInValues) {
    setFailure(null)
    setUnconfirmedEmail(null)
    try {
      await logIn(values.email, values.password)
    } catch (error) {
      if (isApiError(error) && error.code === "email_not_confirmed") {
        setUnconfirmedEmail(values.email)
        return
      }
      setFailure(
        (isApiError(error) && messageForCode(error.code)) || genericFailure("entrar"),
      )
      return
    }
    router.replace("/")
    router.refresh()
  }

  return (
    <form onSubmit={handleSubmit(onSubmit)} noValidate className="flex flex-col gap-6">
      {failure && (
        <Alert variant="destructive">
          <TriangleAlertIcon strokeWidth={1.75} />
          <AlertDescription>{failure}</AlertDescription>
        </Alert>
      )}
      {unconfirmedEmail && (
        <div className="flex flex-col gap-3">
          <Alert variant="warning">
            <MailWarningIcon strokeWidth={1.75} />
            <AlertTitle>Confirme seu e-mail para entrar</AlertTitle>
            <AlertDescription>
              Seu cadastro ainda aguarda a confirmação. Abra o link que enviamos para{" "}
              {unconfirmedEmail}. Se o e-mail não chegou, peça outro.
            </AlertDescription>
          </Alert>
          <ResendConfirmationButton email={unconfirmedEmail} />
        </div>
      )}
      <FieldGroup>
        <Field data-invalid={!!errors.email}>
          <FieldLabel htmlFor="email">E-mail</FieldLabel>
          <Input
            id="email"
            type="email"
            autoComplete="email"
            placeholder="nome@dominio.com"
            aria-invalid={!!errors.email}
            {...register("email")}
          />
          <FieldError errors={[errors.email]} />
        </Field>
        <Field data-invalid={!!errors.password}>
          <div className="flex items-center justify-between gap-2">
            <FieldLabel htmlFor="password">Senha</FieldLabel>
            <Link
              href="/forgot-password"
              className="text-sm font-medium text-primary underline-offset-4 hover:underline"
            >
              Esqueci a senha
            </Link>
          </div>
          <Input
            id="password"
            type="password"
            autoComplete="current-password"
            aria-invalid={!!errors.password}
            {...register("password")}
          />
          <FieldError errors={[errors.password]} />
        </Field>
      </FieldGroup>
      <Button type="submit" size="lg" className="w-full" disabled={isSubmitting}>
        {isSubmitting && <Spinner />}
        Entrar
      </Button>
      <p className="text-center text-sm text-muted-foreground">
        Ainda não tem conta?{" "}
        <Link href="/signup" className="font-medium text-primary underline-offset-4 hover:underline">
          Criar conta
        </Link>
      </p>
    </form>
  )
}
