"use client"

import { useQueryClient } from "@tanstack/react-query"
import { CircleCheckIcon, TriangleAlertIcon } from "lucide-react"
import Link from "next/link"
import { useEffect, useRef, useState } from "react"

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Button, buttonVariants } from "@/components/ui/button"
import { Spinner } from "@/components/ui/spinner"
import { confirmEmailChange } from "@/lib/api"
import { genericFailure, isApiError } from "@/lib/auth-messages"

type State =
  | { status: "confirming" }
  | { status: "done" }
  /** Unknown, expired, used or replaced link (the API does not tell them apart). */
  | { status: "invalid" }
  /** The new e-mail became the account of someone else meanwhile. */
  | { status: "taken" }
  | { status: "failed"; message: string }

/**
 * The page the link in the e-mail-change message opens. It confirms on load:
 * the e-mail of the account becomes the new one. Nobody is signed in by it, and
 * a session that is already open stays open. `token` is empty when the link
 * lost it.
 */
export function ConfirmEmailChange({ token }: { token: string }) {
  const queryClient = useQueryClient()
  const [state, setState] = useState<State>(
    token === "" ? { status: "invalid" } : { status: "confirming" },
  )
  // The link works once, and React runs effects twice in development: only
  // the first run may spend it.
  const started = useRef(false)

  async function confirm() {
    setState({ status: "confirming" })
    try {
      await confirmEmailChange(token)
    } catch (error) {
      if (isApiError(error) && error.code === "invalid_confirmation_token") {
        setState({ status: "invalid" })
      } else if (isApiError(error) && error.code === "email_taken") {
        setState({ status: "taken" })
      } else {
        setState({ status: "failed", message: genericFailure("confirmar a troca de e-mail") })
      }
      return
    }
    // The header and the profile of an open session show the old e-mail.
    void queryClient.invalidateQueries({ queryKey: ["current-user"] })
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
        Confirmando o novo e-mail…
      </div>
    )
  }

  if (state.status === "done") {
    return (
      <div className="flex flex-col gap-6">
        <Alert variant="success">
          <CircleCheckIcon strokeWidth={1.75} />
          <AlertTitle>E-mail alterado</AlertTitle>
          <AlertDescription>
            A troca foi concluída. Daqui em diante, entre com o novo e-mail e a mesma senha.
            Enviamos um aviso ao e-mail antigo.
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
            Nesse meio-tempo, uma conta foi criada com o novo e-mail, então a troca não foi feita.
            Você continua entrando com o e-mail de antes. Se quiser, peça a troca para outro
            endereço.
          </AlertDescription>
        </Alert>
        <Link href="/profile" className={buttonVariants({ size: "lg", className: "w-full" })}>
          Ir para o perfil
        </Link>
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
          O link da troca de e-mail é inválido, expirou, já foi usado ou foi trocado por um mais
          recente. Entre no seu perfil: se a troca ainda estiver pendente, você pode reenviar o
          link; se não, peça a troca de novo.
        </AlertDescription>
      </Alert>
      <Link href="/profile" className={buttonVariants({ size: "lg", className: "w-full" })}>
        Ir para o perfil
      </Link>
    </div>
  )
}
