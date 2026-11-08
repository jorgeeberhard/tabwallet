import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { act } from "@testing-library/react"
import { hydrateRoot } from "react-dom/client"
import { renderToString } from "react-dom/server"
import { afterEach, describe, expect, it, vi } from "vitest"

import { Welcome } from "@/components/welcome"

const user = { id: "u1", email: "ana@example.com", name: "Ana", createdAt: "2026-01-01" }

function app(client: QueryClient) {
  return (
    <QueryClientProvider client={client}>
      <Welcome />
    </QueryClientProvider>
  )
}

describe("Welcome", () => {
  afterEach(() => {
    vi.restoreAllMocks()
    document.body.innerHTML = ""
  })

  it("hydrates without a mismatch even when the user is already in the query cache", async () => {
    // The server renders with an empty cache (the skeleton). In the browser the
    // header's account menu may have fetched the user before this part hydrates.
    const html = renderToString(app(new QueryClient()))
    const warm = new QueryClient()
    warm.setQueryData(["current-user"], user)
    const errors = vi.spyOn(console, "error").mockImplementation(() => {})
    const container = document.createElement("div")
    container.innerHTML = html
    document.body.appendChild(container)

    await act(async () => {
      hydrateRoot(container, app(warm), { onRecoverableError: (error) => console.error(error) })
    })

    expect(errors).not.toHaveBeenCalled()
    expect(container).toHaveTextContent("Você entrou como Ana")
  })
})
