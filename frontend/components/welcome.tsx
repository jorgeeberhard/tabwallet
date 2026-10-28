"use client"

import { Skeleton } from "@/components/ui/skeleton"
import { useCurrentUser } from "@/components/user-menu"

/** Placeholder of the authenticated home until balance and transactions arrive. */
export function Welcome() {
  const { data: user } = useCurrentUser()

  return (
    <div className="flex flex-col gap-1">
      {user ? (
        <p>
          Você entrou como <strong className="font-semibold">{user.email}</strong>.
        </p>
      ) : (
        <Skeleton className="h-6 w-64" />
      )}
      <p className="max-w-prose text-muted-foreground">
        Em breve aqui: saldo, transações e faturas de cartão.
      </p>
    </div>
  )
}
