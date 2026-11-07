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
import type { InstallmentPurchase } from "@/lib/credit-cards-api"
import { formatBRL, formatDate, formatMonthOfYear } from "@/lib/format"

interface InstallmentPurchaseRowProps {
  purchase: InstallmentPurchase
  onEdit: (purchase: InstallmentPurchase) => void
  /** Delete the purchase, or cancel its remaining Installments when some are paid. */
  onCancel: (purchase: InstallmentPurchase) => void
}

/** What the person can do about a purchase, from the state of its Installments. */
export function installmentPurchaseActions(purchase: InstallmentPurchase) {
  const cancellable = purchase.installments.filter(
    (installment) => installment.invoice.state !== "paid" && !installment.anticipated,
  )
  // Paid and anticipated Installments are history: "delete" leaves them.
  const onlyRemaining = purchase.hasPaidInstallments || purchase.hasAnticipatedInstallments
  return {
    /** Nothing is left to cancel when every Installment is paid or anticipated. */
    canCancel: cancellable.length > 0,
    /** With paid or anticipated Installments, "delete" only cancels the remaining ones. */
    cancelsOnlyRemaining: onlyRemaining,
    cancelLabel: onlyRemaining ? "Cancelar parcelas restantes" : "Excluir",
  }
}

/**
 * A purchase split in Installments in the statement: the whole purchase, with
 * its Installments listed under it (number, invoice and state of each). The
 * "3/10" lines of one invoice are `InvoiceInstallmentRow`.
 */
export function InstallmentPurchaseRow({
  purchase,
  onEdit,
  onCancel,
}: InstallmentPurchaseRowProps) {
  const { canCancel, cancelLabel } = installmentPurchaseActions(purchase)
  const first = purchase.installments[0]
  const last = purchase.installments[purchase.installments.length - 1]
  const share = first ? first.amount : purchase.total
  const summary = purchase.cancelled
    ? `${purchase.installments.length} de ${purchase.installmentCount} parcelas · restantes canceladas`
    : `${purchase.installmentCount}x`

  return (
    <li className="px-4 py-3">
      <div className="flex items-center gap-3">
        <span className="flex size-9 shrink-0 items-center justify-center rounded-full bg-expense-soft text-expense">
          <LayersIcon strokeWidth={1.75} className="size-4" aria-hidden />
          <span className="sr-only">Compra parcelada</span>
        </span>

        <div className="flex min-w-0 flex-1 flex-col gap-1">
          <span className="truncate font-medium">{purchase.description}</span>
          <span className="flex flex-wrap items-center gap-x-2 gap-y-1 text-sm text-muted-foreground">
            <time dateTime={purchase.date}>{formatDate(purchase.date)}</time>
            <Badge variant={purchase.category.active ? "secondary" : "outline"}>
              {purchase.category.name}
              {!purchase.category.active && " (desativada)"}
            </Badge>
            <Badge variant="outline">{purchase.creditCard.name}</Badge>
            <Badge variant="brand">{summary}</Badge>
            {first && last && (
              <span>
                {first === last
                  ? `Fatura de ${formatMonthOfYear(first.referenceMonth)}`
                  : `Faturas de ${formatMonthOfYear(first.referenceMonth)} a ${formatMonthOfYear(last.referenceMonth)}`}
              </span>
            )}
            {purchase.isProjection && <Badge variant="warning">Projeção</Badge>}
          </span>
        </div>

        <div className="flex flex-col items-end">
          <Money value={purchase.total} kind="expense" />
          {!purchase.cancelled && (
            <span className="text-xs text-muted-foreground">
              parcelas de <Money value={share} showSign={false} size="sm" />
            </span>
          )}
        </div>

        <div className="hidden items-center sm:flex">
          <Button
            variant="ghost"
            size="icon"
            aria-label={`Editar ${purchase.description}`}
            onClick={() => onEdit(purchase)}
          >
            <PencilIcon strokeWidth={1.75} />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            disabled={!canCancel}
            aria-label={`${cancelLabel} ${purchase.description}`}
            title={canCancel ? undefined : "Todas as parcelas estão pagas ou antecipadas"}
            onClick={() => onCancel(purchase)}
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
              <DropdownMenuItem
                variant="destructive"
                disabled={!canCancel}
                onSelect={() => onCancel(purchase)}
              >
                <Trash2Icon strokeWidth={1.75} />
                {cancelLabel}
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </div>

      <details className="mt-2 pl-12 text-sm">
        <summary className="w-fit cursor-pointer text-muted-foreground hover:text-foreground">
          Ver parcelas
        </summary>
        <ol aria-label={`Parcelas de ${purchase.description}`} className="mt-2 flex flex-col gap-1">
          {purchase.installments.map((installment) => (
            <li key={installment.id} className="flex items-center gap-2">
              <span className="w-12 tabular-nums text-muted-foreground">
                {installment.number}/{purchase.installmentCount}
              </span>
              <Money value={installment.amount} kind="expense" size="sm" />
              <span className="text-muted-foreground">
                fatura de {formatMonthOfYear(installment.referenceMonth)} ·{" "}
                {invoiceStateLabel(installment.invoice.state).toLowerCase()}
              </span>
              {installment.anticipated && (
                <Badge variant="outline">
                  Antecipada
                  {installment.discountCents > 0 &&
                    ` · desconto de ${formatBRL(installment.discountCents / 100)}`}
                </Badge>
              )}
            </li>
          ))}
        </ol>
      </details>
    </li>
  )
}
