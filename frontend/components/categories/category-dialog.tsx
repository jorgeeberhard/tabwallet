"use client"

import { useMutation, useQueryClient } from "@tanstack/react-query"
import { TriangleAlertIcon } from "lucide-react"
import { useState } from "react"
import { toast } from "sonner"

import { Alert, AlertDescription } from "@/components/ui/alert"
import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Field, FieldError, FieldLabel } from "@/components/ui/field"
import { Input } from "@/components/ui/input"
import { Spinner } from "@/components/ui/spinner"
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group"
import { ApiError } from "@/lib/api"
import {
  CATEGORIES_KEY,
  type Category,
  type CategoryType,
  createCategory,
  reactivateCategory,
  renameCategory,
} from "@/lib/categories-api"
import { categoryFailure } from "@/lib/category-messages"

export const MAX_CATEGORY_NAME_LENGTH = 60

export type CategoryDialogMode =
  | { kind: "create"; type: CategoryType }
  | { kind: "rename"; category: Category }

interface CategoryDialogProps {
  mode: CategoryDialogMode
  onClose: () => void
  /** Called after a category was created or reactivated, with its type. */
  onSaved: (type: CategoryType) => void
}

/** Create or rename a category. Mount it only while it is open. */
export function CategoryDialog({ mode, onClose, onSaved }: CategoryDialogProps) {
  const queryClient = useQueryClient()
  const isRename = mode.kind === "rename"
  const [name, setName] = useState(isRename ? mode.category.name : "")
  const [type, setType] = useState<CategoryType>(isRename ? mode.category.type : mode.type)
  const [problem, setProblem] = useState<string | null>(null)
  // Set when the name belongs to a deactivated category the user can bring back.
  const [inactiveId, setInactiveId] = useState<string | null>(null)

  const save = useMutation({
    mutationFn: () =>
      isRename ? renameCategory(mode.category.id, name.trim()) : createCategory(name.trim(), type),
    onSuccess: (category) => {
      void queryClient.invalidateQueries({ queryKey: CATEGORIES_KEY })
      toast.success(isRename ? "Categoria renomeada." : "Categoria criada.")
      onSaved(category.type)
      onClose()
    },
    onError: (error) => {
      if (error instanceof ApiError && error.code === "category_inactive_exists") {
        setInactiveId(error.categoryId ?? null)
      }
      setProblem(categoryFailure(error, isRename ? "renomear a categoria" : "criar a categoria"))
    },
  })

  const reactivate = useMutation({
    mutationFn: (id: string) => reactivateCategory(id),
    onSuccess: (category) => {
      void queryClient.invalidateQueries({ queryKey: CATEGORIES_KEY })
      toast.success("Categoria reativada.")
      onSaved(category.type)
      onClose()
    },
    onError: (error) => setProblem(categoryFailure(error, "reativar a categoria")),
  })

  const busy = save.isPending || reactivate.isPending
  const trimmed = name.trim()
  const nameError =
    trimmed.length === 0
      ? "Digite o nome da categoria."
      : trimmed.length > MAX_CATEGORY_NAME_LENGTH
        ? `Use até ${MAX_CATEGORY_NAME_LENGTH} caracteres.`
        : null
  const [showNameError, setShowNameError] = useState(false)

  function submit(event: React.FormEvent) {
    event.preventDefault()
    setProblem(null)
    setInactiveId(null)
    if (nameError) {
      setShowNameError(true)
      return
    }
    save.mutate()
  }

  return (
    <Dialog open onOpenChange={(open) => !open && !busy && onClose()}>
      <DialogContent showCloseButton={false}>
        <form onSubmit={submit} noValidate className="grid gap-4">
          <DialogHeader>
            <DialogTitle className="font-display text-lg font-bold">
              {isRename ? "Renomear categoria" : "Nova categoria"}
            </DialogTitle>
            <DialogDescription>
              {isRename
                ? "O novo nome vale também para as transações que já usam esta categoria."
                : "Escolha o tipo e o nome. O nome não pode se repetir dentro do mesmo tipo."}
            </DialogDescription>
          </DialogHeader>

          {!isRename && (
            <Field>
              <FieldLabel id="category-type-label">Tipo</FieldLabel>
              <ToggleGroup
                type="single"
                aria-labelledby="category-type-label"
                value={type}
                onValueChange={(value) => value && setType(value as CategoryType)}
                spacing={0}
                className="w-full rounded-lg bg-muted p-1"
              >
                <ToggleGroupItem
                  value="expense"
                  className="flex-1 data-[state=on]:bg-card data-[state=on]:shadow-sm"
                >
                  Despesa
                </ToggleGroupItem>
                <ToggleGroupItem
                  value="income"
                  className="flex-1 data-[state=on]:bg-card data-[state=on]:shadow-sm"
                >
                  Receita
                </ToggleGroupItem>
              </ToggleGroup>
            </Field>
          )}

          <Field data-invalid={showNameError && !!nameError}>
            <FieldLabel htmlFor="category-name">Nome</FieldLabel>
            <Input
              id="category-name"
              autoFocus
              autoComplete="off"
              placeholder="Ex.: Pets"
              value={name}
              aria-invalid={showNameError && !!nameError}
              onChange={(event) => setName(event.target.value)}
            />
            {showNameError && nameError && <FieldError>{nameError}</FieldError>}
          </Field>

          {problem && (
            <Alert variant={inactiveId ? "info" : "destructive"}>
              <TriangleAlertIcon strokeWidth={1.75} />
              <AlertDescription>
                <p>{problem}</p>
                {inactiveId && (
                  <Button
                    type="button"
                    size="sm"
                    variant="outline"
                    className="mt-2"
                    disabled={busy}
                    onClick={() => reactivate.mutate(inactiveId)}
                  >
                    {reactivate.isPending && <Spinner />}
                    Reativar categoria
                  </Button>
                )}
              </AlertDescription>
            </Alert>
          )}

          <DialogFooter className="-mx-6 -mb-6 mt-2 rounded-b-xl p-6 pt-4">
            <Button type="button" variant="outline" disabled={busy} onClick={onClose}>
              Cancelar
            </Button>
            <Button type="submit" disabled={busy}>
              {save.isPending && <Spinner />}
              {isRename ? "Salvar nome" : "Criar categoria"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
