"use client"

import { EllipsisIcon, LayersIcon, PencilIcon, Trash2Icon } from "lucide-react"

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
import type { InvoiceInstallment } from "@/lib/credit-cards-api"
import { formatDate, formatMonthOfYear } from "@/lib/format"

interface InvoiceInstallmentRowProps {
  item: InvoiceInstallment
  onEdit: (item: InvoiceInstallment) => void
  /** Delete the purchase, or cancel its remaining Installments when some are paid. */
  onCancel: (item: InvoiceInstallment) => void
}

/**
 * One Installment as a line of its invoice ("Geladeira 3/10"). Editing and
 * cancelling act on the whole purchase: an Installment in a paid invoice is
 * history and cannot be cancelled, and a purchase with paid Installments can
 * only change its Category.
 */
export function InvoiceInstallmentRow({ item, onEdit, onCancel }: InvoiceInstallmentRowProps) {
  const paid = item.invoice.state === "paid"
  const cancelLabel = item.purchaseHasPaidInstallments ? "Cancelar parcelas restantes" : "Excluir"
  const label = `${item.description} ${item.number}/${item.installmentCount}`

  return (
    <li className="flex items-center gap-3 px-4 py-3">
      <span className="flex size-9 shrink-0 items-center justify-center rounded-full bg-expense-soft text-expense">
        <LayersIcon strokeWidth={1.75} className="size-4" aria-hidden />
        <span className="sr-only">Parcela de compra parcelada</span>
      </span>

      <div className="flex min-w-0 flex-1 flex-col gap-1">
        <span className="flex items-baseline gap-2">
          <span className="truncate font-medium">{item.description}</span>
          <span className="shrink-0 tabular-nums text-sm font-semibold text-muted-foreground">
            {item.number}/{item.installmentCount}
          </span>
        </span>
        <span className="flex flex-wrap items-center gap-x-2 gap-y-1 text-sm text-muted-foreground">
          <time dateTime={item.date}>Compra de {formatDate(item.date)}</time>
          <Badge variant={item.category.active ? "secondary" : "outline"}>
            {item.category.name}
            {!item.category.active && " (desativada)"}
          </Badge>
          <Badge variant={paid ? "income" : "brand"}>
            Fatura de {formatMonthOfYear(item.referenceMonth)} ·{" "}
            {invoiceStateLabel(item.invoice.state).toLowerCase()}
          </Badge>
        </span>
      </div>

      <Money value={item.amount} kind="expense" />

      <div className="hidden items-center sm:flex">
        <Button
          variant="ghost"
          size="icon"
          aria-label={`Editar ${label}`}
          onClick={() => onEdit(item)}
        >
          <PencilIcon strokeWidth={1.75} />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          disabled={paid}
          aria-label={`${cancelLabel} ${label}`}
          title={paid ? "Parcelas de fatura paga não podem ser canceladas" : undefined}
          onClick={() => onCancel(item)}
        >
          <Trash2Icon strokeWidth={1.75} />
        </Button>
      </div>

      <div className="sm:hidden">
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="ghost" size="icon" aria-label={`Ações de ${label}`}>
              <EllipsisIcon strokeWidth={1.75} />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem onSelect={() => onEdit(item)}>
              <PencilIcon strokeWidth={1.75} />
              Editar
            </DropdownMenuItem>
            <DropdownMenuItem variant="destructive" disabled={paid} onSelect={() => onCancel(item)}>
              <Trash2Icon strokeWidth={1.75} />
              {cancelLabel}
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </li>
  )
}
