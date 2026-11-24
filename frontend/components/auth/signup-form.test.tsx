import { render, screen } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { SignupForm } from "@/components/auth/signup-form"

const router = vi.hoisted(() => ({ replace: vi.fn(), refresh: vi.fn() }))
vi.mock("next/navigation", () => ({ useRouter: () => router }))

function answer(status: number, body?: unknown) {
  return Promise.resolve(
    new Response(body === undefined ? null : JSON.stringify(body), {
      status,
      headers: { "Content-Type": "application/json" },
    }),
  )
}

async function fillAndSubmit(
  { name = "Ana Souza", email = "ana@example.com", password = "Correct horse battery 7" } = {},
) {
  if (name) await userEvent.type(screen.getByLabelText("Nome"), name)
  await userEvent.type(screen.getByLabelText("E-mail"), email)
  await userEvent.type(screen.getByLabelText("Senha"), password)
  await userEvent.click(screen.getByRole("button", { name: "Criar conta" }))
}

describe("SignupForm", () => {
  const fetchMock = vi.fn()

  beforeEach(() => {
    vi.stubGlobal("fetch", fetchMock)
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    fetchMock.mockReset()
    router.replace.mockReset()
  })

  it("explains a missing name, an invalid e-mail and a short password without calling the API", async () => {
    render(<SignupForm />)

    await userEvent.type(screen.getByLabelText("E-mail"), "ana")
    await userEvent.type(screen.getByLabelText("Senha"), "short")
    await userEvent.click(screen.getByRole("button", { name: "Criar conta" }))

    expect(await screen.findByText("Digite seu nome.")).toBeInTheDocument()
    expect(screen.getByText("Digite um e-mail válido, como nome@dominio.com.")).toBeInTheDocument()
    expect(screen.getByText("A senha precisa ter pelo menos 10 caracteres.")).toBeInTheDocument()
    expect(fetchMock).not.toHaveBeenCalled()
  })

  it("asks for the sign-up with name, e-mail and password and shows the check-your-e-mail screen", async () => {
    fetchMock.mockReturnValue(answer(202))
    render(<SignupForm />)

    await fillAndSubmit()

    expect(await screen.findByText("Confira seu e-mail")).toBeInTheDocument()
    expect(screen.getByText(/Enviamos um e-mail de confirmação para ana@example.com/)).toBeInTheDocument()
    expect(screen.getByRole("button", { name: "Reenviar e-mail" })).toBeInTheDocument()
    expect(screen.getByRole("link", { name: "Voltar para entrar" })).toHaveAttribute(
      "href",
      "/login",
    )
    const [url, init] = fetchMock.mock.calls[0]
    expect(url).toBe("/api/auth/signup")
    expect(JSON.parse(init.body)).toEqual({
      email: "ana@example.com",
      name: "Ana Souza",
      password: "Correct horse battery 7",
    })
    expect(screen.queryByLabelText("Senha")).not.toBeInTheDocument()
  })

  it("does not go to the logged-in area: the account only exists after the e-mail is confirmed", async () => {
    fetchMock.mockReturnValue(answer(202))
    render(<SignupForm />)

    await fillAndSubmit()

    await screen.findByText("Confira seu e-mail")
    expect(router.replace).not.toHaveBeenCalled()
    expect(router.refresh).not.toHaveBeenCalled()
  })

  it("resends the e-mail from the check-your-e-mail screen and says nothing about the account", async () => {
    fetchMock.mockReturnValue(answer(202))
    render(<SignupForm />)
    await fillAndSubmit()
    await screen.findByText("Confira seu e-mail")

    await userEvent.click(screen.getByRole("button", { name: "Reenviar e-mail" }))

    expect(await screen.findByText(/Se houver um cadastro aguardando confirmação/)).toBeInTheDocument()
    const [url, init] = fetchMock.mock.calls[1]
    expect(url).toBe("/api/auth/signup/resend")
    expect(JSON.parse(init.body)).toEqual({ email: "ana@example.com" })
  })

  it("tells the person to wait when the resend limit was reached", async () => {
    fetchMock.mockReturnValueOnce(answer(202))
    fetchMock.mockReturnValueOnce(
      answer(429, { error: { code: "too_many_requests", message: "Too many attempts." } }),
    )
    render(<SignupForm />)
    await fillAndSubmit()
    await screen.findByText("Confira seu e-mail")

    await userEvent.click(screen.getByRole("button", { name: "Reenviar e-mail" }))

    expect(
      await screen.findByText("Muitas tentativas seguidas. Aguarde alguns minutos e tente de novo."),
    ).toBeInTheDocument()
  })

  it("tells the person to wait when too many sign-ups were asked for the same e-mail", async () => {
    fetchMock.mockReturnValue(
      answer(429, { error: { code: "too_many_requests", message: "Too many attempts." } }),
    )
    render(<SignupForm />)

    await fillAndSubmit()

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Muitas tentativas seguidas. Aguarde alguns minutos e tente de novo.",
    )
  })

  it("asks to check the connection when the API cannot be reached", async () => {
    fetchMock.mockRejectedValue(new TypeError("Failed to fetch"))
    render(<SignupForm />)

    await fillAndSubmit()

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Não foi possível criar a conta. Verifique a conexão e tente de novo.",
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
    "Evite sequências e repetições, como Abcdefghij1 ou Ab1Ab1Ab1Ab1. Escolha outra senha.",
  ],
  ["password_contains_email", "A senha não pode conter o seu e-mail. Escolha outra."],
  ["password_contains_name", "A senha não pode conter o seu nome. Escolha outra."],
  ["password_composition", "A senha precisa ter pelo menos 3 destes tipos de caractere: letras minúsculas, letras maiúsculas, números e símbolos."],
] as const

describe("SignupForm password rule", () => {
  const fetchMock = vi.fn()

  beforeEach(() => {
    vi.stubGlobal("fetch", fetchMock)
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    fetchMock.mockReset()
    router.replace.mockReset()
  })

  it("shows a short hint of the rule next to the password field, with no strength meter", () => {
    render(<SignupForm />)

    expect(screen.getByText("Use de 10 a 128 caracteres, com pelo menos 3 destes tipos: letras minúsculas, letras maiúsculas, números e símbolos. Evite senhas comuns, sequências e o seu e-mail ou nome.")).toBeInTheDocument()
    expect(screen.queryByRole("meter")).not.toBeInTheDocument()
    expect(screen.queryByRole("progressbar")).not.toBeInTheDocument()
  })

  it.each(refusals)("shows the reason for %s under the password field", async (code, message) => {
    fetchMock.mockReturnValue(
      answer(422, {
        error: { code: "validation_failed", message: "invalid", fields: { password: code } },
      }),
    )
    render(<SignupForm />)

    await fillAndSubmit({ password: "Senha1234567" })

    expect(await screen.findByText(message)).toBeInTheDocument()
    expect(screen.getByLabelText("Senha")).toHaveAttribute("aria-invalid", "true")
    expect(screen.queryByText("Confira seu e-mail")).not.toBeInTheDocument()
  })
})
