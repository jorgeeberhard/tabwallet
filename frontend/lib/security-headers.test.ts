import { describe, expect, it } from "vitest"

import { securityHeaders } from "@/lib/security-headers"

function header(headers: ReturnType<typeof securityHeaders>, key: string) {
  return headers.find((h) => h.key.toLowerCase() === key.toLowerCase())?.value
}

describe("securityHeaders", () => {
  it("forbids framing the app and sniffing content types", () => {
    const headers = securityHeaders({ production: true })

    expect(header(headers, "X-Content-Type-Options")).toBe("nosniff")
    expect(header(headers, "X-Frame-Options")).toBe("DENY")
    expect(header(headers, "Content-Security-Policy")).toContain("frame-ancestors 'none'")
    expect(header(headers, "Referrer-Policy")).toBe("strict-origin-when-cross-origin")
  })

  it("restricts every content source to the app's own origin", () => {
    const csp = header(securityHeaders({ production: true }), "Content-Security-Policy")!

    expect(csp).toContain("default-src 'self'")
    expect(csp).toContain("connect-src 'self'")
    expect(csp).toContain("object-src 'none'")
    expect(csp).toContain("base-uri 'self'")
    expect(csp).toContain("form-action 'self'")
  })

  it("lets production run no eval and no script from other origins", () => {
    const csp = header(securityHeaders({ production: true }), "Content-Security-Policy")!
    const scripts = csp.split(";").find((d) => d.trim().startsWith("script-src"))!

    expect(scripts).not.toContain("unsafe-eval")
    expect(scripts).not.toMatch(/https?:/)
  })

  it("allows eval only in development, where React needs it", () => {
    const dev = header(securityHeaders({ production: false }), "Content-Security-Policy")!
    const production = header(securityHeaders({ production: true }), "Content-Security-Policy")!

    expect(dev).toContain("'unsafe-eval'")
    expect(production).not.toContain("'unsafe-eval'")
  })

  it("asks for HTTPS (HSTS and upgrading requests) only in production", () => {
    const production = securityHeaders({ production: true })
    const dev = securityHeaders({ production: false })

    expect(header(production, "Strict-Transport-Security")).toBe(
      "max-age=63072000; includeSubDomains",
    )
    expect(header(production, "Content-Security-Policy")).toContain("upgrade-insecure-requests")
    expect(header(dev, "Strict-Transport-Security")).toBeUndefined()
    expect(header(dev, "Content-Security-Policy")).not.toContain("upgrade-insecure-requests")
  })

  it("turns off browser features the app does not use", () => {
    const policy = header(securityHeaders({ production: true }), "Permissions-Policy")!

    for (const feature of ["camera", "microphone", "geolocation"]) {
      expect(policy).toContain(`${feature}=()`)
    }
  })
})
