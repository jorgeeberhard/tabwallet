"use client"

import {
  ChartLineIcon,
  ChartPieIcon,
  CreditCardIcon,
  HouseIcon,
  MenuIcon,
  PiggyBankIcon,
  ReceiptTextIcon,
  RepeatIcon,
  TagIcon,
} from "lucide-react"
import Link from "next/link"
import { usePathname } from "next/navigation"
import { type ComponentType, type ReactNode, useState } from "react"

import { useExceededBudgetCount } from "@/components/budgets/budget-alerts"
import { Logo } from "@/components/logo"
import { Button } from "@/components/ui/button"
import { Sheet, SheetContent, SheetHeader, SheetTitle, SheetTrigger } from "@/components/ui/sheet"
import { activeNavHref, NAV_ITEMS, type NavHref } from "@/lib/navigation"

const ICONS: Record<NavHref, ComponentType<{ className?: string; strokeWidth?: number }>> = {
  "/": HouseIcon,
  "/transacoes": ReceiptTextIcon,
  "/cartoes": CreditCardIcon,
  "/fixos": RepeatIcon,
  "/categorias": TagIcon,
  "/orcamentos": PiggyBankIcon,
  "/relatorios": ChartPieIcon,
  "/saldo": ChartLineIcon,
}

/** Number of budgets over their limit: the digit is for the eye, the sentence for screen readers. */
function OverLimitBadge({ count }: { count: number }) {
  return (
    <span className="ml-1.5 inline-flex min-w-5 items-center justify-center rounded-full bg-expense-soft px-1.5 text-xs font-semibold text-expense">
      <span aria-hidden="true">{count}</span>
      <span className="sr-only">{count} acima do limite</span>
    </span>
  )
}

/**
 * The top bar of every signed-in page (Design System `AppHeader`): the logo
 * (link to the home page), the eight sections of the app and `right` (the
 * theme and account menus). From `lg` up the sections sit in the bar; below
 * it they fold into a menu that opens as a side sheet.
 */
export function AppHeader({ right }: { right: ReactNode }) {
  const pathname = usePathname()
  const active = activeNavHref(pathname)
  const overLimit = useExceededBudgetCount()
  // The sheet is open for the page it was opened on, so moving to another page
  // (a link, or the browser's Back) closes it without an effect.
  const [openFor, setOpenFor] = useState<string | null>(null)
  const menuOpen = openFor === pathname

  return (
    <header className="sticky top-0 z-40 border-b bg-card">
      <div className="mx-auto flex h-16 w-full max-w-[1200px] items-center gap-2 px-4 md:px-6 lg:gap-4">
        <Sheet open={menuOpen} onOpenChange={(open) => setOpenFor(open ? pathname : null)}>
          <SheetTrigger asChild>
            <Button variant="ghost" size="icon" aria-label="Abrir menu" className="-ml-2 lg:hidden">
              <MenuIcon strokeWidth={1.75} />
            </Button>
          </SheetTrigger>
          <SheetContent side="left" aria-describedby={undefined} className="gap-2">
            <SheetHeader>
              <SheetTitle>Menu</SheetTitle>
            </SheetHeader>
            <nav aria-label="Menu principal" className="flex flex-col gap-1 px-2 pb-4">
              {NAV_ITEMS.map((item) => {
                const Icon = ICONS[item.href]
                return (
                  <Link
                    key={item.href}
                    href={item.href}
                    aria-current={active === item.href ? "page" : undefined}
                    onClick={() => setOpenFor(null)}
                    className="flex min-h-11 items-center gap-3 rounded-lg px-3 text-base font-medium text-foreground hover:bg-muted aria-[current=page]:bg-brand-soft aria-[current=page]:font-semibold aria-[current=page]:text-primary aria-[current=page]:shadow-[inset_3px_0_0_0_var(--primary)]"
                  >
                    <Icon strokeWidth={1.75} className="size-5 shrink-0" />
                    {item.label}
                    {item.href === "/orcamentos" && overLimit > 0 && <OverLimitBadge count={overLimit} />}
                  </Link>
                )
              })}
            </nav>
          </SheetContent>
        </Sheet>

        <Link href="/" className="inline-flex shrink-0 items-center rounded-md">
          <Logo height={28} />
        </Link>

        <nav aria-label="Principal" className="hidden min-w-0 flex-1 items-center gap-0.5 lg:flex">
          {NAV_ITEMS.map((item) => (
            <Link
              key={item.href}
              href={item.href}
              aria-current={active === item.href ? "page" : undefined}
              className="inline-flex h-10 items-center rounded-lg px-2.5 text-sm font-medium whitespace-nowrap text-muted-foreground hover:bg-muted hover:text-foreground aria-[current=page]:bg-brand-soft aria-[current=page]:text-primary aria-[current=page]:shadow-[inset_0_-2px_0_0_var(--primary)]"
            >
              {item.label}
              {item.href === "/orcamentos" && overLimit > 0 && <OverLimitBadge count={overLimit} />}
            </Link>
          ))}
        </nav>

        <div className="ml-auto flex items-center gap-1 lg:ml-0">{right}</div>
      </div>
    </header>
  )
}
