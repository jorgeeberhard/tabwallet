import { render, screen } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { ConfirmEmail } from "@/components/auth/confirm-email"

function answer(status: number, body?: unknown) {
  return Promise.resolve(
    new Response(body === undefined ? null : JSON.stringify(body), {
      status,
      headers: { "Content-Type": "application/json" },
    }),
  )
}

const apiError = (code: string) => ({ error: { code, message: code } })

describe("ConfirmEmail", () => {
  const fetchMock = vi.fn()

  beforeEach(() => {
    vi.stubGlobal("fetch", fetchMock)
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    fetchMock.mockReset()
  })

  it("confirms the link once and sends the person to the login", async () => {
    fetchMock.mockReturnValue(answer(204))
    render(<ConfirmEmail token="abc123" />)

    expect(await screen.findByText("E-mail confirmado")).toBeInTheDocument()
    expect(screen.getByText(/Entre com o e-mail e a senha que você escolheu/)).toBeInTheDocument()
    expect(screen.getByRole("link", { name: "Entrar" })).toHaveAttribute("href", "/login")
    expect(fetchMock).toHaveBeenCalledTimes(1)
    const [url, init] = fetchMock.mock.calls[0]
    expect(url).toBe("/api/auth/signup/confirm")
    expect(JSON.parse(init.body)).toEqual({ token: "abc123" })
  })

  it("shows that it is confirming while the API answers", () => {
    fetchMock.mockReturnValue(new Promise(() => {}))
    render(<ConfirmEmail token="abc123" />)

    expect(screen.getByText(/Confirmando seu e-mail/)).toBeInTheDocument()
  })

  it("explains an expired, used or replaced link and offers to send another", async () => {
    fetchMock.mockReturnValueOnce(answer(400, apiError("invalid_confirmation_token")))
    fetchMock.mockReturnValueOnce(answer(202))
    render(<ConfirmEmail token="old" />)

    expect(await screen.findByText("Este link não vale mais")).toBeInTheDocument()
    await userEvent.type(screen.getByLabelText("E-mail do cadastro"), "ana@example.com")
    await userEvent.click(screen.getByRole("button", { name: "Enviar novo link" }))

    expect(await screen.findByText(/Se houver um cadastro aguardando confirmação/)).toBeInTheDocument()
    const [url, init] = fetchMock.mock.calls[1]
    expect(url).toBe("/api/auth/signup/resend")
    expect(JSON.parse(init.body)).toEqual({ email: "ana@example.com" })
    expect(screen.getByRole("link", { name: "Fazer o cadastro de novo" })).toHaveAttribute(
      "href",
      "/signup",
    )
  })

  it("asks for a valid e-mail before sending another link", async () => {
    fetchMock.mockReturnValueOnce(answer(400, apiError("invalid_confirmation_token")))
    render(<ConfirmEmail token="old" />)
    await screen.findByText("Este link não vale mais")

    await userEvent.type(screen.getByLabelText("E-mail do cadastro"), "ana")
    await userEvent.click(screen.getByRole("button", { name: "Enviar novo link" }))

    expect(
      await screen.findByText("Digite um e-mail válido, como nome@dominio.com."),
    ).toBeInTheDocument()
    expect(fetchMock).toHaveBeenCalledTimes(1)
  })

  it("treats a link without a token like an invalid one, without calling the API", () => {
    render(<ConfirmEmail token="" />)

    expect(screen.getByText("Este link não vale mais")).toBeInTheDocument()
    expect(fetchMock).not.toHaveBeenCalled()
  })

  it("refuses with an explanation when the e-mail became someone else's account", async () => {
    fetchMock.mockReturnValue(answer(409, apiError("email_taken")))
    render(<ConfirmEmail token="abc123" />)

    expect(await screen.findByText("Este e-mail já tem uma conta")).toBeInTheDocument()
    expect(screen.getByRole("link", { name: "Entrar" })).toHaveAttribute("href", "/login")
    expect(screen.getByRole("link", { name: "Esqueci a senha" })).toHaveAttribute(
      "href",
      "/forgot-password",
    )
  })

  it("lets the person try again when the API cannot be reached", async () => {
    fetchMock.mockRejectedValueOnce(new TypeError("Failed to fetch"))
    fetchMock.mockReturnValueOnce(answer(204))
    render(<ConfirmEmail token="abc123" />)

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Não foi possível confirmar o e-mail. Verifique a conexão e tente de novo.",
    )
    await userEvent.click(screen.getByRole("button", { name: "Tentar de novo" }))

    expect(await screen.findByText("E-mail confirmado")).toBeInTheDocument()
  })
})
