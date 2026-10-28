/** What the proxy found out about the request's session cookie. */
export type SessionState =
  /** No session cookie. */
  | "none"
  /** The API accepted the cookie. */
  | "valid"
  /** The API rejected the cookie (expired, forged, user gone). */
  | "invalid"
  /** The API could not be asked (down, timeout). */
  | "unknown"

const PUBLIC_PAGES = ["/login", "/signup"]

/**
 * Where to send the request, or `null` to let it through.
 *
 * Visitors only see the login and sign-up pages; a signed-in user is sent
 * home from them. When the API cannot be asked ("unknown") private pages stay
 * closed, but the public ones open so the person is not stuck in a redirect.
 */
export function decideAccess(
  pathname: string,
  session: SessionState,
): { redirectTo: string } | null {
  const isPublic = PUBLIC_PAGES.includes(pathname)
  if (session === "valid") return isPublic ? { redirectTo: "/" } : null
  return isPublic ? null : { redirectTo: "/login" }
}
