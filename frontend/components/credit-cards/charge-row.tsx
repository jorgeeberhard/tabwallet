"use client"

import { EllipsisIcon, PencilIcon, PercentIcon, Trash2Icon } from "lucide-react"

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
import type { Charge } from "@/lib/credit-cards-api"
import { formatDate, formatMonthOfYear } from "@/lib/format"

interface ChargeRowProps {
  charge: Charge
  /** Hide the card name (the invoice screen is already about one card). */
  showCard?: boolean
  onEdit: (charge: Charge) => void
  onDelete: (charge: Charge) => void
}

/**
 * One Charge (Encargo): interest, penalty or IOF the bank charged on an
 * invoice. It is an expense of the "Juros e encargos" Category, linked to the
 * invoice. Like the purchases, a charge of a paid invoice is history: its
 * actions are off.
 */
export function ChargeRow({ charge, showCard = true, onEdit, onDelete }: ChargeRowProps) {
  const locked = charge.invoice.state === "paid"
  const invoiceLabel = `Fatura de ${formatMonthOfYear(charge.referenceMonth)}`

  return (
    <li className="flex items-center gap-3 px-4 py-3">
      <span className="flex size-9 shrink-0 items-center justify-center rounded-full bg-expense-soft text-expense">
        <PercentIcon strokeWidth={1.75} className="size-4" aria-hidden />
        <span className="sr-only">Encargos</span>
      </span>

      <div className="flex min-w-0 flex-1 flex-col gap-1">
        <span className="truncate font-medium">{charge.description}</span>
        <span className="flex flex-wrap items-center gap-x-2 gap-y-1 text-sm text-muted-foreground">
          <time dateTime={charge.date}>{formatDate(charge.date)}</time>
          <Badge variant="secondary">Encargos</Badge>
          <Badge variant={charge.category.active ? "outline" : "secondary"}>
            {charge.category.name}
            {!charge.category.active && " (desativada)"}
          </Badge>
          {showCard && <Badge variant="outline">{charge.creditCard.name}</Badge>}
          <Badge variant={charge.invoice.state === "paid" ? "income" : "brand"}>
            {invoiceLabel} · {invoiceStateLabel(charge.invoice.state).toLowerCase()}
          </Badge>
          {charge.isProjection && <Badge variant="warning">Projeção</Badge>}
        </span>
      </div>

      <Money value={charge.amount} kind="expense" />

      <div className="hidden items-center sm:flex">
        <Button
          variant="ghost"
          size="icon"
          disabled={locked}
          aria-label={`Editar ${charge.description}`}
          title={locked ? "Encargos de fatura paga não podem ser alterados" : undefined}
          onClick={() => onEdit(charge)}
        >
          <PencilIcon strokeWidth={1.75} />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          disabled={locked}
          aria-label={`Excluir ${charge.description}`}
          title={locked ? "Encargos de fatura paga não podem ser excluídos" : undefined}
          onClick={() => onDelete(charge)}
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
              aria-label={`Ações de ${charge.description}`}
            >
              <EllipsisIcon strokeWidth={1.75} />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem onSelect={() => onEdit(charge)}>
              <PencilIcon strokeWidth={1.75} />
              Editar
            </DropdownMenuItem>
            <DropdownMenuItem variant="destructive" onSelect={() => onDelete(charge)}>
              <Trash2Icon strokeWidth={1.75} />
              Excluir
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </li>
  )
}
