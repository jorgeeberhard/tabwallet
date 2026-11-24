import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { render, screen } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { toast } from "sonner"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { ProfileForms } from "@/components/profile/profile-forms"
import type { User } from "@/lib/api"

const state = vi.hoisted(() => ({ user: null as unknown }))

// The real query, so that what the form saves in the cache reaches the screen.
vi.mock("@/components/user-menu", async () => {
  const { useQuery } = await import("@tanstack/react-query")
  return {
    useCurrentUser: () => ({
      data: useQuery({
        queryKey: ["current-user"],
        queryFn: () => state.user as User,
        staleTime: Infinity,
      }).data,
    }),
  }
})
vi.mock("sonner", () => ({ toast: { success: vi.fn(), error: vi.fn() } }))

function answer(status: number, body?: unknown) {
  return Promise.resolve(
    new Response(body === undefined ? null : JSON.stringify(body), {
      status,
      headers: { "Content-Type": "application/json" },
    }),
  )
}

const apiError = (code: string, fields: Record<string, string> = {}) => ({
  error: { code, message: code, fields },
})

const ana: User = {
  id: "1",
  email: "ana@example.com",
  name: "Ana Souza",
  createdAt: "2026-01-15T00:00:00Z",
  pendingEmailChange: null,
}

const pending = { email: "ana.nova@example.com", expiresAt: "2026-01-16T12:00:00Z" }

function profileAnswer(withPending: boolean) {
  return {
    user: { id: "1", email: ana.email, name: ana.name, created_at: ana.createdAt },
    pending_email_change: withPending
      ? { email: pending.email, expires_at: pending.expiresAt }
      : null,
  }
}

function renderForms() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  render(
    <QueryClientProvider client={client}>
      <ProfileForms />
    </QueryClientProvider>,
  )
}

describe("ProfileForms e-mail change", () => {
  const fetchMock = vi.fn()

  beforeEach(() => {
    vi.stubGlobal("fetch", fetchMock)
    state.user = ana
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    fetchMock.mockReset()
    vi.mocked(toast.success).mockClear()
  })

  it("says that a new e-mail needs confirmation and the current one works until then", async () => {
    renderForms()

    expect(await screen.findByText(/enviamos um link de confirmação para o novo e-mail/)).toBeInTheDocument()
    expect(screen.getByText(/continua entrando com o atual até confirmar/)).toBeInTheDocument()
    expect(screen.queryByText("Troca de e-mail pendente")).not.toBeInTheDocument()
    expect(screen.getByRole("button", { name: "Trocar e-mail" })).toBeInTheDocument()
  })

  it("asks for the change with the current password and then shows it as pending", async () => {
    fetchMock.mockReturnValue(answer(200, profileAnswer(true)))
    renderForms()

    const email = await screen.findByLabelText("E-mail")
    await userEvent.clear(email)
    await userEvent.type(email, "ana.nova@example.com")
    await userEvent.type(document.getElementById("email-current-password")!, "correct horse battery")
    await userEvent.click(screen.getByRole("button", { name: "Trocar e-mail" }))

    expect(await screen.findByText("Troca de e-mail pendente")).toBeInTheDocument()
    const [url, init] = fetchMock.mock.calls[0]
    expect(url).toBe("/api/profile")
    expect(init.method).toBe("PATCH")
    expect(JSON.parse(init.body)).toEqual({
      email: "ana.nova@example.com",
      current_password: "correct horse battery",
    })
    expect(toast.success).toHaveBeenCalledWith(
      "Enviamos um link de confirmação para ana.nova@example.com.",
    )
    // The pending address and the one that still works are both on screen,
    // and the form goes back to the e-mail that works.
    expect(screen.getAllByText("ana.nova@example.com").length).toBeGreaterThan(0)
    expect(screen.getByLabelText("E-mail")).toHaveValue("ana@example.com")
    expect(document.getElementById("email-current-password")).toHaveValue("")
  })

  it("shows a refused current password under the password field and no pending change", async () => {
    fetchMock.mockReturnValue(answer(403, apiError("invalid_current_password")))
    renderForms()

    const email = await screen.findByLabelText("E-mail")
    await userEvent.clear(email)
    await userEvent.type(email, "ana.nova@example.com")
    await userEvent.type(document.getElementById("email-current-password")!, "wrong password!!")
    await userEvent.click(screen.getByRole("button", { name: "Trocar e-mail" }))

    expect(
      await screen.findByText("A senha atual não confere. Confira e tente de novo."),
    ).toBeInTheDocument()
    expect(screen.queryByText("Troca de e-mail pendente")).not.toBeInTheDocument()
  })

  it("does not ask the API to change the e-mail to the one already in use", async () => {
    renderForms()

    const email = await screen.findByLabelText("E-mail")
    await userEvent.clear(email)
    await userEvent.type(email, "ANA@example.com")
    await userEvent.type(document.getElementById("email-current-password")!, "correct horse battery")
    await userEvent.click(screen.getByRole("button", { name: "Trocar e-mail" }))

    expect(
      await screen.findByText("Este já é o seu e-mail de acesso. Digite o novo e-mail."),
    ).toBeInTheDocument()
    expect(fetchMock).not.toHaveBeenCalled()
  })

  describe("with a pending change", () => {
    beforeEach(() => {
      state.user = { ...ana, pendingEmailChange: pending }
    })

    it("shows the new address, which e-mail still works and how the link behaves", async () => {
      renderForms()

      expect(await screen.findByText("Troca de e-mail pendente")).toBeInTheDocument()
      expect(screen.getByText("ana.nova@example.com")).toBeInTheDocument()
      expect(screen.getByText(/vale por 24 horas e só pode ser usado uma vez/)).toBeInTheDocument()
      expect(screen.getByText("ana@example.com")).toBeInTheDocument()
      expect(screen.getByText(/Até lá, você continua entrando com/)).toBeInTheDocument()
      expect(screen.getByText(/substitui esta e invalida o link anterior/)).toBeInTheDocument()
      expect(screen.getByRole("button", { name: "Reenviar link" })).toBeInTheDocument()
    })

    it("sends the link again to the pending address", async () => {
      fetchMock.mockReturnValue(answer(202))
      renderForms()

      await userEvent.click(await screen.findByRole("button", { name: "Reenviar link" }))

      expect(
        await screen.findByText(
          "Enviamos um novo link para ana.nova@example.com. O link anterior deixou de valer.",
        ),
      ).toBeInTheDocument()
      const [url, init] = fetchMock.mock.calls[0]
      expect(url).toBe("/api/profile/email-change/resend")
      expect(init.method).toBe("POST")
    })

    it("tells the person to wait when the resend limit is reached", async () => {
      fetchMock.mockReturnValue(answer(429, apiError("too_many_requests")))
      renderForms()

      await userEvent.click(await screen.findByRole("button", { name: "Reenviar link" }))

      expect(
        await screen.findByText("Muitas tentativas seguidas. Aguarde alguns minutos e tente de novo."),
      ).toBeInTheDocument()
    })

    it("says so when the change is no longer pending", async () => {
      fetchMock.mockReturnValue(answer(404, apiError("no_pending_email_change")))
      renderForms()

      await userEvent.click(await screen.findByRole("button", { name: "Reenviar link" }))

      expect(await screen.findByText(/Não há troca de e-mail pendente/)).toBeInTheDocument()
    })
  })
})
