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
  type CardPurchase,
  CREDIT_CARDS_KEY,
  deleteCardPurchase,
  INVOICES_KEY,
} from "@/lib/credit-cards-api"
import { creditCardFailure } from "@/lib/credit-card-messages"
import { TRANSACTIONS_KEY } from "@/lib/transactions-api"

interface DeleteCardPurchaseDialogProps {
  /** The purchase to delete; `null` keeps the dialog closed. */
  purchase: CardPurchase | null
  onClose: () => void
}

/** Asks for confirmation and deletes a card purchase (unless its invoice is paid). */
export function DeleteCardPurchaseDialog({ purchase, onClose }: DeleteCardPurchaseDialogProps) {
  const queryClient = useQueryClient()
  const remove = useMutation({
    mutationFn: (target: CardPurchase) => deleteCardPurchase(target.id),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: TRANSACTIONS_KEY })
      void queryClient.invalidateQueries({ queryKey: INVOICES_KEY })
      void queryClient.invalidateQueries({ queryKey: CREDIT_CARDS_KEY })
      toast.success("Compra excluída.")
      onClose()
    },
    onError: (error) => {
      toast.error(creditCardFailure(error, "excluir a compra"))
      onClose()
    },
  })

  return (
    <AlertDialog open={purchase !== null} onOpenChange={(open) => !open && onClose()}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Excluir compra?</AlertDialogTitle>
          <AlertDialogDescription>
            {purchase
              ? `"${purchase.description}" será removida da fatura. Essa ação não pode ser desfeita.`
              : ""}
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel disabled={remove.isPending}>Cancelar</AlertDialogCancel>
          <Button
            variant="destructive"
            disabled={remove.isPending}
            onClick={() => purchase && remove.mutate(purchase)}
          >
            {remove.isPending && <Spinner />}
            Excluir
          </Button>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  )
}
