import type { NextConfig } from "next";

import { securityHeaders } from "./lib/security-headers";

// Where the Rust API listens. The browser never calls it directly: it calls
// `/api/*` on this app, which forwards to the API. One origin means the
// `httpOnly` session cookie is first-party (SameSite=Lax works) and the API
// needs no CORS. Set API_URL in production (read at build time).
const API_URL = process.env.API_URL ?? "http://127.0.0.1:8080";

const nextConfig: NextConfig = {
  /* config options here */
  cacheComponents: true,
  partialPrefetching: true,
  async headers() {
    return [
      {
        source: "/:path*",
        headers: securityHeaders({ production: process.env.NODE_ENV === "production" }),
      },
    ]
  },
  async rewrites() {
    return [{ source: "/api/:path*", destination: `${API_URL}/:path*` }];
  },
  turbopack: {
    rules: {
      "*.css": {
        loaders: ["@tailwindcss/turbopack"],
        as: "*.css",
      },
    },
  },
};

export default nextConfig;
