import { ChartLineIcon, ReceiptTextIcon, RepeatIcon, TagIcon } from "lucide-react"
import Link from "next/link"

import { BalanceSection } from "@/components/balance/balance-section"
import { Welcome } from "@/components/welcome"

const SHORTCUT_CLASS =
  "inline-flex h-10 items-center gap-2 rounded-lg border border-input bg-card px-4 text-sm font-semibold hover:bg-muted"

export default function Home() {
  return (
    <section className="flex flex-col gap-6">
      <div className="flex flex-col gap-2">
        <h1 className="font-display text-3xl font-bold tracking-tight">
          Seu dinheiro, em uma aba.
        </h1>
        <Welcome />
      </div>
      <BalanceSection />
      <nav aria-label="Atalhos" className="flex flex-wrap gap-2">
        <Link href="/saldo" className={SHORTCUT_CLASS}>
          <ChartLineIcon strokeWidth={1.75} className="size-4" />
          Evolução do saldo
        </Link>
        <Link href="/transacoes" className={SHORTCUT_CLASS}>
          <ReceiptTextIcon strokeWidth={1.75} className="size-4" />
          Transações
        </Link>
        <Link href="/fixos" className={SHORTCUT_CLASS}>
          <RepeatIcon strokeWidth={1.75} className="size-4" />
          Gastos e receitas fixos
        </Link>
        <Link href="/categorias" className={SHORTCUT_CLASS}>
          <TagIcon strokeWidth={1.75} className="size-4" />
          Categorias
        </Link>
      </nav>
    </section>
  )
}
