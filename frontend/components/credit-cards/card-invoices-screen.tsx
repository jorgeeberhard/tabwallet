"use client"

import { useQuery } from "@tanstack/react-query"
import { ArrowLeftIcon, InboxIcon, PlusIcon, TriangleAlertIcon } from "lucide-react"
import Link from "next/link"
import { useState } from "react"

import {
  CardPurchaseDialog,
  type CardPurchaseDialogMode,
} from "@/components/credit-cards/card-purchase-dialog"
import { CardPurchaseRow } from "@/components/credit-cards/card-purchase-row"
import { ChargeDialog, type ChargeDialogMode } from "@/components/credit-cards/charge-dialog"
import { ChargeRow } from "@/components/credit-cards/charge-row"
import { CorrectPaymentDialog } from "@/components/credit-cards/correct-payment-dialog"
import { DeleteCardPurchaseDialog } from "@/components/credit-cards/delete-card-purchase-dialog"
import { DeleteChargeDialog } from "@/components/credit-cards/delete-charge-dialog"
import { InvoicePaymentRow } from "@/components/credit-cards/invoice-payment-row"
import { PayInvoiceDialog } from "@/components/credit-cards/pay-invoice-dialog"
import { InvoiceCard, invoiceStateLabel } from "@/components/invoice-card"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card } from "@/components/ui/card"
import { Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle } from "@/components/ui/empty"
import { Field, FieldLabel } from "@/components/ui/field"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { Skeleton } from "@/components/ui/skeleton"
import {
  type CardPurchase,
  type Charge,
  CREDIT_CARDS_KEY,
  getInvoice,
  type Invoice,
  invoiceKey,
  invoicesKey,
  listCreditCards,
  listInvoices,
} from "@/lib/credit-cards-api"
import { formatMonthOfYear } from "@/lib/format"

/** The invoice to show first: the first open one, else the latest. */
function defaultInvoice(invoices: Invoice[]): Invoice | undefined {
  return invoices.find((invoice) => invoice.state === "open") ?? invoices[invoices.length - 1]
}

interface CardInvoicesScreenProps {
  cardId: string
}

/** UC15/UC16: the invoices of one credit card, with the purchases of each. */
export function CardInvoicesScreen({ cardId }: CardInvoicesScreenProps) {
  const [chosen, setChosen] = useState<string | null>(null)
  const [dialog, setDialog] = useState<CardPurchaseDialogMode | null>(null)
  const [toDelete, setToDelete] = useState<CardPurchase | null>(null)
  const [paying, setPaying] = useState(false)
  const [correcting, setCorrecting] = useState(false)
  const [chargeDialog, setChargeDialog] = useState<ChargeDialogMode | null>(null)
  const [chargeToDelete, setChargeToDelete] = useState<Charge | null>(null)

  const cards = useQuery({ queryKey: CREDIT_CARDS_KEY, queryFn: listCreditCards })
  const card = cards.data?.find((candidate) => candidate.id === cardId)
  const invoices = useQuery({
    queryKey: invoicesKey(cardId),
    queryFn: () => listInvoices(cardId),
    enabled: !!card,
  })

  const list = invoices.data ?? []
  const closingMonth =
    list.find((invoice) => invoice.closingMonth === chosen)?.closingMonth ??
    defaultInvoice(list)?.closingMonth
  const detail = useQuery({
    queryKey: invoiceKey(cardId, closingMonth ?? ""),
    queryFn: () => getInvoice(cardId, closingMonth as string),
    enabled: !!closingMonth,
  })

  const backLink = (
    <Button variant="ghost" size="sm" className="-ml-2 w-fit" asChild>
      <Link href="/cartoes">
        <ArrowLeftIcon strokeWidth={1.75} />
        Cartões
      </Link>
    </Button>
  )

  if (cards.isError || invoices.isError) {
    return (
      <section className="flex flex-col gap-6">
        {backLink}
        <Alert variant="destructive">
          <TriangleAlertIcon strokeWidth={1.75} />
          <AlertDescription>
            <p>Não foi possível carregar as faturas. Verifique a conexão e tente de novo.</p>
            <Button
              size="sm"
              variant="outline"
              className="mt-2"
              onClick={() => {
                void cards.refetch()
                void invoices.refetch()
              }}
            >
              Tentar de novo
            </Button>
          </AlertDescription>
        </Alert>
      </section>
    )
  }

  if (cards.isSuccess && !card) {
    return (
      <section className="flex flex-col gap-6">
        {backLink}
        <Alert variant="warning">
          <TriangleAlertIcon strokeWidth={1.75} />
          <AlertDescription>Este cartão não existe.</AlertDescription>
        </Alert>
      </section>
    )
  }

  return (
    <section className="flex flex-col gap-6">
      {backLink}
      <div className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
        <div className="flex flex-col gap-1">
          <h1 className="font-display text-3xl font-bold tracking-tight">
            {card ? `Faturas do ${card.name}` : "Faturas"}
          </h1>
          <p className="max-w-prose text-muted-foreground">
            {card
              ? `Fecha dia ${card.closingDay} e vence dia ${card.dueDay}. A fatura conta no mês do vencimento.`
              : "Carregando o cartão..."}
          </p>
          {card && !card.active && (
            <Badge variant="secondary" className="mt-1">
              Cartão desativado
            </Badge>
          )}
        </div>
        <Button
          disabled={!card?.active}
          onClick={() => setDialog({ kind: "create", creditCardId: cardId })}
        >
          <PlusIcon strokeWidth={1.75} />
          Nova compra
        </Button>
      </div>

      {!card || invoices.isPending ? (
        <div className="flex flex-col gap-2" aria-label="Carregando faturas">
          <Skeleton className="h-40 w-full max-w-xl" />
          <Skeleton className="h-16 w-full" />
        </div>
      ) : (
        <>
          <Field className="max-w-xl">
            <FieldLabel htmlFor="invoice-select">Fatura</FieldLabel>
            <Select value={closingMonth} onValueChange={setChosen}>
              <SelectTrigger id="invoice-select" className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {[...list].reverse().map((invoice) => (
                  <SelectItem key={invoice.closingMonth} value={invoice.closingMonth}>
                    {formatMonthOfYear(invoice.referenceMonth)} · {invoiceStateLabel(invoice.state)}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </Field>

          {detail.isError ? (
            <Alert variant="destructive">
              <TriangleAlertIcon strokeWidth={1.75} />
              <AlertDescription>
                <p>Não foi possível carregar esta fatura.</p>
                <Button
                  size="sm"
                  variant="outline"
                  className="mt-2"
                  onClick={() => void detail.refetch()}
                >
                  Tentar de novo
                </Button>
              </AlertDescription>
            </Alert>
          ) : detail.isPending ? (
            <Skeleton className="h-40 w-full max-w-xl" />
          ) : (
            <>
              <InvoiceCard
                cardName={card.name}
                invoice={detail.data.invoice}
                carriedBalance={detail.data.carriedBalance}
                onPayEarly={() => setPaying(true)}
                onCorrectPayment={() => setCorrecting(true)}
                onAddCharges={() =>
                  setChargeDialog({
                    kind: "create",
                    cardId,
                    cardName: card.name,
                    invoice: detail.data.invoice,
                  })
                }
              />
              {detail.data.invoice.state === "paid" && (
                <p className="max-w-prose text-sm text-muted-foreground">
                  Fatura paga: as compras e os encargos dela não podem mais ser editados nem
                  excluídos.
                </p>
              )}
              {detail.data.payments.length > 0 && (
                <div className="flex flex-col gap-2">
                  <h2 className="font-display text-lg font-bold tracking-tight">
                    Pagamentos da fatura
                  </h2>
                  <Card className="gap-0 overflow-hidden py-0">
                    <ul aria-label="Pagamentos da fatura" className="divide-y">
                      {detail.data.payments.map((payment) => (
                        <InvoicePaymentRow
                          key={payment.id ?? "automatic"}
                          payment={payment}
                          referenceMonth={detail.data.invoice.referenceMonth}
                        />
                      ))}
                    </ul>
                  </Card>
                </div>
              )}
              {detail.data.charges.length > 0 && (
                <div className="flex flex-col gap-2">
                  <h2 className="font-display text-lg font-bold tracking-tight">
                    Encargos da fatura
                  </h2>
                  <Card className="gap-0 overflow-hidden py-0">
                    <ul aria-label="Encargos da fatura" className="divide-y">
                      {detail.data.charges.map((charge) => (
                        <ChargeRow
                          key={charge.id}
                          charge={charge}
                          showCard={false}
                          onEdit={(target) => setChargeDialog({ kind: "edit", charge: target })}
                          onDelete={setChargeToDelete}
                        />
                      ))}
                    </ul>
                  </Card>
                </div>
              )}
              {detail.data.purchases.length === 0 ? (
                <Empty className="border">
                  <EmptyHeader>
                    <EmptyMedia variant="icon">
                      <InboxIcon strokeWidth={1.75} />
                    </EmptyMedia>
                    <EmptyTitle>Nenhuma compra nesta fatura</EmptyTitle>
                    <EmptyDescription>
                      As compras feitas até o dia de fechamento entram na fatura que fecha no mês.
                    </EmptyDescription>
                  </EmptyHeader>
                </Empty>
              ) : (
                <Card className="gap-0 py-0">
                  <ul aria-label="Compras da fatura" className="divide-y">
                    {detail.data.purchases.map((purchase) => (
                      <CardPurchaseRow
                        key={purchase.id}
                        purchase={purchase}
                        showCard={false}
                        onEdit={(target) => setDialog({ kind: "edit", purchase: target })}
                        onDelete={setToDelete}
                      />
                    ))}
                  </ul>
                </Card>
              )}
            </>
          )}
        </>
      )}

      {dialog && (
        <CardPurchaseDialog
          key={dialog.kind === "edit" ? dialog.purchase.id : "create"}
          mode={dialog}
          onClose={() => setDialog(null)}
        />
      )}
      <DeleteCardPurchaseDialog purchase={toDelete} onClose={() => setToDelete(null)} />
      {chargeDialog && (
        <ChargeDialog
          key={chargeDialog.kind === "edit" ? chargeDialog.charge.id : "create"}
          mode={chargeDialog}
          onClose={() => setChargeDialog(null)}
        />
      )}
      <DeleteChargeDialog charge={chargeToDelete} onClose={() => setChargeToDelete(null)} />
      {correcting && card && detail.data && (
        <CorrectPaymentDialog
          key={detail.data.invoice.closingMonth}
          cardId={cardId}
          cardName={card.name}
          invoice={detail.data.invoice}
          onClose={() => setCorrecting(false)}
        />
      )}
      {paying && card && detail.data && (
        <PayInvoiceDialog
          key={detail.data.invoice.closingMonth}
          cardId={cardId}
          cardName={card.name}
          invoice={detail.data.invoice}
          onClose={() => setPaying(false)}
        />
      )}
    </section>
  )
}
