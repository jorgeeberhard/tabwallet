"use client"

import { zodResolver } from "@hookform/resolvers/zod"
import { useQueryClient } from "@tanstack/react-query"
import { MailCheckIcon, TriangleAlertIcon } from "lucide-react"
import { useState } from "react"
import { type FieldValues, type Path, type UseFormSetError, useForm } from "react-hook-form"
import { toast } from "sonner"

import { useCurrentUser } from "@/components/user-menu"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import {
  Field,
  FieldDescription,
  FieldError,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field"
import { Input } from "@/components/ui/input"
import { Skeleton } from "@/components/ui/skeleton"
import { Spinner } from "@/components/ui/spinner"
import {
  type PendingEmailChange,
  type ProfileChanges,
  type User,
  resendEmailChange,
  updateProfile,
} from "@/lib/api"
import {
  type ProfileEmailValues,
  type ProfileNameValues,
  type ProfilePasswordValues,
  profileEmailSchema,
  profileNameSchema,
  profilePasswordSchema,
} from "@/lib/auth-schemas"
import { genericFailure, isApiError, messageForCode } from "@/lib/auth-messages"
import { PASSWORD_HINT } from "@/lib/password-rule"

/** Name, e-mail and password, each in its own card and saved on its own. */
export function ProfileForms() {
  const { data: user } = useCurrentUser()

  if (!user) {
    return (
      <div className="flex flex-col gap-6">
        <Skeleton className="h-48 w-full" />
        <Skeleton className="h-64 w-full" />
        <Skeleton className="h-80 w-full" />
      </div>
    )
  }
  return (
    <div className="flex max-w-[640px] flex-col gap-6">
      <NameCard user={user} />
      <EmailCard user={user} />
      <PasswordCard />
    </div>
  )
}

/**
 * Sends the changes and keeps the cached user (shown in the header) in step.
 * Returns the error to show on the form, or `null` when it was saved.
 */
function useSaveProfile() {
  const queryClient = useQueryClient()
  return async function save<T extends FieldValues>(
    changes: ProfileChanges,
    setError: UseFormSetError<T>,
    fieldFor: Record<string, Path<T>>,
    action: string,
  ): Promise<{ saved: true; user: User } | { saved: false; failure: string | null }> {
    try {
      const user = await updateProfile(changes)
      queryClient.setQueryData<User>(["current-user"], user)
      return { saved: true, user }
    } catch (error) {
      if (isApiError(error)) {
        // Errors that belong to one field are shown under it.
        const field = fieldFor[error.code]
        if (field) {
          setError(field, { message: messageForCode(error.code) })
          return { saved: false, failure: null }
        }
        if (error.code === "validation_failed") {
          let placed = false
          for (const [name, rule] of Object.entries(error.fields)) {
            const target = fieldFor[name]
            if (target) {
              setError(target, { message: messageForCode(rule) ?? "Confira este campo." })
              placed = true
            }
          }
          if (placed) return { saved: false, failure: null }
        }
      }
      return { saved: false, failure: genericFailure(action) }
    }
  }
}

function FailureAlert({ message }: { message: string | null }) {
  if (!message) return null
  return (
    <Alert variant="destructive">
      <TriangleAlertIcon strokeWidth={1.75} />
      <AlertDescription>{message}</AlertDescription>
    </Alert>
  )
}

function NameCard({ user }: { user: User }) {
  const save = useSaveProfile()
  const [failure, setFailure] = useState<string | null>(null)
  const {
    register,
    handleSubmit,
    setError,
    formState: { errors, isSubmitting },
  } = useForm<ProfileNameValues>({
    resolver: zodResolver(profileNameSchema),
    defaultValues: { name: user.name },
  })

  async function onSubmit(values: ProfileNameValues) {
    setFailure(null)
    const result = await save({ name: values.name }, setError, { name: "name" }, "salvar o nome")
    if (result.saved) toast.success("Nome salvo.")
    else setFailure(result.failure)
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle className="font-display text-xl font-bold tracking-tight">
          <h2>Dados pessoais</h2>
        </CardTitle>
        <CardDescription>Como você quer ser chamado. Este campo é opcional.</CardDescription>
      </CardHeader>
      <CardContent>
        <form onSubmit={handleSubmit(onSubmit)} noValidate className="flex flex-col gap-6">
          <FailureAlert message={failure} />
          <FieldGroup>
            <Field data-invalid={!!errors.name}>
              <FieldLabel htmlFor="name">Nome</FieldLabel>
              <Input
                id="name"
                autoComplete="name"
                aria-invalid={!!errors.name}
                {...register("name")}
              />
              <FieldError errors={[errors.name]} />
            </Field>
          </FieldGroup>
          <Button type="submit" className="self-start" disabled={isSubmitting}>
            {isSubmitting && <Spinner />}
            Salvar nome
          </Button>
        </form>
      </CardContent>
    </Card>
  )
}

function EmailCard({ user }: { user: User }) {
  const save = useSaveProfile()
  const [failure, setFailure] = useState<string | null>(null)
  const {
    register,
    handleSubmit,
    setError,
    reset,
    formState: { errors, isSubmitting },
  } = useForm<ProfileEmailValues>({
    resolver: zodResolver(profileEmailSchema),
    defaultValues: { email: user.email, currentPassword: "" },
  })

  async function onSubmit(values: ProfileEmailValues) {
    setFailure(null)
    const requested = values.email.trim()
    if (requested.toLowerCase() === user.email.toLowerCase()) {
      setError("email", { message: "Este já é o seu e-mail de acesso. Digite o novo e-mail." })
      return
    }
    const result = await save(
      { email: requested, currentPassword: values.currentPassword },
      setError,
      {
        email: "email",
        current_password: "currentPassword",
        invalid_current_password: "currentPassword",
      },
      "pedir a troca do e-mail",
    )
    if (result.saved) {
      toast.success(`Enviamos um link de confirmação para ${requested}.`)
      // The e-mail does not change until the link is opened: the form goes
      // back to the e-mail that still works.
      reset({ email: result.user.email, currentPassword: "" })
    } else {
      setFailure(result.failure)
    }
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle className="font-display text-xl font-bold tracking-tight">
          <h2>E-mail</h2>
        </CardTitle>
        <CardDescription>
          É o e-mail que você usa para entrar e onde recebe o link de recuperação de senha. Ao
          trocar, enviamos um link de confirmação para o novo e-mail; você continua entrando com o
          atual até confirmar.
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-6">
        {user.pendingEmailChange && (
          <PendingEmailChangeNotice pending={user.pendingEmailChange} currentEmail={user.email} />
        )}
        <form onSubmit={handleSubmit(onSubmit)} noValidate className="flex flex-col gap-6">
          <FailureAlert message={failure} />
          <FieldGroup>
            <Field data-invalid={!!errors.email}>
              <FieldLabel htmlFor="email">E-mail</FieldLabel>
              <Input
                id="email"
                type="email"
                autoComplete="email"
                aria-invalid={!!errors.email}
                {...register("email")}
              />
              <FieldError errors={[errors.email]} />
            </Field>
            <Field data-invalid={!!errors.currentPassword}>
              <FieldLabel htmlFor="email-current-password">Senha atual</FieldLabel>
              <Input
                id="email-current-password"
                type="password"
                autoComplete="current-password"
                aria-invalid={!!errors.currentPassword}
                {...register("currentPassword")}
              />
              {errors.currentPassword ? (
                <FieldError errors={[errors.currentPassword]} />
              ) : (
                <FieldDescription>Pedimos a senha para confirmar que é você.</FieldDescription>
              )}
            </Field>
          </FieldGroup>
          <Button type="submit" className="self-start" disabled={isSubmitting}>
            {isSubmitting && <Spinner />}
            Trocar e-mail
          </Button>
        </form>
      </CardContent>
    </Card>
  )
}

/**
 * Shown while a change of e-mail waits for the link sent to the new address:
 * which address, how it works, and a button to send the link again. A new
 * request in the form below replaces this one.
 */
function PendingEmailChangeNotice({
  pending,
  currentEmail,
}: {
  pending: PendingEmailChange
  currentEmail: string
}) {
  const queryClient = useQueryClient()
  const [state, setState] = useState<"idle" | "sending" | "sent">("idle")
  const [failure, setFailure] = useState<string | null>(null)

  async function resend() {
    setFailure(null)
    setState("sending")
    try {
      await resendEmailChange()
    } catch (error) {
      setState("idle")
      if (isApiError(error) && error.code === "no_pending_email_change") {
        // Confirmed or expired meanwhile: read the profile again.
        void queryClient.invalidateQueries({ queryKey: ["current-user"] })
      }
      setFailure(
        (isApiError(error) && messageForCode(error.code)) || genericFailure("reenviar o link"),
      )
      return
    }
    setState("sent")
  }

  return (
    <div className="flex flex-col gap-3">
      <Alert variant="info">
        <MailCheckIcon strokeWidth={1.75} />
        <AlertTitle>Troca de e-mail pendente</AlertTitle>
        <AlertDescription>
          <p>
            Enviamos um link de confirmação para <strong>{pending.email}</strong>. Abra o link
            (vale por 24 horas e só pode ser usado uma vez) para concluir a troca. Veja também a
            caixa de spam.
          </p>
          <p>
            Até lá, você continua entrando com <strong>{currentEmail}</strong>. Quando a troca for
            concluída, avisamos esse e-mail. Pedir outra troca abaixo substitui esta e invalida o link anterior.
          </p>
        </AlertDescription>
      </Alert>
      <Button
        type="button"
        variant="outline"
        className="self-start"
        disabled={state === "sending"}
        onClick={resend}
      >
        {state === "sending" && <Spinner />}
        Reenviar link
      </Button>
      <div aria-live="polite" className="text-sm">
        {state === "sent" && (
          <p className="text-muted-foreground">
            Enviamos um novo link para {pending.email}. O link anterior deixou de valer.
          </p>
        )}
        {failure && <p className="text-destructive">{failure}</p>}
      </div>
    </div>
  )
}

function PasswordCard() {
  const save = useSaveProfile()
  const [failure, setFailure] = useState<string | null>(null)
  const {
    register,
    handleSubmit,
    setError,
    reset,
    formState: { errors, isSubmitting },
  } = useForm<ProfilePasswordValues>({
    resolver: zodResolver(profilePasswordSchema),
    defaultValues: { currentPassword: "", newPassword: "", confirmation: "" },
  })

  async function onSubmit(values: ProfilePasswordValues) {
    setFailure(null)
    const result = await save(
      { newPassword: values.newPassword, currentPassword: values.currentPassword },
      setError,
      {
        new_password: "newPassword",
        current_password: "currentPassword",
        invalid_current_password: "currentPassword",
      },
      "salvar a senha",
    )
    if (result.saved) {
      toast.success("Senha alterada.")
      reset()
    } else {
      setFailure(result.failure)
    }
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle className="font-display text-xl font-bold tracking-tight">
          <h2>Senha</h2>
        </CardTitle>
        <CardDescription>
          Trocar a senha não encerra as sessões já abertas em outros aparelhos.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <form onSubmit={handleSubmit(onSubmit)} noValidate className="flex flex-col gap-6">
          <FailureAlert message={failure} />
          <FieldGroup>
            <Field data-invalid={!!errors.currentPassword}>
              <FieldLabel htmlFor="password-current-password">Senha atual</FieldLabel>
              <Input
                id="password-current-password"
                type="password"
                autoComplete="current-password"
                aria-invalid={!!errors.currentPassword}
                {...register("currentPassword")}
              />
              <FieldError errors={[errors.currentPassword]} />
            </Field>
            <Field data-invalid={!!errors.newPassword}>
              <FieldLabel htmlFor="new-password">Nova senha</FieldLabel>
              <Input
                id="new-password"
                type="password"
                autoComplete="new-password"
                aria-invalid={!!errors.newPassword}
                {...register("newPassword")}
              />
              {errors.newPassword ? (
                <FieldError errors={[errors.newPassword]} />
              ) : (
                <FieldDescription>{PASSWORD_HINT}</FieldDescription>
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
          <Button type="submit" className="self-start" disabled={isSubmitting}>
            {isSubmitting && <Spinner />}
            Alterar senha
          </Button>
        </form>
      </CardContent>
    </Card>
  )
}
