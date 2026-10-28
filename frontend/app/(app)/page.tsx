import { TagIcon } from "lucide-react"
import Link from "next/link"

import { Welcome } from "@/components/welcome"

export default function Home() {
  return (
    <section className="flex flex-col gap-6">
      <div className="flex flex-col gap-2">
        <h1 className="font-display text-3xl font-bold tracking-tight">
          Seu dinheiro, em uma aba.
        </h1>
        <Welcome />
      </div>
      <nav aria-label="Atalhos" className="flex flex-wrap gap-2">
        <Link
          href="/categorias"
          className="inline-flex h-10 items-center gap-2 rounded-lg border border-input bg-card px-4 text-sm font-semibold hover:bg-muted"
        >
          <TagIcon strokeWidth={1.75} className="size-4" />
          Categorias
        </Link>
      </nav>
    </section>
  )
}
