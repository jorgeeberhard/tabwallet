import { render, screen, waitFor } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { LoginForm } from "@/components/auth/login-form"

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

describe("LoginForm", () => {
  const fetchMock = vi.fn()

  beforeEach(() => {
    vi.stubGlobal("fetch", fetchMock)
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    fetchMock.mockReset()
    router.replace.mockReset()
  })

  it("explains what is missing and does not call the API", async () => {
    render(<LoginForm />)

    await userEvent.click(screen.getByRole("button", { name: "Entrar" }))

    expect(await screen.findByText("Digite seu e-mail.")).toBeInTheDocument()
    expect(screen.getByText("Digite sua senha.")).toBeInTheDocument()
    expect(fetchMock).not.toHaveBeenCalled()
  })

  it("signs in and goes to the home page", async () => {
    fetchMock.mockReturnValue(
      answer(200, { user: { id: "1", email: "ana@example.com", created_at: "2026-01-15T00:00:00Z" } }),
    )
    render(<LoginForm />)

    await userEvent.type(screen.getByLabelText("E-mail"), "ana@example.com")
    await userEvent.type(screen.getByLabelText("Senha"), "correct horse battery")
    await userEvent.click(screen.getByRole("button", { name: "Entrar" }))

    await waitFor(() => expect(router.replace).toHaveBeenCalledWith("/"))
    const [url, init] = fetchMock.mock.calls[0]
    expect(url).toBe("/api/auth/login")
    expect(JSON.parse(init.body)).toEqual({
      email: "ana@example.com",
      password: "correct horse battery",
    })
  })

  it("shows a clear message when the e-mail or password is wrong", async () => {
    fetchMock.mockReturnValue(
      answer(401, { error: { code: "invalid_credentials", message: "Wrong e-mail or password." } }),
    )
    render(<LoginForm />)

    await userEvent.type(screen.getByLabelText("E-mail"), "ana@example.com")
    await userEvent.type(screen.getByLabelText("Senha"), "wrong password")
    await userEvent.click(screen.getByRole("button", { name: "Entrar" }))

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "E-mail ou senha incorretos. Confira os dados e tente de novo.",
    )
    expect(router.replace).not.toHaveBeenCalled()
  })

  it("asks to check the connection when the API cannot be reached", async () => {
    fetchMock.mockRejectedValue(new TypeError("Failed to fetch"))
    render(<LoginForm />)

    await userEvent.type(screen.getByLabelText("E-mail"), "ana@example.com")
    await userEvent.type(screen.getByLabelText("Senha"), "whatever")
    await userEvent.click(screen.getByRole("button", { name: "Entrar" }))

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Não foi possível entrar. Verifique a conexão e tente de novo.",
    )
  })
})
