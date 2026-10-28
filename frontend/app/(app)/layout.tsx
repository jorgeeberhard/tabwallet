import { Logo } from "@/components/logo"
import { ThemeToggle } from "@/components/theme-toggle"
import { UserMenu } from "@/components/user-menu"

/** Shell of every signed-in page. The proxy keeps visitors out of it. */
export default function AppLayout({ children }: LayoutProps<"/">) {
  return (
    <>
      <header className="border-b bg-card">
        <div className="mx-auto flex h-16 w-full max-w-[1200px] items-center justify-between px-4 md:px-6">
          <Logo height={28} />
          <div className="flex items-center gap-1">
            <ThemeToggle />
            <UserMenu />
          </div>
        </div>
      </header>
      <main className="mx-auto flex w-full max-w-[1200px] flex-1 flex-col gap-8 px-4 py-8 md:px-6">
        {children}
      </main>
    </>
  )
}
