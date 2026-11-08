"use client"

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { PencilIcon, PlusIcon, TagIcon, TriangleAlertIcon } from "lucide-react"
import { useState } from "react"
import { toast } from "sonner"

import {
  CategoryDialog,
  type CategoryDialogMode,
} from "@/components/categories/category-dialog"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card } from "@/components/ui/card"
import { Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle } from "@/components/ui/empty"
import { Skeleton } from "@/components/ui/skeleton"
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group"
import {
  CATEGORIES_KEY,
  type Category,
  type CategoryType,
  deactivateCategory,
  listCategories,
  reactivateCategory,
} from "@/lib/categories-api"
import { categoryFailure } from "@/lib/category-messages"

const TYPE_LABEL: Record<CategoryType, string> = { expense: "Despesas", income: "Receitas" }

/** UC09: manage income and expense categories. */
export function CategoriesScreen() {
  const queryClient = useQueryClient()
  const [type, setType] = useState<CategoryType>("expense")
  const [dialog, setDialog] = useState<CategoryDialogMode | null>(null)
  const { data, isPending, isError, refetch } = useQuery({
    queryKey: CATEGORIES_KEY,
    queryFn: listCategories,
  })

  const deactivate = useMutation({
    mutationFn: (category: Category) => deactivateCategory(category.id),
    onSuccess: (category) => {
      void queryClient.invalidateQueries({ queryKey: CATEGORIES_KEY })
      toast.success(`${category.name} foi desativada. Ela continua nas transações antigas.`)
    },
    onError: (error) => toast.error(categoryFailure(error, "desativar a categoria")),
  })
  const reactivate = useMutation({
    mutationFn: (category: Category) => reactivateCategory(category.id),
    onSuccess: (category) => {
      void queryClient.invalidateQueries({ queryKey: CATEGORIES_KEY })
      toast.success(`${category.name} foi reativada.`)
    },
    onError: (error) => toast.error(categoryFailure(error, "reativar a categoria")),
  })

  const ofType = (data ?? []).filter((category) => category.type === type)
  const active = ofType.filter((category) => category.active)
  const inactive = ofType.filter((category) => !category.active)

  return (
    <section className="flex flex-col gap-6">
      <div className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
        <div className="flex flex-col gap-1">
          <h1 className="font-display text-3xl font-bold tracking-tight">Categorias</h1>
          <p className="max-w-prose text-muted-foreground">
            Organize suas receitas e despesas. Uma categoria desativada some dos novos lançamentos,
            mas continua nas transações antigas.
          </p>
        </div>
        <Button onClick={() => setDialog({ kind: "create", type })}>
          <PlusIcon strokeWidth={1.75} />
          Nova categoria
        </Button>
      </div>

      <ToggleGroup
        type="single"
        aria-label="Tipo de categoria"
        value={type}
        onValueChange={(value) => value && setType(value as CategoryType)}
        spacing={0}
        className="w-full max-w-sm rounded-lg bg-muted p-1"
      >
        {(Object.keys(TYPE_LABEL) as CategoryType[]).map((value) => (
          <ToggleGroupItem
            key={value}
            value={value}
            className="h-9 flex-1 data-[state=on]:bg-card data-[state=on]:shadow-sm"
          >
            {TYPE_LABEL[value]}
          </ToggleGroupItem>
        ))}
      </ToggleGroup>

      {isError ? (
        <Alert variant="destructive">
          <TriangleAlertIcon strokeWidth={1.75} />
          <AlertDescription>
            <p>Não foi possível carregar as categorias. Verifique a conexão e tente de novo.</p>
            <Button size="sm" variant="outline" className="mt-2" onClick={() => void refetch()}>
              Tentar de novo
            </Button>
          </AlertDescription>
        </Alert>
      ) : isPending ? (
        <div className="flex flex-col gap-2" aria-label="Carregando categorias">
          <Skeleton className="h-14 w-full" />
          <Skeleton className="h-14 w-full" />
          <Skeleton className="h-14 w-full" />
        </div>
      ) : (
        <>
          {active.length === 0 ? (
            <Empty className="border">
              <EmptyHeader>
                <EmptyMedia variant="icon">
                  <TagIcon strokeWidth={1.75} />
                </EmptyMedia>
                <EmptyTitle>Nenhuma categoria ativa</EmptyTitle>
                <EmptyDescription>
                  Crie uma categoria para classificar suas{" "}
                  {type === "expense" ? "despesas" : "receitas"}.
                </EmptyDescription>
              </EmptyHeader>
              <Button onClick={() => setDialog({ kind: "create", type })}>Nova categoria</Button>
            </Empty>
          ) : (
            <Card className="gap-0 py-0">
              <ul aria-label={`Categorias de ${TYPE_LABEL[type].toLowerCase()}`} className="divide-y">
                {active.map((category) => (
                  <li key={category.id} className="flex items-center gap-3 px-4 py-3">
                    <TagIcon strokeWidth={1.75} className="size-4 shrink-0 text-muted-foreground" />
                    <div className="flex min-w-0 flex-1 flex-col">
                      <span className="truncate font-medium">{category.name}</span>
                      {category.isCharges && (
                        <span className="text-xs text-muted-foreground">
                          Recebe os juros, multas e IOF das faturas.
                        </span>
                      )}
                    </div>
                    <Button
                      variant="ghost"
                      size="icon"
                      aria-label={`Renomear ${category.name}`}
                      onClick={() => setDialog({ kind: "rename", category })}
                    >
                      <PencilIcon strokeWidth={1.75} />
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      aria-label={`Desativar ${category.name}`}
                      disabled={deactivate.isPending}
                      onClick={() => deactivate.mutate(category)}
                    >
                      Desativar
                    </Button>
                  </li>
                ))}
              </ul>
            </Card>
          )}

          {inactive.length > 0 && (
            <div className="flex flex-col gap-3">
              <h2 className="font-display text-xl font-bold tracking-tight">Desativadas</h2>
              <Card className="gap-0 py-0">
                <ul aria-label="Categorias desativadas" className="divide-y">
                  {inactive.map((category) => (
                    <li key={category.id} className="flex items-center gap-3 px-4 py-3">
                      <TagIcon strokeWidth={1.75} className="size-4 shrink-0 text-muted-foreground" />
                      <span className="min-w-0 flex-1 truncate text-muted-foreground">
                        {category.name}
                      </span>
                      <Badge variant="secondary">Desativada</Badge>
                      <Button
                        variant="outline"
                        size="sm"
                        aria-label={`Reativar ${category.name}`}
                        disabled={reactivate.isPending}
                        onClick={() => reactivate.mutate(category)}
                      >
                        Reativar
                      </Button>
                    </li>
                  ))}
                </ul>
              </Card>
            </div>
          )}
        </>
      )}

      {dialog && (
        <CategoryDialog
          // A new key per target resets the form state.
          key={dialog.kind === "rename" ? dialog.category.id : "create"}
          mode={dialog}
          onClose={() => setDialog(null)}
          onSaved={setType}
        />
      )}
    </section>
  )
}
