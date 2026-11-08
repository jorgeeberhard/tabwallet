import { api } from "@/lib/api"

export type CategoryType = "income" | "expense"

export interface Category {
  id: string
  name: string
  type: CategoryType
  active: boolean
  /** The default "Juros e encargos" category, where Charges are recorded. */
  isCharges: boolean
}

interface CategoryPayload {
  id: string
  name: string
  type: CategoryType
  active: boolean
  is_charges: boolean
}

function toCategory(payload: CategoryPayload): Category {
  return {
    id: payload.id,
    name: payload.name,
    type: payload.type,
    active: payload.active,
    isCharges: payload.is_charges,
  }
}

/** Query key of the full list (active and inactive); mutations invalidate it. */
export const CATEGORIES_KEY = ["categories"] as const

export async function listCategories(): Promise<Category[]> {
  const { categories } = await api<{ categories: CategoryPayload[] }>("/categories")
  return categories.map(toCategory)
}

export async function createCategory(name: string, type: CategoryType): Promise<Category> {
  const { category } = await api<{ category: CategoryPayload }>("/categories", {
    method: "POST",
    body: { name, type },
  })
  return toCategory(category)
}

export async function renameCategory(id: string, name: string): Promise<Category> {
  const { category } = await api<{ category: CategoryPayload }>(`/categories/${id}`, {
    method: "PATCH",
    body: { name },
  })
  return toCategory(category)
}

export async function deactivateCategory(id: string): Promise<Category> {
  const { category } = await api<{ category: CategoryPayload }>(`/categories/${id}/deactivate`, {
    method: "POST",
  })
  return toCategory(category)
}

export async function reactivateCategory(id: string): Promise<Category> {
  const { category } = await api<{ category: CategoryPayload }>(`/categories/${id}/reactivate`, {
    method: "POST",
  })
  return toCategory(category)
}
