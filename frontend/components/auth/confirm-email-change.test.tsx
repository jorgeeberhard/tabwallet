import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { render, screen } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { ConfirmEmailChange } from "@/components/auth/confirm-email-change"

function answer(status: number, body?: unknown) {
  return Promise.resolve(
    new Response(body === undefined ? null : JSON.stringify(body), {
      status,
      headers: { "Content-Type": "application/json" },
    }),
  )
}

const apiError = (code: string) => ({ error: { code, message: code } })

function renderPage(token: string) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  const invalidate = vi.spyOn(client, "invalidateQueries")
  render(
    <QueryClientProvider client={client}>
      <ConfirmEmailChange token={token} />
    </QueryClientProvider>,
  )
  return { invalidate }
}

describe("ConfirmEmailChange", () => {
  const fetchMock = vi.fn()

  beforeEach(() => {
    vi.stubGlobal("fetch", fetchMock)
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    fetchMock.mockReset()
  })

  it("confirms the link once, says the old e-mail was told and refreshes the signed-in user", async () => {
    fetchMock.mockReturnValue(answer(204))
    const { invalidate } = renderPage("abc123")

    expect(await screen.findByText("E-mail alterado")).toBeInTheDocument()
    expect(screen.getByText(/entre com o novo e-mail e a mesma senha/)).toBeInTheDocument()
    expect(screen.getByText(/Enviamos um aviso ao e-mail antigo/)).toBeInTheDocument()
    expect(screen.getByRole("link", { name: "Entrar" })).toHaveAttribute("href", "/login")
    expect(fetchMock).toHaveBeenCalledTimes(1)
    const [url, init] = fetchMock.mock.calls[0]
    expect(url).toBe("/api/auth/email-change/confirm")
    expect(init.method).toBe("POST")
    expect(JSON.parse(init.body)).toEqual({ token: "abc123" })
    expect(invalidate).toHaveBeenCalledWith({ queryKey: ["current-user"] })
  })

  it("shows that it is confirming while the API answers", () => {
    fetchMock.mockReturnValue(new Promise(() => {}))
    renderPage("abc123")

    expect(screen.getByText(/Confirmando o novo e-mail/)).toBeInTheDocument()
  })

  it("explains an expired, used or replaced link and points back to the profile", async () => {
    fetchMock.mockReturnValue(answer(400, apiError("invalid_confirmation_token")))
    renderPage("old")

    expect(await screen.findByText("Este link não vale mais")).toBeInTheDocument()
    expect(screen.getByText(/reenviar o link/)).toBeInTheDocument()
    expect(screen.getByRole("link", { name: "Ir para o perfil" })).toHaveAttribute(
      "href",
      "/profile",
    )
  })

  it("treats a link without a token like an invalid one, without calling the API", () => {
    renderPage("")

    expect(screen.getByText("Este link não vale mais")).toBeInTheDocument()
    expect(fetchMock).not.toHaveBeenCalled()
  })

  it("refuses with an explanation when the new e-mail became someone else's account", async () => {
    fetchMock.mockReturnValue(answer(409, apiError("email_taken")))
    const { invalidate } = renderPage("abc123")

    expect(await screen.findByText("Este e-mail já tem uma conta")).toBeInTheDocument()
    expect(screen.getByText(/Você continua entrando com o e-mail de antes/)).toBeInTheDocument()
    expect(screen.getByRole("link", { name: "Ir para o perfil" })).toHaveAttribute(
      "href",
      "/profile",
    )
    expect(invalidate).not.toHaveBeenCalled()
  })

  it("lets the person try again when the API cannot be reached", async () => {
    fetchMock.mockRejectedValueOnce(new TypeError("Failed to fetch"))
    fetchMock.mockReturnValueOnce(answer(204))
    renderPage("abc123")

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Não foi possível confirmar a troca de e-mail. Verifique a conexão e tente de novo.",
    )
    await userEvent.click(screen.getByRole("button", { name: "Tentar de novo" }))

    expect(await screen.findByText("E-mail alterado")).toBeInTheDocument()
  })
})
