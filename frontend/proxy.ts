import { type NextRequest, NextResponse } from "next/server"

import { decideAccess, type SessionState } from "@/lib/access"

/** Name of the session cookie set by the API (backend/src/auth/mod.rs). */
const SESSION_COOKIE = "tabwallet_session"

// Server-side address of the API, also the target of the `/api` rewrite in
// next.config.ts.
const API_URL = process.env.API_URL ?? "http://127.0.0.1:8080"

/** Asks the API whether the request's session cookie is still good. */
async function checkSession(request: NextRequest): Promise<SessionState> {
  const cookie = request.cookies.get(SESSION_COOKIE)
  if (!cookie) return "none"
  try {
    const response = await fetch(`${API_URL}/auth/me`, {
      headers: { cookie: `${SESSION_COOKIE}=${cookie.value}` },
      cache: "no-store",
      signal: AbortSignal.timeout(3000),
    })
    if (response.ok) return "valid"
    return response.status === 401 ? "invalid" : "unknown"
  } catch {
    return "unknown"
  }
}

export async function proxy(request: NextRequest) {
  const session = await checkSession(request)
  const decision = decideAccess(request.nextUrl.pathname, session)

  if (!decision) return NextResponse.next()

  const response = NextResponse.redirect(new URL(decision.redirectTo, request.url))
  // Drop a cookie the API rejected, so it is not sent again.
  if (session === "invalid") response.cookies.delete(SESSION_COOKIE)
  return response
}

export const config = {
  // Pages only: not the API rewrite, Next.js internals or static files.
  matcher: ["/((?!api/|_next/|favicon.ico|.*\\.[a-zA-Z0-9]+$).*)"],
}
