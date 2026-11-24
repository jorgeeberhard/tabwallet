import { Suspense } from "react"

import { AppHeader } from "@/components/app-header"
import { BackButton } from "@/components/back-button"
import { ThemeToggle } from "@/components/theme-toggle"
import { UserMenu } from "@/components/user-menu"

/** Shell of every signed-in page. The proxy keeps visitors out of it. */
export default function AppLayout({ children }: LayoutProps<"/">) {
  return (
    <>
      {/* The current page comes from the URL, which a dynamic route (a card's
          invoices) only knows at request time. The fallbacks hold the same
          height as the real ones, so nothing moves when those arrive. */}
      <Suspense fallback={<header className="sticky top-0 z-40 h-[65px] border-b bg-card" />}>
        <AppHeader
          right={
            <>
              <ThemeToggle />
              <UserMenu />
            </>
          }
        />
      </Suspense>
      <Suspense fallback={<div aria-hidden="true" className="h-14" />}>
        <BackButton />
      </Suspense>
      <main className="mx-auto flex w-full max-w-[1200px] flex-1 flex-col gap-8 px-4 py-8 md:px-6">
        {children}
      </main>
    </>
  )
}
