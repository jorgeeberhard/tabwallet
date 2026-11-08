import { describe, expect, it } from "vitest"

import { decideAccess } from "@/lib/access"

describe("decideAccess", () => {
  it("sends a visitor without a session from a private page to the login", () => {
    expect(decideAccess("/", "none")).toEqual({ redirectTo: "/login" })
    expect(decideAccess("/transacoes", "none")).toEqual({ redirectTo: "/login" })
  })

  it("sends a stale or rejected session to the login too", () => {
    expect(decideAccess("/", "invalid")).toEqual({ redirectTo: "/login" })
    expect(decideAccess("/", "unknown")).toEqual({ redirectTo: "/login" })
  })

  it("lets a visitor open the login and sign-up pages", () => {
    expect(decideAccess("/login", "none")).toBeNull()
    expect(decideAccess("/signup", "invalid")).toBeNull()
    expect(decideAccess("/login", "unknown")).toBeNull()
  })

  it("opens the password-recovery pages to visitors", () => {
    expect(decideAccess("/forgot-password", "none")).toBeNull()
    expect(decideAccess("/reset-password", "none")).toBeNull()
    expect(decideAccess("/reset-password", "invalid")).toBeNull()
  })

  it("lets anyone holding a reset link use it, even when signed in", () => {
    expect(decideAccess("/reset-password", "valid")).toBeNull()
    expect(decideAccess("/forgot-password", "valid")).toEqual({ redirectTo: "/" })
  })

  it("lets a signed-in user into private pages", () => {
    expect(decideAccess("/", "valid")).toBeNull()
  })

  it("sends a signed-in user away from the login and sign-up pages", () => {
    expect(decideAccess("/login", "valid")).toEqual({ redirectTo: "/" })
    expect(decideAccess("/signup", "valid")).toEqual({ redirectTo: "/" })
  })
})
