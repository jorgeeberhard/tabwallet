import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { render, screen, within } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { CategoriesScreen } from "@/components/categories/categories-screen"

vi.mock("sonner", () => ({ toast: { success: vi.fn(), error: vi.fn() } }))

interface Row {
  id: string
  name: string
  type: "income" | "expense"
  active: boolean
  is_charges?: boolean
}

function json(status: number, body?: unknown) {
  return Promise.resolve(
    new Response(body === undefined ? null : JSON.stringify(body), {
      status,
      headers: { "Content-Type": "application/json" },
    }),
  )
}

function renderScreen() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return render(
    <QueryClientProvider client={client}>
      <CategoriesScreen />
    </QueryClientProvider>,
  )
}

describe("CategoriesScreen", () => {
  const fetchMock = vi.fn()
  const rows: Row[] = [
    { id: "1", name: "Alimentação", type: "expense", active: true },
    { id: "2", name: "Juros e encargos", type: "expense", active: true, is_charges: true },
    { id: "3", name: "Lazer", type: "expense", active: false },
    { id: "4", name: "Salário", type: "income", active: true },
  ]

  beforeEach(() => {
    vi.stubGlobal("fetch", fetchMock)
    fetchMock.mockImplementation((url: string) =>
      url === "/api/categories" ? json(200, { categories: rows }) : json(404),
    )
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    fetchMock.mockReset()
  })

  it("lists the active and the deactivated categories of the chosen type", async () => {
    renderScreen()

    const active = await screen.findByRole("list", { name: "Categorias de despesas" })
    expect(within(active).getByText("Alimentação")).toBeInTheDocument()
    expect(within(active).queryByText("Lazer")).not.toBeInTheDocument()
    const inactive = screen.getByRole("list", { name: "Categorias desativadas" })
    expect(within(inactive).getByText("Lazer")).toBeInTheDocument()
    expect(screen.getByRole("button", { name: "Reativar Lazer" })).toBeInTheDocument()

    await userEvent.click(screen.getByRole("radio", { name: "Receitas" }))

    const incomes = screen.getByRole("list", { name: "Categorias de receitas" })
    expect(within(incomes).getByText("Salário")).toBeInTheDocument()
    expect(screen.queryByText("Alimentação")).not.toBeInTheDocument()
  })

  it("offers to reactivate when the new name matches a deactivated category", async () => {
    fetchMock.mockImplementation((url: string, init?: RequestInit) => {
      if (url === "/api/categories" && init?.method === "POST") {
        return json(409, { error: { code: "category_inactive_exists", category_id: "3" } })
      }
      if (url === "/api/categories/3/reactivate") {
        return json(200, { category: { ...rows[2], active: true } })
      }
      return json(200, { categories: rows })
    })
    renderScreen()
    await screen.findByRole("list", { name: "Categorias de despesas" })

    await userEvent.click(screen.getByRole("button", { name: "Nova categoria" }))
    await userEvent.type(screen.getByLabelText("Nome"), "lazer")
    await userEvent.click(screen.getByRole("button", { name: "Criar categoria" }))

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Já existe uma categoria desativada com este nome.",
    )
    await userEvent.click(screen.getByRole("button", { name: "Reativar categoria" }))

    expect(fetchMock).toHaveBeenCalledWith(
      "/api/categories/3/reactivate",
      expect.objectContaining({ method: "POST" }),
    )
  })

  it("asks for a name before calling the API", async () => {
    renderScreen()
    await screen.findByRole("list", { name: "Categorias de despesas" })

    await userEvent.click(screen.getByRole("button", { name: "Nova categoria" }))
    await userEvent.click(screen.getByRole("button", { name: "Criar categoria" }))

    expect(await screen.findByText("Digite o nome da categoria.")).toBeInTheDocument()
    expect(fetchMock).toHaveBeenCalledTimes(1) // only the list
  })
})
