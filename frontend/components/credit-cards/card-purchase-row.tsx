"use client"

import { CreditCardIcon, EllipsisIcon, PencilIcon, Trash2Icon } from "lucide-react"

import { invoiceStateLabel } from "@/components/invoice-card"
import { Money } from "@/components/money"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import type { CardPurchase } from "@/lib/credit-cards-api"
import { formatDate, formatMonthOfYear } from "@/lib/format"

interface CardPurchaseRowProps {
  purchase: CardPurchase
  /** Hide the card name (the invoice screen is already about one card). */
  showCard?: boolean
  onEdit: (purchase: CardPurchase) => void
  onDelete: (purchase: CardPurchase) => void
}

/**
 * One purchase with a credit card (Design System `TransactionRow`): it shows
 * the card and the invoice it belongs to. A purchase in a paid invoice is
 * history: it cannot be edited or deleted, so its actions are off.
 */
export function CardPurchaseRow({
  purchase,
  showCard = true,
  onEdit,
  onDelete,
}: CardPurchaseRowProps) {
  const locked = purchase.invoice.state === "paid"
  const invoiceLabel = `Fatura de ${formatMonthOfYear(purchase.referenceMonth)}`

  return (
    <li className="flex items-center gap-3 px-4 py-3">
      <span className="flex size-9 shrink-0 items-center justify-center rounded-full bg-expense-soft text-expense">
        <CreditCardIcon strokeWidth={1.75} className="size-4" aria-hidden />
        <span className="sr-only">Compra no cartão</span>
      </span>

      <div className="flex min-w-0 flex-1 flex-col gap-1">
        <span className="truncate font-medium">{purchase.description}</span>
        <span className="flex flex-wrap items-center gap-x-2 gap-y-1 text-sm text-muted-foreground">
          <time dateTime={purchase.date}>{formatDate(purchase.date)}</time>
          <Badge variant={purchase.category.active ? "secondary" : "outline"}>
            {purchase.category.name}
            {!purchase.category.active && " (desativada)"}
          </Badge>
          {showCard && <Badge variant="outline">{purchase.creditCard.name}</Badge>}
          <Badge variant={purchase.invoice.state === "paid" ? "income" : "brand"}>
            {invoiceLabel} · {invoiceStateLabel(purchase.invoice.state).toLowerCase()}
          </Badge>
          {purchase.isProjection && <Badge variant="warning">Projeção</Badge>}
        </span>
      </div>

      <Money value={purchase.amount} kind="expense" />

      <div className="hidden items-center sm:flex">
        <Button
          variant="ghost"
          size="icon"
          disabled={locked}
          aria-label={`Editar ${purchase.description}`}
          title={locked ? "Compras de fatura paga não podem ser alteradas" : undefined}
          onClick={() => onEdit(purchase)}
        >
          <PencilIcon strokeWidth={1.75} />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          disabled={locked}
          aria-label={`Excluir ${purchase.description}`}
          title={locked ? "Compras de fatura paga não podem ser excluídas" : undefined}
          onClick={() => onDelete(purchase)}
        >
          <Trash2Icon strokeWidth={1.75} />
        </Button>
      </div>

      <div className="sm:hidden">
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              disabled={locked}
              aria-label={`Ações de ${purchase.description}`}
            >
              <EllipsisIcon strokeWidth={1.75} />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem onSelect={() => onEdit(purchase)}>
              <PencilIcon strokeWidth={1.75} />
              Editar
            </DropdownMenuItem>
            <DropdownMenuItem variant="destructive" onSelect={() => onDelete(purchase)}>
              <Trash2Icon strokeWidth={1.75} />
              Excluir
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </li>
  )
}
