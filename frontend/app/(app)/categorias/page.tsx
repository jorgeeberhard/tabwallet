import type { Metadata } from "next"

import { CategoriesScreen } from "@/components/categories/categories-screen"

export const metadata: Metadata = { title: "Categorias" }

export default function CategoriesPage() {
  return <CategoriesScreen />
}
