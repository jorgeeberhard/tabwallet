import { render, screen } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { BackButton } from "@/components/back-button"

const router = { back: vi.fn(), push: vi.fn() }
let pathname = "/transacoes"

vi.mock("next/navigation", () => ({
  usePathname: () => pathname,
  useRouter: () => router,
}))

describe("BackButton", () => {
  beforeEach(() => {
    router.back.mockClear()
    router.push.mockClear()
    pathname = "/transacoes"
  })

  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it("goes to the previous page when the site has history", async () => {
    vi.stubGlobal("navigation", { canGoBack: true })
    render(<BackButton />)

    await userEvent.click(screen.getByRole("button", { name: "Voltar" }))

    expect(router.back).toHaveBeenCalledTimes(1)
    expect(router.push).not.toHaveBeenCalled()
  })

  it("goes to the home page when the site has no earlier entry", async () => {
    vi.stubGlobal("navigation", { canGoBack: false })
    render(<BackButton />)

    await userEvent.click(screen.getByRole("button", { name: "Voltar" }))

    expect(router.push).toHaveBeenCalledWith("/")
    expect(router.back).not.toHaveBeenCalled()
  })

  it("goes to the home page when the page was opened directly (no Navigation API)", async () => {
    render(<BackButton />)

    await userEvent.click(screen.getByRole("button", { name: "Voltar" }))

    expect(router.push).toHaveBeenCalledWith("/")
    expect(router.back).not.toHaveBeenCalled()
  })

  it("goes back once the visitor moved between pages (no Navigation API)", async () => {
    const { rerender } = render(<BackButton />)
    pathname = "/cartoes"
    rerender(<BackButton />)

    await userEvent.click(screen.getByRole("button", { name: "Voltar" }))

    expect(router.back).toHaveBeenCalledTimes(1)
  })

  it("is not shown on the home page", () => {
    pathname = "/"
    render(<BackButton />)

    expect(screen.queryByRole("button", { name: "Voltar" })).not.toBeInTheDocument()
  })
})
