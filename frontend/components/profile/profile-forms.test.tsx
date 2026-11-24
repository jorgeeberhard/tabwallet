import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { render, screen } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { ProfileForms } from "@/components/profile/profile-forms"

vi.mock("@/components/user-menu", () => ({
  useCurrentUser: () => ({
    data: {
      id: "1",
      email: "ana@example.com",
      name: "Ana Souza",
      createdAt: "2026-01-15T00:00:00Z",
    },
  }),
}))
vi.mock("sonner", () => ({ toast: { success: vi.fn(), error: vi.fn() } }))

function answer(status: number, body?: unknown) {
  return Promise.resolve(
    new Response(body === undefined ? null : JSON.stringify(body), {
      status,
      headers: { "Content-Type": "application/json" },
    }),
  )
}

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

function renderForms() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  render(
    <QueryClientProvider client={client}>
      <ProfileForms />
    </QueryClientProvider>,
  )
}

describe("ProfileForms password rule", () => {
  const fetchMock = vi.fn()

  beforeEach(() => {
    vi.stubGlobal("fetch", fetchMock)
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    fetchMock.mockReset()
  })

  it("shows a short hint of the rule next to the new password field", () => {
    renderForms()

    expect(
      screen.getByText(
        "Use de 10 a 128 caracteres, com pelo menos 3 destes tipos: letras minúsculas, letras maiúsculas, números e símbolos. Evite senhas comuns, sequências e o seu e-mail ou nome.",
      ),
    ).toBeInTheDocument()
  })

  it.each(refusals)("shows the reason for %s under the new password field", async (code, message) => {
    fetchMock.mockReturnValue(
      answer(422, {
        error: { code: "validation_failed", message: "invalid", fields: { new_password: code } },
      }),
    )
    renderForms()

    await userEvent.type(document.getElementById("password-current-password")!, "correct horse battery")
    await userEvent.type(screen.getByLabelText("Nova senha"), "Senha1234567")
    await userEvent.type(screen.getByLabelText("Repita a nova senha"), "Senha1234567")
    await userEvent.click(screen.getByRole("button", { name: "Alterar senha" }))

    expect(await screen.findByText(message)).toBeInTheDocument()
    expect(screen.getByLabelText("Nova senha")).toHaveAttribute("aria-invalid", "true")
  })
})
