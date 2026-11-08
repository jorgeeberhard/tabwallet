/**
 * Security headers sent with every response of the front-end (wired in
 * next.config.ts).
 *
 * The CSP has no nonce: nonces need fully dynamic rendering and are
 * incompatible with the partial prerendering this app uses, so scripts and
 * styles allow 'unsafe-inline' (Next.js injects inline bootstrap scripts and
 * styles). The rest of the policy still holds: everything loads from the app's
 * own origin, no plugins, no framing, no form posts elsewhere. The main XSS
 * defence is that user text is only ever rendered through React (escaped),
 * never as HTML, and that the session cookie is httpOnly.
 */

export interface Header {
  key: string
  value: string
}

/** `production` is false for `next dev`, where React needs eval. */
export function securityHeaders({ production }: { production: boolean }): Header[] {
  const csp = [
    "default-src 'self'",
    `script-src 'self' 'unsafe-inline'${production ? "" : " 'unsafe-eval'"}`,
    "style-src 'self' 'unsafe-inline'",
    "img-src 'self' blob: data:",
    "font-src 'self'",
    "connect-src 'self'",
    "object-src 'none'",
    "base-uri 'self'",
    "form-action 'self'",
    "frame-ancestors 'none'",
    ...(production ? ["upgrade-insecure-requests"] : []),
  ].join("; ")

  const headers: Header[] = [
    { key: "Content-Security-Policy", value: csp },
    { key: "X-Content-Type-Options", value: "nosniff" },
    // Older browsers ignore frame-ancestors.
    { key: "X-Frame-Options", value: "DENY" },
    { key: "Referrer-Policy", value: "strict-origin-when-cross-origin" },
    { key: "Permissions-Policy", value: "camera=(), microphone=(), geolocation=()" },
  ]
  if (production) {
    // HTTPS only, for two years, subdomains included.
    headers.push({
      key: "Strict-Transport-Security",
      value: "max-age=63072000; includeSubDomains",
    })
  }
  return headers
}
