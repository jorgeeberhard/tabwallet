"use client"

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { CreditCardIcon, PencilIcon, PlusIcon, TriangleAlertIcon } from "lucide-react"
import Link from "next/link"
import { useState } from "react"
import { toast } from "sonner"

import {
  CreditCardDialog,
  type CreditCardDialogMode,
} from "@/components/credit-cards/credit-card-dialog"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card } from "@/components/ui/card"
import { Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle } from "@/components/ui/empty"
import { Skeleton } from "@/components/ui/skeleton"
import {
  CREDIT_CARDS_KEY,
  type CreditCard,
  deactivateCreditCard,
  listCreditCards,
  reactivateCreditCard,
} from "@/lib/credit-cards-api"
import { creditCardFailure } from "@/lib/credit-card-messages"

/** UC16: manage credit cards: register, change the days, deactivate. */
export function CreditCardsScreen() {
  const queryClient = useQueryClient()
  const [dialog, setDialog] = useState<CreditCardDialogMode | null>(null)
  const { data, isPending, isError, refetch } = useQuery({
    queryKey: CREDIT_CARDS_KEY,
    queryFn: listCreditCards,
  })

  const deactivate = useMutation({
    mutationFn: (card: CreditCard) => deactivateCreditCard(card.id),
    onSuccess: (card) => {
      void queryClient.invalidateQueries({ queryKey: CREDIT_CARDS_KEY })
      toast.success(`${card.name} foi desativado. O histórico e as faturas continuam.`)
    },
    onError: (error) => toast.error(creditCardFailure(error, "desativar o cartão")),
  })
  const reactivate = useMutation({
    mutationFn: (card: CreditCard) => reactivateCreditCard(card.id),
    onSuccess: (card) => {
      void queryClient.invalidateQueries({ queryKey: CREDIT_CARDS_KEY })
      toast.success(`${card.name} foi reativado.`)
    },
    onError: (error) => toast.error(creditCardFailure(error, "reativar o cartão")),
  })

  const cards = data ?? []
  const active = cards.filter((card) => card.active)
  const inactive = cards.filter((card) => !card.active)

  return (
    <section className="flex flex-col gap-6">
      <div className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
        <div className="flex flex-col gap-1">
          <h1 className="font-display text-3xl font-bold tracking-tight">Cartões</h1>
          <p className="max-w-prose text-muted-foreground">
            Cadastre seus cartões de crédito com o dia de fechamento e o dia de vencimento. As
            compras entram na fatura certa e só mexem no saldo quando a fatura é paga.
          </p>
        </div>
        <Button onClick={() => setDialog({ kind: "create" })}>
          <PlusIcon strokeWidth={1.75} />
          Novo cartão
        </Button>
      </div>

      {isError ? (
        <Alert variant="destructive">
          <TriangleAlertIcon strokeWidth={1.75} />
          <AlertDescription>
            <p>Não foi possível carregar os cartões. Verifique a conexão e tente de novo.</p>
            <Button size="sm" variant="outline" className="mt-2" onClick={() => void refetch()}>
              Tentar de novo
            </Button>
          </AlertDescription>
        </Alert>
      ) : isPending ? (
        <div className="flex flex-col gap-2" aria-label="Carregando cartões">
          <Skeleton className="h-16 w-full" />
          <Skeleton className="h-16 w-full" />
        </div>
      ) : (
        <>
          {active.length === 0 ? (
            <Empty className="border">
              <EmptyHeader>
                <EmptyMedia variant="icon">
                  <CreditCardIcon strokeWidth={1.75} />
                </EmptyMedia>
                <EmptyTitle>Nenhum cartão ativo</EmptyTitle>
                <EmptyDescription>
                  Cadastre um cartão para lançar compras e acompanhar as faturas.
                </EmptyDescription>
              </EmptyHeader>
              <Button onClick={() => setDialog({ kind: "create" })}>Novo cartão</Button>
            </Empty>
          ) : (
            <Card className="gap-0 py-0">
              <ul aria-label="Cartões ativos" className="divide-y">
                {active.map((card) => (
                  <li
                    key={card.id}
                    className="flex flex-wrap items-center gap-x-3 gap-y-2 px-4 py-3"
                  >
                    <CreditCardIcon
                      strokeWidth={1.75}
                      className="size-4 shrink-0 text-muted-foreground"
                    />
                    <div className="flex min-w-0 flex-1 flex-col">
                      <span className="truncate font-medium">{card.name}</span>
                      <span className="text-sm text-muted-foreground">
                        Fecha dia {card.closingDay} · vence dia {card.dueDay}
                      </span>
                    </div>
                    <Button variant="outline" size="sm" asChild>
                      <Link href={`/cartoes/${card.id}`}>Ver faturas</Link>
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon"
                      aria-label={`Editar ${card.name}`}
                      onClick={() => setDialog({ kind: "edit", card })}
                    >
                      <PencilIcon strokeWidth={1.75} />
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      aria-label={`Desativar ${card.name}`}
                      disabled={deactivate.isPending}
                      onClick={() => deactivate.mutate(card)}
                    >
                      Desativar
                    </Button>
                  </li>
                ))}
              </ul>
            </Card>
          )}

          {inactive.length > 0 && (
            <div className="flex flex-col gap-3">
              <h2 className="font-display text-xl font-bold tracking-tight">Desativados</h2>
              <Card className="gap-0 py-0">
                <ul aria-label="Cartões desativados" className="divide-y">
                  {inactive.map((card) => (
                    <li
                      key={card.id}
                      className="flex flex-wrap items-center gap-x-3 gap-y-2 px-4 py-3"
                    >
                      <CreditCardIcon
                        strokeWidth={1.75}
                        className="size-4 shrink-0 text-muted-foreground"
                      />
                      <span className="min-w-0 flex-1 truncate text-muted-foreground">
                        {card.name}
                      </span>
                      <Badge variant="secondary">Desativado</Badge>
                      <Button variant="outline" size="sm" asChild>
                        <Link href={`/cartoes/${card.id}`}>Ver faturas</Link>
                      </Button>
                      <Button
                        variant="outline"
                        size="sm"
                        aria-label={`Reativar ${card.name}`}
                        disabled={reactivate.isPending}
                        onClick={() => reactivate.mutate(card)}
                      >
                        Reativar
                      </Button>
                    </li>
                  ))}
                </ul>
              </Card>
            </div>
          )}
        </>
      )}

      {dialog && (
        <CreditCardDialog
          // A new key per target resets the form state.
          key={dialog.kind === "edit" ? dialog.card.id : "create"}
          mode={dialog}
          onClose={() => setDialog(null)}
        />
      )}
    </section>
  )
}
