import { render, screen, waitFor } from "@testing-library/react"
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

  it("explains an invalid e-mail and a short password without calling the API", async () => {
    render(<SignupForm />)

    await userEvent.type(screen.getByLabelText("E-mail"), "ana")
    await userEvent.type(screen.getByLabelText("Senha"), "short")
    await userEvent.click(screen.getByRole("button", { name: "Criar conta" }))

    expect(
      await screen.findByText("Digite um e-mail válido, como nome@dominio.com."),
    ).toBeInTheDocument()
    expect(screen.getByText("A senha precisa ter pelo menos 10 caracteres.")).toBeInTheDocument()
    expect(fetchMock).not.toHaveBeenCalled()
  })

  it("creates the account and goes to the home page", async () => {
    fetchMock.mockReturnValue(
      answer(201, { user: { id: "1", email: "ana@example.com", created_at: "2026-01-15T00:00:00Z" } }),
    )
    render(<SignupForm />)

    await userEvent.type(screen.getByLabelText("E-mail"), "ana@example.com")
    await userEvent.type(screen.getByLabelText("Senha"), "correct horse battery")
    await userEvent.click(screen.getByRole("button", { name: "Criar conta" }))

    await waitFor(() => expect(router.replace).toHaveBeenCalledWith("/"))
    expect(fetchMock.mock.calls[0][0]).toBe("/api/auth/signup")
  })

  it("shows on the e-mail field that the e-mail is already registered", async () => {
    fetchMock.mockReturnValue(
      answer(409, { error: { code: "email_taken", message: "An account with this e-mail already exists." } }),
    )
    render(<SignupForm />)

    await userEvent.type(screen.getByLabelText("E-mail"), "ana@example.com")
    await userEvent.type(screen.getByLabelText("Senha"), "correct horse battery")
    await userEvent.click(screen.getByRole("button", { name: "Criar conta" }))

    expect(
      await screen.findByText("Já existe uma conta com este e-mail. Entre ou use outro e-mail."),
    ).toBeInTheDocument()
    expect(screen.getByLabelText("E-mail")).toHaveAttribute("aria-invalid", "true")
    expect(router.replace).not.toHaveBeenCalled()
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

    expect(screen.getByText("Use de 10 a 128 caracteres; uma frase longa é uma boa senha. Evite senhas comuns, sequências e o seu e-mail ou nome.")).toBeInTheDocument()
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

    await userEvent.type(screen.getByLabelText("E-mail"), "ana@example.com")
    await userEvent.type(screen.getByLabelText("Senha"), "senha1234567")
    await userEvent.click(screen.getByRole("button", { name: "Criar conta" }))

    expect(await screen.findByText(message)).toBeInTheDocument()
    expect(screen.getByLabelText("Senha")).toHaveAttribute("aria-invalid", "true")
    expect(router.replace).not.toHaveBeenCalled()
  })
})
