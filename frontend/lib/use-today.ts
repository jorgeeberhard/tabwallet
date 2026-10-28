"use client"

import { useSyncExternalStore } from "react"

import { todayInSaoPaulo } from "@/lib/today"

const subscribe = () => () => {}

/**
 * Today's calendar day in America/Sao_Paulo, or `null` while the page is
 * being prerendered on the server: the current time cannot be read there
 * (Next.js would freeze it into the prerender), so callers show a loading
 * state until the browser has taken over.
 */
export function useToday(): string | null {
  return useSyncExternalStore(subscribe, () => todayInSaoPaulo(), () => null)
}
