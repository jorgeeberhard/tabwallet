import { useSyncExternalStore } from "react"

const subscribe = () => () => {}

/**
 * False while the page is rendered on the server and during hydration, true
 * afterwards. Use it to keep the first client render equal to the server HTML
 * when the data may already be in the query cache (a mismatch makes React throw
 * the server HTML away and warn).
 */
export function useHydrated(): boolean {
  return useSyncExternalStore(
    subscribe,
    () => true,
    () => false,
  )
}
