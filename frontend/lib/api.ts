/**
 * Calls the back-end through the Next.js `/api` rewrite (see next.config.ts),
 * so the browser only ever talks to the front-end's own origin and the
 * `httpOnly` session cookie travels with every request.
 */

/** An error answer from the API: `{"error": {"code", "message", "fields"?}}`. */
export class ApiError extends Error {
  constructor(
    readonly status: number,
    /** Stable machine-readable code, e.g. "email_taken". */
    readonly code: string,
    /** For validation errors: field name to rule code. */
    readonly fields: Record<string, string> = {},
  ) {
    super(code)
    this.name = "ApiError"
  }
}

/** The request never got an answer (offline, server down). */
export const NETWORK_ERROR_CODE = "network_error"

export async function api<T = void>(
  path: string,
  options: { method?: "GET" | "POST"; body?: unknown } = {},
): Promise<T> {
  const { method = "GET", body } = options

  let response: Response
  try {
    response = await fetch(`/api${path}`, {
      method,
      credentials: "same-origin",
      headers: body === undefined ? undefined : { "Content-Type": "application/json" },
      body: body === undefined ? undefined : JSON.stringify(body),
    })
  } catch {
    throw new ApiError(0, NETWORK_ERROR_CODE)
  }

  if (!response.ok) {
    const payload = await response.json().catch(() => null)
    throw new ApiError(
      response.status,
      payload?.error?.code ?? "unknown_error",
      payload?.error?.fields ?? {},
    )
  }
  if (response.status === 204) return undefined as T
  return (await response.json()) as T
}

export interface User {
  id: string
  email: string
  createdAt: string
}

interface UserPayload {
  user: { id: string; email: string; created_at: string }
}

function toUser({ user }: UserPayload): User {
  return { id: user.id, email: user.email, createdAt: user.created_at }
}

export async function signUp(email: string, password: string): Promise<User> {
  return toUser(
    await api<UserPayload>("/auth/signup", { method: "POST", body: { email, password } }),
  )
}

export async function logIn(email: string, password: string): Promise<User> {
  return toUser(
    await api<UserPayload>("/auth/login", { method: "POST", body: { email, password } }),
  )
}

export async function logOut(): Promise<void> {
  await api("/auth/logout", { method: "POST" })
}

export async function getCurrentUser(): Promise<User> {
  return toUser(await api<UserPayload>("/auth/me"))
}
