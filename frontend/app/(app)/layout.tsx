import { AppHeader } from "@/components/app-header"
import { BackButton } from "@/components/back-button"
import { ThemeToggle } from "@/components/theme-toggle"
import { UserMenu } from "@/components/user-menu"

/** Shell of every signed-in page. The proxy keeps visitors out of it. */
export default function AppLayout({ children }: LayoutProps<"/">) {
  return (
    <>
      <AppHeader
        right={
          <>
            <ThemeToggle />
            <UserMenu />
          </>
        }
      />
      <BackButton />
      <main className="mx-auto flex w-full max-w-[1200px] flex-1 flex-col gap-8 px-4 py-8 md:px-6">
        {children}
      </main>
    </>
  )
}
