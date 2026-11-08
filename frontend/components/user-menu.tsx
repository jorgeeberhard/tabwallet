"use client"

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { LogOutIcon, SettingsIcon, UserRoundIcon } from "lucide-react"
import Link from "next/link"
import { useRouter } from "next/navigation"
import { toast } from "sonner"

import { Button } from "@/components/ui/button"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { getCurrentUser, logOut } from "@/lib/api"
import { useHydrated } from "@/lib/use-hydrated"

/** Shared by the header and the home page so the user is fetched once. */
export function useCurrentUser() {
  const query = useQuery({ queryKey: ["current-user"], queryFn: getCurrentUser })
  // The account menu in the header may fetch the user before the page below it
  // hydrates; showing the data only after hydration keeps both renders equal.
  const hydrated = useHydrated()
  return { data: hydrated ? query.data : undefined }
}

/** Account menu with the signed-in e-mail and the logout action. */
export function UserMenu() {
  const router = useRouter()
  const queryClient = useQueryClient()
  const { data: user } = useCurrentUser()
  const logout = useMutation({
    mutationFn: logOut,
    onSuccess: () => {
      // Nothing cached may outlive the session.
      queryClient.clear()
      router.replace("/login")
      router.refresh()
    },
    onError: () => toast.error("Não foi possível sair. Verifique a conexão e tente de novo."),
  })

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="icon" aria-label="Menu da conta">
          <UserRoundIcon strokeWidth={1.75} />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="min-w-56">
        {user && (
          <>
            <DropdownMenuLabel className="font-normal text-muted-foreground">
              {user.name || user.email}
            </DropdownMenuLabel>
            <DropdownMenuSeparator />
          </>
        )}
        <DropdownMenuItem asChild>
          <Link href="/profile">
            <SettingsIcon strokeWidth={1.75} />
            Perfil
          </Link>
        </DropdownMenuItem>
        <DropdownMenuItem disabled={logout.isPending} onSelect={() => logout.mutate()}>
          <LogOutIcon strokeWidth={1.75} />
          Sair
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
