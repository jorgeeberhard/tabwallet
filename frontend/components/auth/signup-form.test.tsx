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
    expect(screen.getByText("Use uma senha com pelo menos 8 caracteres.")).toBeInTheDocument()
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
