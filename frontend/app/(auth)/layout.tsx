import { Logo } from "@/components/logo"
import { ThemeToggle } from "@/components/theme-toggle"

/** Sign-up and login pages show only the logo (Design System, AppHeader). */
export default function AuthLayout({ children }: LayoutProps<"/">) {
  return (
    <>
      <header className="border-b bg-card">
        <div className="mx-auto flex h-16 w-full max-w-[1200px] items-center justify-between px-4 md:px-6">
          <Logo height={28} />
          <ThemeToggle />
        </div>
      </header>
      <main className="mx-auto flex w-full max-w-[420px] flex-1 flex-col justify-center gap-6 px-4 py-8 md:px-6">
        {children}
      </main>
    </>
  )
}
