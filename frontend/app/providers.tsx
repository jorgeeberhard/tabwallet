"use client"

import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { ThemeProvider } from "next-themes"
import type { ReactNode } from "react"

import { Toaster } from "@/components/ui/sonner"
import { TooltipProvider } from "@/components/ui/tooltip"

let browserQueryClient: QueryClient | undefined

function makeQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: {
        // Avoid refetching right after a server-rendered or just-fetched result.
        staleTime: 60 * 1000,
      },
    },
  })
}

function getQueryClient() {
  // On the server, never share a client between requests. In the browser,
  // reuse one client so the cache survives re-renders.
  if (typeof window === "undefined") return makeQueryClient()
  browserQueryClient ??= makeQueryClient()
  return browserQueryClient
}

export function Providers({ children }: { children: ReactNode }) {
  return (
    <ThemeProvider
      attribute="class"
      defaultTheme="system"
      enableSystem
      disableTransitionOnChange
    >
      <QueryClientProvider client={getQueryClient()}>
        <TooltipProvider>
          {children}
          <Toaster />
        </TooltipProvider>
      </QueryClientProvider>
    </ThemeProvider>
  )
}
