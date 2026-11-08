"use client"

import { zodResolver } from "@hookform/resolvers/zod"
import { MailCheckIcon, TriangleAlertIcon } from "lucide-react"
import Link from "next/link"
import { useState } from "react"
import { useForm } from "react-hook-form"

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Button } from "@/components/ui/button"
import { Field, FieldError, FieldGroup, FieldLabel } from "@/components/ui/field"
import { Input } from "@/components/ui/input"
import { Spinner } from "@/components/ui/spinner"
import { requestPasswordReset } from "@/lib/api"
import { type ForgotPasswordValues, forgotPasswordSchema } from "@/lib/auth-schemas"
import { genericFailure, isApiError, messageForCode } from "@/lib/auth-messages"

/**
 * Asks for the reset link. The answer is the same whether or not the address
 * has an account (the API does not reveal it), so the text says "se existir".
 */
export function ForgotPasswordForm() {
  const [sentTo, setSentTo] = useState<string | null>(null)
  const [failure, setFailure] = useState<string | null>(null)
  const {
    register,
    handleSubmit,
    formState: { errors, isSubmitting },
  } = useForm<ForgotPasswordValues>({
    resolver: zodResolver(forgotPasswordSchema),
    defaultValues: { email: "" },
  })

  async function onSubmit(values: ForgotPasswordValues) {
    setFailure(null)
    try {
      await requestPasswordReset(values.email)
    } catch (error) {
      setFailure(
        (isApiError(error) && messageForCode(error.code)) ||
          genericFailure("enviar o link"),
      )
      return
    }
    setSentTo(values.email)
  }

  if (sentTo) {
    return (
      <div className="flex flex-col gap-6">
        <Alert>
          <MailCheckIcon strokeWidth={1.75} />
          <AlertTitle>Confira seu e-mail</AlertTitle>
          <AlertDescription>
            Se existir uma conta com {sentTo}, enviamos um link para criar uma nova senha. Ele
            vale por 60 minutos e só pode ser usado uma vez.
          </AlertDescription>
        </Alert>
        <p className="text-center text-sm text-muted-foreground">
          <Link
            href="/login"
            className="font-medium text-primary underline-offset-4 hover:underline"
          >
            Voltar para entrar
          </Link>
        </p>
      </div>
    )
  }

  return (
    <form onSubmit={handleSubmit(onSubmit)} noValidate className="flex flex-col gap-6">
      {failure && (
        <Alert variant="destructive">
          <TriangleAlertIcon strokeWidth={1.75} />
          <AlertDescription>{failure}</AlertDescription>
        </Alert>
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
      </FieldGroup>
      <Button type="submit" size="lg" className="w-full" disabled={isSubmitting}>
        {isSubmitting && <Spinner />}
        Enviar link
      </Button>
      <p className="text-center text-sm text-muted-foreground">
        Lembrou a senha?{" "}
        <Link href="/login" className="font-medium text-primary underline-offset-4 hover:underline">
          Entrar
        </Link>
      </p>
    </form>
  )
}
