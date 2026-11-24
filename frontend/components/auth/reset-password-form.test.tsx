import { render, screen } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { ResetPasswordForm } from "@/components/auth/reset-password-form"

function answer(status: number, body?: unknown) {
  return Promise.resolve(
    new Response(body === undefined ? null : JSON.stringify(body), {
      status,
      headers: { "Content-Type": "application/json" },
    }),
  )
}

async function fillAndSubmit(password: string, confirmation: string) {
  await userEvent.type(screen.getByLabelText("Nova senha"), password)
  await userEvent.type(screen.getByLabelText("Repita a nova senha"), confirmation)
  await userEvent.click(screen.getByRole("button", { name: "Redefinir senha" }))
}

describe("ResetPasswordForm", () => {
  const fetchMock = vi.fn()

  beforeEach(() => {
    vi.stubGlobal("fetch", fetchMock)
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    fetchMock.mockReset()
  })

  it("does not call the API when the two passwords differ", async () => {
    render(<ResetPasswordForm token="abc" />)

    await fillAndSubmit("a brand new passphrase", "another passphrase")

    expect(await screen.findByText("As senhas não são iguais.")).toBeInTheDocument()
    expect(fetchMock).not.toHaveBeenCalled()
  })

  it("sends the token with the new password and offers to sign in", async () => {
    fetchMock.mockReturnValue(answer(204))
    render(<ResetPasswordForm token="abc" />)

    await fillAndSubmit("a brand new passphrase", "a brand new passphrase")

    expect(await screen.findByText("Senha redefinida")).toBeInTheDocument()
    expect(screen.getByRole("link", { name: "Entrar" })).toHaveAttribute("href", "/login")
    const [url, init] = fetchMock.mock.calls[0]
    expect(url).toBe("/api/auth/password-reset/confirm")
    expect(JSON.parse(init.body)).toEqual({ token: "abc", password: "a brand new passphrase" })
  })

  it("explains an expired or used link and offers a new one", async () => {
    fetchMock.mockReturnValue(
      answer(400, { error: { code: "invalid_reset_token", message: "invalid" } }),
    )
    render(<ResetPasswordForm token="abc" />)

    await fillAndSubmit("a brand new passphrase", "a brand new passphrase")

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Este link de redefinição é inválido, expirou ou já foi usado. Peça um novo link.",
    )
    expect(screen.getByRole("link", { name: "Pedir novo link" })).toHaveAttribute(
      "href",
      "/forgot-password",
    )
  })
})

const refusals = [
  ["password_too_short", "A senha precisa ter pelo menos 10 caracteres."],
  ["password_too_long", "A senha pode ter no máximo 128 caracteres."],
  [
    "password_common",
    "Essa senha é muito comum e fácil de adivinhar. Escolha outra, de preferência uma frase.",
  ],
  [
    "password_sequence",
    "Evite sequências e repetições, como 1234567890 ou aaaaaaaaaa. Escolha outra senha.",
  ],
  ["password_contains_email", "A senha não pode conter o seu e-mail. Escolha outra."],
  ["password_contains_name", "A senha não pode conter o seu nome. Escolha outra."],
] as const

describe("ResetPasswordForm password rule", () => {
  const fetchMock = vi.fn()

  beforeEach(() => {
    vi.stubGlobal("fetch", fetchMock)
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    fetchMock.mockReset()
  })

  it("shows a short hint of the rule next to the new password field", () => {
    render(<ResetPasswordForm token="abc" />)

    expect(screen.getByText("Use de 10 a 128 caracteres; uma frase longa é uma boa senha. Evite senhas comuns, sequências e o seu e-mail ou nome.")).toBeInTheDocument()
  })

  it.each(refusals)("shows the reason for %s under the new password field", async (code, message) => {
    fetchMock.mockReturnValue(
      answer(422, {
        error: { code: "validation_failed", message: "invalid", fields: { password: code } },
      }),
    )
    render(<ResetPasswordForm token="abc" />)

    await fillAndSubmit("senha1234567", "senha1234567")

    expect(await screen.findByText(message)).toBeInTheDocument()
    expect(screen.getByLabelText("Nova senha")).toHaveAttribute("aria-invalid", "true")
    expect(screen.queryByText("Senha redefinida")).not.toBeInTheDocument()
  })
})
