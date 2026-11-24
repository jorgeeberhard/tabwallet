"use client"

import { CircleCheckIcon, TriangleAlertIcon } from "lucide-react"
import Link from "next/link"
import { useEffect, useRef, useState } from "react"

import { ResendConfirmationForm } from "@/components/auth/resend-confirmation"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Button, buttonVariants } from "@/components/ui/button"
import { Spinner } from "@/components/ui/spinner"
import { confirmSignUp } from "@/lib/api"
import { genericFailure, isApiError } from "@/lib/auth-messages"

type State =
  | { status: "confirming" }
  | { status: "done" }
  /** Unknown, expired, used or replaced link (the API does not tell them apart). */
  | { status: "invalid" }
  /** The e-mail became the account of someone else meanwhile. */
  | { status: "taken" }
  | { status: "failed"; message: string }

/**
 * The page the link in the confirmation e-mail opens. It confirms on load: the
 * account is created, but nobody is signed in, so the person goes to the login
 * with the password chosen at sign-up. `token` is empty when the link lost it.
 */
export function ConfirmEmail({ token }: { token: string }) {
  const [state, setState] = useState<State>(
    token === "" ? { status: "invalid" } : { status: "confirming" },
  )
  // The link works once, and React runs effects twice in development: only
  // the first run may spend it.
  const started = useRef(false)

  async function confirm() {
    setState({ status: "confirming" })
    try {
      await confirmSignUp(token)
    } catch (error) {
      if (isApiError(error) && error.code === "invalid_confirmation_token") {
        setState({ status: "invalid" })
      } else if (isApiError(error) && error.code === "email_taken") {
        setState({ status: "taken" })
      } else {
        setState({ status: "failed", message: genericFailure("confirmar o e-mail") })
      }
      return
    }
    setState({ status: "done" })
  }

  useEffect(() => {
    if (token === "" || started.current) return
    started.current = true
    void confirm()
    // eslint-disable-next-line react-hooks/exhaustive-deps -- runs once per token
  }, [token])

  if (state.status === "confirming") {
    return (
      <div aria-live="polite" className="flex items-center gap-3 text-sm text-muted-foreground">
        <Spinner />
        Confirmando seu e-mail…
      </div>
    )
  }

  if (state.status === "done") {
    return (
      <div className="flex flex-col gap-6">
        <Alert variant="success">
          <CircleCheckIcon strokeWidth={1.75} />
          <AlertTitle>E-mail confirmado</AlertTitle>
          <AlertDescription>
            Sua conta foi criada. Entre com o e-mail e a senha que você escolheu no cadastro.
          </AlertDescription>
        </Alert>
        <Link href="/login" className={buttonVariants({ size: "lg", className: "w-full" })}>
          Entrar
        </Link>
      </div>
    )
  }

  if (state.status === "taken") {
    return (
      <div className="flex flex-col gap-6">
        <Alert variant="warning">
          <TriangleAlertIcon strokeWidth={1.75} />
          <AlertTitle>Este e-mail já tem uma conta</AlertTitle>
          <AlertDescription>
            Nesse meio-tempo, uma conta foi criada com este e-mail, então não foi possível
            confirmar este cadastro. Se a conta é sua, entre ou redefina a senha.
          </AlertDescription>
        </Alert>
        <Link href="/login" className={buttonVariants({ size: "lg", className: "w-full" })}>
          Entrar
        </Link>
        <p className="text-center text-sm text-muted-foreground">
          <Link
            href="/forgot-password"
            className="font-medium text-primary underline-offset-4 hover:underline"
          >
            Esqueci a senha
          </Link>
        </p>
      </div>
    )
  }

  if (state.status === "failed") {
    return (
      <div className="flex flex-col gap-6">
        <Alert variant="destructive">
          <TriangleAlertIcon strokeWidth={1.75} />
          <AlertDescription>{state.message}</AlertDescription>
        </Alert>
        <Button size="lg" className="w-full" onClick={confirm}>
          Tentar de novo
        </Button>
      </div>
    )
  }

  return (
    <div className="flex flex-col gap-6">
      <Alert variant="warning">
        <TriangleAlertIcon strokeWidth={1.75} />
        <AlertTitle>Este link não vale mais</AlertTitle>
        <AlertDescription>
          O link de confirmação é inválido, expirou, já foi usado ou foi trocado por um mais
          recente. Informe o e-mail do cadastro para receber outro. Se já se passaram 24 horas
          desde o cadastro, ele deixou de valer: faça o cadastro de novo.
        </AlertDescription>
      </Alert>
      <ResendConfirmationForm />
      <p className="text-center text-sm text-muted-foreground">
        <Link href="/signup" className="font-medium text-primary underline-offset-4 hover:underline">
          Fazer o cadastro de novo
        </Link>
        {" · "}
        <Link href="/login" className="font-medium text-primary underline-offset-4 hover:underline">
          Entrar
        </Link>
      </p>
    </div>
  )
}
