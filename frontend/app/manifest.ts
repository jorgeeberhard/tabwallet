import type { MetadataRoute } from "next"

// Manifest values cannot use CSS variables: the colours below are the light
// theme's tokens (`--background` for the splash, `--card` for the browser bar).
export default function manifest(): MetadataRoute.Manifest {
  return {
    name: "TabWallet",
    short_name: "TabWallet",
    description: "Seu dinheiro, em uma aba.",
    lang: "pt-BR",
    start_url: "/",
    display: "standalone",
    background_color: "#f4f5f8",
    theme_color: "#ffffff",
    icons: [
      { src: "/icons/icon-192.png", sizes: "192x192", type: "image/png" },
      { src: "/icons/icon-512.png", sizes: "512x512", type: "image/png" },
    ],
  }
}
