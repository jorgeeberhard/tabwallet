"use client"

import Link from "next/link"

import { Skeleton } from "@/components/ui/skeleton"
import { useCurrentUser } from "@/components/user-menu"

/** Greeting of the authenticated home, above the balance. */
export function Welcome() {
  const { data: user } = useCurrentUser()

  return (
    <div className="flex flex-col gap-1">
      {user ? (
        <p>
          Você entrou como <strong className="font-semibold">{user.name || user.email}</strong>.{" "}
          <Link
            href="/profile"
            className="font-medium text-primary underline-offset-4 hover:underline"
          >
            Editar perfil
          </Link>
        </p>
      ) : (
        <Skeleton className="h-6 w-64" />
      )}
      <p className="max-w-prose text-muted-foreground">
        Em breve aqui: pagamento de faturas, saldo dos próximos meses e orçamentos.
      </p>
    </div>
  )
}
