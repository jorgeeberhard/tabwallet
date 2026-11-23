"use client"

import { useMutation, useQueryClient } from "@tanstack/react-query"
import { toast } from "sonner"

import {
  AlertDialog,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog"
import { Button } from "@/components/ui/button"
import { Spinner } from "@/components/ui/spinner"
import {
  cancelInstallmentPurchase,
  CREDIT_CARDS_KEY,
  INVOICES_KEY,
} from "@/lib/credit-cards-api"
import { creditCardFailure } from "@/lib/credit-card-messages"
import { TRANSACTIONS_KEY } from "@/lib/transactions-api"

/** What the dialog needs to know about the purchase. */
export interface CancelInstallmentTarget {
  id: string
  description: string
  /** Some Installment is in a paid invoice: only the remaining ones are cancelled. */
  hasPaidInstallments: boolean
  /** Some Installment was anticipated: it stays, like a paid one. */
  hasAnticipatedInstallments: boolean
}

interface CancelInstallmentPurchaseDialogProps {
  /** The purchase to delete or cancel; `null` keeps the dialog closed. */
  purchase: CancelInstallmentTarget | null
  onClose: () => void
}

/**
 * Asks for confirmation and removes an installment purchase. While no
 * Installment is paid the whole purchase is deleted; with paid Installments
 * those stay and only the remaining ones are cancelled.
 */
export function CancelInstallmentPurchaseDialog({
  purchase,
  onClose,
}: CancelInstallmentPurchaseDialogProps) {
  const queryClient = useQueryClient()
  const remove = useMutation({
    mutationFn: (target: CancelInstallmentTarget) => cancelInstallmentPurchase(target.id),
    onSuccess: (remaining) => {
      void queryClient.invalidateQueries({ queryKey: TRANSACTIONS_KEY })
      void queryClient.invalidateQueries({ queryKey: INVOICES_KEY })
      void queryClient.invalidateQueries({ queryKey: CREDIT_CARDS_KEY })
      toast.success(
        remaining ? "Parcelas restantes canceladas. As pagas continuam." : "Compra parcelada excluída.",
      )
      onClose()
    },
    onError: (error) => {
      toast.error(creditCardFailure(error, "cancelar a compra parcelada"))
      onClose()
    },
  })

  const onlyRemaining =
    (purchase?.hasPaidInstallments || purchase?.hasAnticipatedInstallments) ?? false

  return (
    <AlertDialog open={purchase !== null} onOpenChange={(open) => !open && onClose()}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>
            {onlyRemaining ? "Cancelar parcelas restantes?" : "Excluir compra parcelada?"}
          </AlertDialogTitle>
          <AlertDialogDescription>
            {purchase
              ? onlyRemaining
                ? `"${purchase.description}" já tem parcelas pagas ou antecipadas. Elas continuam como estão; só as parcelas restantes serão removidas das faturas. Essa ação não pode ser desfeita.`
                : `"${purchase.description}" e todas as suas parcelas serão removidas das faturas. Essa ação não pode ser desfeita.`
              : ""}
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel disabled={remove.isPending}>Voltar</AlertDialogCancel>
          <Button
            variant="destructive"
            disabled={remove.isPending}
            onClick={() => purchase && remove.mutate(purchase)}
          >
            {remove.isPending && <Spinner />}
            {onlyRemaining ? "Cancelar parcelas" : "Excluir"}
          </Button>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  )
}
