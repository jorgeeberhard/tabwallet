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
import { BALANCE_KEY } from "@/lib/balance-api"
import { creditCardFailure } from "@/lib/credit-card-messages"
import { type Charge, CREDIT_CARDS_KEY, deleteCharge, INVOICES_KEY } from "@/lib/credit-cards-api"
import { TRANSACTIONS_KEY } from "@/lib/transactions-api"

interface DeleteChargeDialogProps {
  /** The charge to delete; `null` keeps the dialog closed. */
  charge: Charge | null
  onClose: () => void
}

/** Asks for confirmation and deletes a Charge (unless its invoice is paid). */
export function DeleteChargeDialog({ charge, onClose }: DeleteChargeDialogProps) {
  const queryClient = useQueryClient()
  const remove = useMutation({
    mutationFn: (target: Charge) => deleteCharge(target.id),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: TRANSACTIONS_KEY })
      void queryClient.invalidateQueries({ queryKey: INVOICES_KEY })
      void queryClient.invalidateQueries({ queryKey: CREDIT_CARDS_KEY })
      void queryClient.invalidateQueries({ queryKey: BALANCE_KEY })
      toast.success("Encargo excluído.")
      onClose()
    },
    onError: (error) => {
      toast.error(creditCardFailure(error, "excluir o encargo"))
      onClose()
    },
  })

  return (
    <AlertDialog open={charge !== null} onOpenChange={(open) => !open && onClose()}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Excluir encargo?</AlertDialogTitle>
          <AlertDialogDescription>
            {charge
              ? `"${charge.description}" será removido da fatura. Essa ação não pode ser desfeita.`
              : ""}
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel disabled={remove.isPending}>Cancelar</AlertDialogCancel>
          <Button
            variant="destructive"
            disabled={remove.isPending}
            onClick={() => charge && remove.mutate(charge)}
          >
            {remove.isPending && <Spinner />}
            Excluir
          </Button>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  )
}
