"use client"

import { zodResolver } from "@hookform/resolvers/zod"
import { useState } from "react"
import { useForm } from "react-hook-form"

import { Button } from "@/components/ui/button"
import { Field, FieldError, FieldGroup, FieldLabel } from "@/components/ui/field"
import { Input } from "@/components/ui/input"
import { Spinner } from "@/components/ui/spinner"
import { resendConfirmation } from "@/lib/api"
import { type ResendConfirmationValues, resendConfirmationSchema } from "@/lib/auth-schemas"
import { genericFailure, isApiError, messageForCode } from "@/lib/auth-messages"

/**
 * What the person sees after asking for the e-mail again. The API answers the
 * same for any address, so this never says whether a sign-up exists.
 */
const SENT_MESSAGE =
  "Pronto. Se houver um cadastro aguardando confirmação para este e-mail, enviamos um novo link. O link anterior deixou de valer."

function failureMessage(error: unknown): string {
  return (isApiError(error) && messageForCode(error.code)) || genericFailure("reenviar o e-mail")
}

/**
 * "Reenviar e-mail" for an address the person already typed (after the
 * sign-up, or on the login screen).
 */
export function ResendConfirmationButton({ email }: { email: string }) {
  const [state, setState] = useState<"idle" | "sending" | "sent">("idle")
  const [failure, setFailure] = useState<string | null>(null)

  async function resend() {
    setFailure(null)
    setState("sending")
    try {
      await resendConfirmation(email)
    } catch (error) {
      setFailure(failureMessage(error))
      setState("idle")
      return
    }
    setState("sent")
  }

  return (
    <div className="flex flex-col gap-3">
      <Button
        type="button"
        variant="outline"
        className="w-full"
        disabled={state === "sending"}
        onClick={resend}
      >
        {state === "sending" && <Spinner />}
        Reenviar e-mail
      </Button>
      <div aria-live="polite" className="text-sm">
        {state === "sent" && <p className="text-muted-foreground">{SENT_MESSAGE}</p>}
        {failure && <p className="text-destructive">{failure}</p>}
      </div>
    </div>
  )
}

/**
 * Asks for the e-mail, then sends the link again: for the person who opened an
 * expired or used link and so has no address at hand.
 */
export function ResendConfirmationForm() {
  const [sent, setSent] = useState(false)
  const [failure, setFailure] = useState<string | null>(null)
  const {
    register,
    handleSubmit,
    formState: { errors, isSubmitting },
  } = useForm<ResendConfirmationValues>({
    resolver: zodResolver(resendConfirmationSchema),
    defaultValues: { email: "" },
  })

  async function onSubmit(values: ResendConfirmationValues) {
    setFailure(null)
    setSent(false)
    try {
      await resendConfirmation(values.email)
    } catch (error) {
      setFailure(failureMessage(error))
      return
    }
    setSent(true)
  }

  return (
    <form onSubmit={handleSubmit(onSubmit)} noValidate className="flex flex-col gap-4">
      <FieldGroup>
        <Field data-invalid={!!errors.email}>
          <FieldLabel htmlFor="resend-email">E-mail do cadastro</FieldLabel>
          <Input
            id="resend-email"
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
        Enviar novo link
      </Button>
      <div aria-live="polite" className="text-sm">
        {sent && <p className="text-muted-foreground">{SENT_MESSAGE}</p>}
        {failure && <p className="text-destructive">{failure}</p>}
      </div>
    </form>
  )
}
