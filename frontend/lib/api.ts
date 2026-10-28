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
    /** For "category_inactive_exists": the deactivated category with that name. */
    readonly categoryId?: string,
  ) {
    super(code)
    this.name = "ApiError"
  }
}

/** The request never got an answer (offline, server down). */
export const NETWORK_ERROR_CODE = "network_error"

export async function api<T = void>(
  path: string,
  options: { method?: "GET" | "POST" | "PATCH" | "DELETE"; body?: unknown } = {},
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
      payload?.error?.category_id,
    )
  }
  // 204 and 202 answers carry no body.
  if (response.status === 204 || response.status === 202) return undefined as T
  return (await response.json()) as T
}

export interface User {
  id: string
  email: string
  /** Empty when the person never set one. */
  name: string
  createdAt: string
}

interface UserPayload {
  user: { id: string; email: string; name: string; created_at: string }
}

function toUser({ user }: UserPayload): User {
  return { id: user.id, email: user.email, name: user.name, createdAt: user.created_at }
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

/** What the profile form may change. Only the fields present are sent. */
export interface ProfileChanges {
  name?: string
  email?: string
  newPassword?: string
  /** Required by the API to change the e-mail or the password. */
  currentPassword?: string
}

export async function updateProfile(changes: ProfileChanges): Promise<User> {
  return toUser(
    await api<UserPayload>("/profile", {
      method: "PATCH",
      body: {
        name: changes.name,
        email: changes.email,
        new_password: changes.newPassword,
        current_password: changes.currentPassword,
      },
    }),
  )
}

/** Asks for a reset link. The API answers the same for any address. */
export async function requestPasswordReset(email: string): Promise<void> {
  await api("/auth/password-reset/request", { method: "POST", body: { email } })
}

export async function confirmPasswordReset(token: string, password: string): Promise<void> {
  await api("/auth/password-reset/confirm", { method: "POST", body: { token, password } })
}
