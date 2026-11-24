/** What the browser tells us about the history of this tab. */
export interface HistoryEnvironment {
  /** The Navigation API, where the browser has it (`canGoBack` only counts same-origin entries). */
  navigation?: { canGoBack: boolean }
  /** `document.referrer`: where the visitor was when this document was opened. */
  referrer: string
  /** `location.origin` of the site. */
  origin: string
  /** True once the visitor moved to another page of the app without reloading. */
  navigatedInApp: boolean
}

/**
 * Whether the browser's Back button would stay inside the site. `history.length`
 * cannot tell: it also counts the pages the visitor saw before arriving. The
 * Navigation API answers exactly this; without it, the app's own page changes
 * and a same-origin referrer are the evidence.
 */
export function hasPreviousPageInSite(env: HistoryEnvironment): boolean {
  if (typeof env.navigation?.canGoBack === "boolean") return env.navigation.canGoBack
  if (env.navigatedInApp) return true
  return env.referrer.startsWith(`${env.origin}/`)
}
