import { render, screen } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { ForgotPasswordForm } from "@/components/auth/forgot-password-form"

describe("ForgotPasswordForm", () => {
  const fetchMock = vi.fn()

  beforeEach(() => {
    vi.stubGlobal("fetch", fetchMock)
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    fetchMock.mockReset()
  })

  it("asks for a valid e-mail before calling the API", async () => {
    render(<ForgotPasswordForm />)

    await userEvent.type(screen.getByLabelText("E-mail"), "not-an-email")
    await userEvent.click(screen.getByRole("button", { name: "Enviar link" }))

    expect(
      await screen.findByText("Digite um e-mail válido, como nome@dominio.com."),
    ).toBeInTheDocument()
    expect(fetchMock).not.toHaveBeenCalled()
  })

  it("asks for the link and says only that one is sent if the account exists", async () => {
    fetchMock.mockResolvedValue(new Response(null, { status: 202 }))
    render(<ForgotPasswordForm />)

    await userEvent.type(screen.getByLabelText("E-mail"), "ana@example.com")
    await userEvent.click(screen.getByRole("button", { name: "Enviar link" }))

    expect(await screen.findByText(/Se existir uma conta com ana@example.com/)).toBeInTheDocument()
    const [url, init] = fetchMock.mock.calls[0]
    expect(url).toBe("/api/auth/password-reset/request")
    expect(JSON.parse(init.body)).toEqual({ email: "ana@example.com" })
  })

  it("asks to check the connection when the API cannot be reached", async () => {
    fetchMock.mockRejectedValue(new TypeError("Failed to fetch"))
    render(<ForgotPasswordForm />)

    await userEvent.type(screen.getByLabelText("E-mail"), "ana@example.com")
    await userEvent.click(screen.getByRole("button", { name: "Enviar link" }))

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Não foi possível enviar o link. Verifique a conexão e tente de novo.",
    )
  })
})
