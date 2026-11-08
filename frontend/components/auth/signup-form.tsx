"use client"

import { zodResolver } from "@hookform/resolvers/zod"
import { TriangleAlertIcon } from "lucide-react"
import Link from "next/link"
import { useRouter } from "next/navigation"
import { useState } from "react"
import { useForm } from "react-hook-form"

import { Alert, AlertDescription } from "@/components/ui/alert"
import { Button } from "@/components/ui/button"
import {
  Field,
  FieldDescription,
  FieldError,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field"
import { Input } from "@/components/ui/input"
import { Spinner } from "@/components/ui/spinner"
import { signUp } from "@/lib/api"
import { type SignUpValues, signUpSchema } from "@/lib/auth-schemas"
import { genericFailure, isApiError, messageForCode } from "@/lib/auth-messages"

export function SignupForm() {
  const router = useRouter()
  const [failure, setFailure] = useState<string | null>(null)
  const {
    register,
    handleSubmit,
    setError,
    formState: { errors, isSubmitting },
  } = useForm<SignUpValues>({
    resolver: zodResolver(signUpSchema),
    defaultValues: { email: "", password: "" },
  })

  async function onSubmit(values: SignUpValues) {
    setFailure(null)
    try {
      await signUp(values.email, values.password)
    } catch (error) {
      if (isApiError(error)) {
        if (error.code === "email_taken") {
          setError("email", { message: messageForCode(error.code) })
          return
        }
        // Rules the form should have caught itself, e.g. after a rule change.
        const fieldMessages = Object.entries(error.fields)
        if (error.code === "validation_failed" && fieldMessages.length > 0) {
          for (const [field, rule] of fieldMessages) {
            if (field === "email" || field === "password") {
              setError(field, { message: messageForCode(rule) ?? "Confira este campo." })
            }
          }
          return
        }
      }
      setFailure(genericFailure("criar a conta"))
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
          <FieldLabel htmlFor="password">Senha</FieldLabel>
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
      </FieldGroup>
      <Button type="submit" size="lg" className="w-full" disabled={isSubmitting}>
        {isSubmitting && <Spinner />}
        Criar conta
      </Button>
      <p className="text-center text-sm text-muted-foreground">
        Já tem conta?{" "}
        <Link href="/login" className="font-medium text-primary underline-offset-4 hover:underline">
          Entrar
        </Link>
      </p>
    </form>
  )
}
