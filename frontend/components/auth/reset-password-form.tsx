"use client"

import { zodResolver } from "@hookform/resolvers/zod"
import { CircleCheckIcon, TriangleAlertIcon } from "lucide-react"
import Link from "next/link"
import { useState } from "react"
import { useForm } from "react-hook-form"

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Button, buttonVariants } from "@/components/ui/button"
import {
  Field,
  FieldDescription,
  FieldError,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field"
import { Input } from "@/components/ui/input"
import { Spinner } from "@/components/ui/spinner"
import { confirmPasswordReset } from "@/lib/api"
import { type ResetPasswordValues, resetPasswordSchema } from "@/lib/auth-schemas"
import { genericFailure, isApiError, messageForCode } from "@/lib/auth-messages"

/** Sets the new password with the token from the e-mailed link. */
export function ResetPasswordForm({ token }: { token: string }) {
  const [done, setDone] = useState(false)
  const [failure, setFailure] = useState<string | null>(null)
  const {
    register,
    handleSubmit,
    setError,
    formState: { errors, isSubmitting },
  } = useForm<ResetPasswordValues>({
    resolver: zodResolver(resetPasswordSchema),
    defaultValues: { password: "", confirmation: "" },
  })

  async function onSubmit(values: ResetPasswordValues) {
    setFailure(null)
    try {
      await confirmPasswordReset(token, values.password)
    } catch (error) {
      if (isApiError(error)) {
        const rule = error.fields.password
        if (error.code === "validation_failed" && rule) {
          setError("password", { message: messageForCode(rule) ?? "Confira este campo." })
          return
        }
        setFailure(messageForCode(error.code) ?? genericFailure("redefinir a senha"))
        return
      }
      setFailure(genericFailure("redefinir a senha"))
      return
    }
    setDone(true)
  }

  if (done) {
    return (
      <div className="flex flex-col gap-6">
        <Alert>
          <CircleCheckIcon strokeWidth={1.75} />
          <AlertTitle>Senha redefinida</AlertTitle>
          <AlertDescription>Agora você já pode entrar com a nova senha.</AlertDescription>
        </Alert>
        <Link href="/login" className={buttonVariants({ size: "lg", className: "w-full" })}>
          Entrar
        </Link>
      </div>
    )
  }

  return (
    <form onSubmit={handleSubmit(onSubmit)} noValidate className="flex flex-col gap-6">
      {failure && (
        <Alert variant="destructive">
          <TriangleAlertIcon strokeWidth={1.75} />
          <AlertDescription>
            {failure}{" "}
            <Link
              href="/forgot-password"
              className="font-medium underline underline-offset-4"
            >
              Pedir novo link
            </Link>
          </AlertDescription>
        </Alert>
      )}
      <FieldGroup>
        <Field data-invalid={!!errors.password}>
          <FieldLabel htmlFor="password">Nova senha</FieldLabel>
          <Input
            id="password"
            type="password"
            autoComplete="new-password"
            aria-invalid={!!errors.password}
            {...register("password")}
          />
          {errors.password ? (
            <FieldError errors={[errors.password]} />
          ) : (
            <FieldDescription>Use pelo menos 8 caracteres.</FieldDescription>
          )}
        </Field>
        <Field data-invalid={!!errors.confirmation}>
          <FieldLabel htmlFor="confirmation">Repita a nova senha</FieldLabel>
          <Input
            id="confirmation"
            type="password"
            autoComplete="new-password"
            aria-invalid={!!errors.confirmation}
            {...register("confirmation")}
          />
          <FieldError errors={[errors.confirmation]} />
        </Field>
      </FieldGroup>
      <Button type="submit" size="lg" className="w-full" disabled={isSubmitting}>
        {isSubmitting && <Spinner />}
        Redefinir senha
      </Button>
    </form>
  )
}
