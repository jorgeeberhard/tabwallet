"use client"

import { useQuery } from "@tanstack/react-query"
import { useState } from "react"

import { BalanceCard } from "@/components/balance-card"
import { OpeningBalanceDialog } from "@/components/balance/opening-balance-dialog"
import { Money } from "@/components/money"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { Button } from "@/components/ui/button"
import { Skeleton } from "@/components/ui/skeleton"
import { BALANCE_KEY, getBalance } from "@/lib/balance-api"

/**
 * The Current Balance on the home page, with the Opening Balance it includes.
 * While no Opening Balance was informed it offers to inform one; once informed
 * the offer goes away and the value is shown.
 */
export function BalanceSection() {
  const balance = useQuery({ queryKey: BALANCE_KEY, queryFn: getBalance })
  const [informing, setInforming] = useState(false)

  if (balance.isPending) {
    return <Skeleton className="h-36 w-full max-w-xl rounded-xl" />
  }
  if (balance.isError) {
    return (
      <Alert variant="destructive" className="max-w-xl">
        <AlertDescription>
          Não foi possível carregar o saldo.{" "}
          <button type="button" className="font-semibold underline" onClick={() => balance.refetch()}>
            Tentar de novo
          </button>
        </AlertDescription>
      </Alert>
    )
  }

  const data = balance.data
  return (
    <>
      <BalanceCard balance={data.currentBalance}>
        {data.openingBalanceSet && data.openingBalance !== null ? (
          <p className="flex flex-wrap items-baseline gap-x-2 text-sm text-muted-foreground">
            Saldo inicial informado:
            <Money value={data.openingBalance} showSign={false} kind="neutral" size="sm" />
          </p>
        ) : (
          <div className="flex flex-col items-start gap-2">
            <p className="text-sm text-muted-foreground">
              Já tinha dinheiro antes de usar o TabWallet? Informe o saldo inicial para que o saldo
              atual o considere.
            </p>
            <Button variant="outline" size="sm" onClick={() => setInforming(true)}>
              Informar saldo inicial
            </Button>
          </div>
        )}
      </BalanceCard>
      {informing && <OpeningBalanceDialog onClose={() => setInforming(false)} />}
    </>
  )
}
